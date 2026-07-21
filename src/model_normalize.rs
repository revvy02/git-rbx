//! Canonicalize the world reference frame of model assets before diffing.
//!
//! Why this is necessary:
//!
//! * A `.rbxm` stores `BasePart.CFrame` in world space even though the asset
//!   itself has no stable world placement. Saving otherwise-identical copies
//!   at different placements can therefore make nearly every part look edited.
//! * `Model.WorldPivotData` cannot be used as that missing reference frame.
//!   Studio permits assigning `Model.WorldPivot` without moving any descendant,
//!   so the pivot-to-part transforms may differ while all part-to-part geometry
//!   is still identical. The tow-truck fixture exhibits exactly this case.
//! * For every matched BasePart we instead calculate `side * base^-1`. A unique
//!   strict majority of equal deltas identifies the asset-wide rigid transform.
//!   Applying its inverse expresses the entire side DOM in the base content
//!   frame. A part with a real local edit retains the residual difference.
//!   Using one arbitrary part as an anchor would be unsafe because that part
//!   may itself be the locally edited outlier. A majority makes ordinary local
//!   edits outliers instead of allowing one of them to redefine the asset's
//!   frame. Requiring more than half of all matched parts, with no tie, is the
//!   corresponding safety boundary: when the geometry does not establish one
//!   dominant frame, we decline normalization rather than guess.
//! * The alignment is applied only to world-space properties:
//!   `BasePart.CFrame` and `Model.WorldPivotData`. Local properties such as
//!   `Attachment.CFrame` and `BasePart.PivotOffset` must remain untouched.
//!   A pivot that moved with its content consequently normalizes to the base;
//!   an independently edited pivot remains different relative to the content
//!   and is still reported.
//! * After the preliminary matching pass establishes the frame, we transform
//!   the whole side DOM before the real diff/merge pass. This matters beyond
//!   comparison: hashes and duplicate-instance matching see canonical CFrames,
//!   and newly added subtrees are copied into the base frame instead of keeping
//!   the side file's arbitrary world offset.
//! * Canonicalization is not permission to discard placement. The extracted
//!   deltas are merged like any other three-way value: an unchanged side loses
//!   to a changed side, equal changes deduplicate, and two different changes
//!   become a `ModelFrame` conflict. The selected delta is applied only after
//!   canonical content/property resolution, so it carries the complete merged
//!   tree—including additions—into the chosen world frame.
//! * A root pivot edit remains an independent property conflict. Applying a
//!   chosen normalized pivot first and a chosen frame second still reconstructs
//!   a raw branch exactly when both choices come from that branch, while also
//!   permitting a user to compose one side's placement with the other's pivot.
//! * Rigid-group detection keeps its exact pivot/part-delta path as stronger
//!   evidence for one-part models and tied descendant clusters. Descendant
//!   consensus is its fallback when the pivot is independent.
//!
//! This canonicalization is deliberately enabled by the CLI only for
//! `.rbxm`/`.rbxmx`. In a `.rbxl`/`.rbxlx`, absolute world placement is authored
//! place content and must never be normalized away.

use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::{CFrame, Variant};
use std::collections::HashSet;

use crate::hash::{DeepHashCache, LazyHashCache};
use crate::match_instances::{get_instance_path, match_children};
use crate::rigid_groups::Rigid;

const MIN_SUPPORT: usize = 2;

#[derive(Debug, Clone)]
pub struct ModelNormalization {
    /// Matched BaseParts with a readable CFrame.
    pub matched_parts: usize,
    /// Matched BaseParts supporting the selected dominant transform.
    pub supporting_parts: usize,
    /// World transform taking base content into the side's content frame.
    pub side_delta: CFrame,
    /// Serialized top-level base-DOM root containing the supporting parts.
    pub base_target_ref: Ref,
}

#[derive(Debug, Clone)]
pub enum ModelFrameDecision {
    /// Normal three-way semantics selected this frame without a conflict.
    Automatic(CFrame),
    /// Both branches changed the asset frame differently.
    Conflict,
}

#[derive(Debug, Clone)]
pub struct ModelFrameMerge {
    /// Model whose content establishes the asset frame and receives the
    /// eventual frame choice.
    pub target_ref: Ref,
    pub path: String,
    pub ours: ModelNormalization,
    pub theirs: ModelNormalization,
    pub decision: ModelFrameDecision,
}

fn class_is_a(class_name: &str, ancestor: &str) -> bool {
    let Ok(database) = rbx_reflection_database::get() else {
        return false;
    };
    let mut current = class_name;
    loop {
        if current == ancestor {
            return true;
        }
        let Some(class) = database.classes.get(current) else {
            return false;
        };
        let Some(parent) = class.superclass.as_ref() else {
            return false;
        };
        current = parent;
    }
}

fn collect_matches(
    base: &WeakDom,
    side: &WeakDom,
    base_parent: Ref,
    side_parent: Ref,
    base_hashes: &LazyHashCache,
    side_hashes: &LazyHashCache,
    base_deep: &DeepHashCache,
    side_deep: &DeepHashCache,
    matches: &mut Vec<(Ref, Ref)>,
) {
    let result = match_children(
        base,
        side,
        base_parent,
        side_parent,
        base_hashes,
        side_hashes,
        base_deep,
        side_deep,
    );
    for (base_ref, side_ref) in result.matched {
        matches.push((base_ref, side_ref));
        collect_matches(
            base,
            side,
            base_ref,
            side_ref,
            base_hashes,
            side_hashes,
            base_deep,
            side_deep,
            matches,
        );
    }
}

fn cframe_property(dom: &WeakDom, referent: Ref) -> Option<CFrame> {
    let instance = dom.get_by_ref(referent)?;
    match instance.properties.get(&"CFrame".into()) {
        Some(Variant::CFrame(cframe)) => Some(*cframe),
        _ => None,
    }
}

fn ancestors(dom: &WeakDom, mut referent: Ref) -> Vec<Ref> {
    let mut result = vec![referent];
    while let Some(instance) = dom.get_by_ref(referent) {
        let parent = instance.parent();
        if parent.is_none() {
            break;
        }
        result.push(parent);
        referent = parent;
    }
    result
}

fn lowest_common_ancestor(dom: &WeakDom, referents: &[Ref]) -> Ref {
    let mut result = referents[0];
    for &other in &referents[1..] {
        let result_ancestors = ancestors(dom, result);
        let mut candidate = other;
        loop {
            if result_ancestors.contains(&candidate) {
                result = candidate;
                break;
            }
            let Some(parent) = dom.get_by_ref(candidate).map(|instance| instance.parent()) else {
                break;
            };
            if parent.is_none() {
                break;
            }
            candidate = parent;
        }
    }
    result
}

fn serialized_asset_root(dom: &WeakDom, mut referent: Ref) -> Ref {
    let root = dom.root_ref();
    while let Some(instance) = dom.get_by_ref(referent) {
        let parent = instance.parent();
        if parent == root || parent.is_none() {
            break;
        }
        referent = parent;
    }
    referent
}

fn dominant_delta(base: &WeakDom, side: &WeakDom) -> Option<(Rigid, usize, Vec<Ref>)> {
    let base_hashes = LazyHashCache::new(base);
    let side_hashes = LazyHashCache::new(side);
    let ignored = HashSet::new();
    let base_deep = DeepHashCache::new(base, &ignored);
    let side_deep = DeepHashCache::new(side, &ignored);
    let mut matches = Vec::new();
    collect_matches(
        base,
        side,
        base.root_ref(),
        side.root_ref(),
        &base_hashes,
        &side_hashes,
        &base_deep,
        &side_deep,
        &mut matches,
    );

    struct Cluster {
        delta: Rigid,
        base_parts: Vec<Ref>,
    }
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut matched_parts = 0;

    for (base_ref, side_ref) in matches {
        let Some(base_instance) = base.get_by_ref(base_ref) else {
            continue;
        };
        let Some(side_instance) = side.get_by_ref(side_ref) else {
            continue;
        };
        if !class_is_a(base_instance.class.as_str(), "BasePart")
            || !class_is_a(side_instance.class.as_str(), "BasePart")
        {
            continue;
        }
        let (Some(base_cframe), Some(side_cframe)) = (
            cframe_property(base, base_ref),
            cframe_property(side, side_ref),
        ) else {
            continue;
        };

        matched_parts += 1;
        let delta = Rigid::delta(
            Rigid::from_cframe(&side_cframe),
            Rigid::from_cframe(&base_cframe),
        );
        if let Some(cluster) = clusters
            .iter_mut()
            .find(|cluster| Rigid::close(&cluster.delta, &delta))
        {
            cluster.base_parts.push(base_ref);
        } else {
            clusters.push(Cluster {
                delta,
                base_parts: vec![base_ref],
            });
        }
    }

    clusters.sort_unstable_by_key(|cluster| std::cmp::Reverse(cluster.base_parts.len()));
    let best = clusters.first()?;
    let tied = clusters
        .get(1)
        .is_some_and(|runner_up| runner_up.base_parts.len() == best.base_parts.len());
    if tied || best.base_parts.len() < MIN_SUPPORT || best.base_parts.len() * 2 <= matched_parts {
        return None;
    }
    Some((best.delta, matched_parts, best.base_parts.clone()))
}

fn transform_world_refs(dom: &mut WeakDom, refs: Vec<Ref>, alignment: Rigid) {
    for referent in refs {
        let Some(instance) = dom.get_by_ref_mut(referent) else {
            continue;
        };

        if class_is_a(instance.class.as_str(), "BasePart") {
            if let Some(Variant::CFrame(cframe)) = instance.properties.get(&"CFrame".into()) {
                let normalized = alignment.mul(Rigid::from_cframe(cframe)).to_cframe();
                instance
                    .properties
                    .insert("CFrame".into(), Variant::CFrame(normalized));
            }
        }

        if class_is_a(instance.class.as_str(), "Model")
            && !class_is_a(instance.class.as_str(), "WorldRoot")
        {
            if let Some(Variant::OptionalCFrame(Some(cframe))) =
                instance.properties.get(&"WorldPivotData".into())
            {
                let normalized = alignment.mul(Rigid::from_cframe(cframe)).to_cframe();
                instance.properties.insert(
                    "WorldPivotData".into(),
                    Variant::OptionalCFrame(Some(normalized)),
                );
            }
        }
    }
}

fn transform_world_properties(dom: &mut WeakDom, alignment: Rigid) {
    let refs = dom
        .descendants()
        .map(|instance| instance.referent())
        .collect();
    transform_world_refs(dom, refs, alignment);
}

fn transform_world_properties_below(dom: &mut WeakDom, root: Ref, alignment: Rigid) {
    let mut pending = vec![root];
    let mut refs = Vec::new();
    while let Some(referent) = pending.pop() {
        let Some(instance) = dom.get_by_ref(referent) else {
            continue;
        };
        pending.extend(instance.children().iter().copied());
        refs.push(referent);
    }

    transform_world_refs(dom, refs, alignment);
}

/// Apply an asset frame to every world-space property in a DOM. This is used
/// before merging when ordinary three-way semantics select one frame.
pub fn apply_model_frame_to_dom(dom: &mut WeakDom, frame: &CFrame) {
    transform_world_properties(dom, Rigid::from_cframe(frame));
}

/// Apply a selected asset frame to one model subtree after its canonical
/// conflicts have been resolved.
pub fn apply_model_frame(dom: &mut WeakDom, target: Ref, frame: &CFrame) {
    transform_world_properties_below(dom, target, Rigid::from_cframe(frame));
}

pub fn model_frames_close(a: &CFrame, b: &CFrame) -> bool {
    Rigid::close(&Rigid::from_cframe(a), &Rigid::from_cframe(b))
}

fn identity_frame() -> CFrame {
    Rigid::identity().to_cframe()
}

/// Canonicalize both branches together and retain the discarded placement as
/// a first-class three-way frame decision. If either branch lacks a safe
/// descendant consensus, neither branch is changed.
pub fn normalize_model_merge_frames(
    base: &WeakDom,
    ours: &mut WeakDom,
    theirs: &mut WeakDom,
) -> Option<ModelFrameMerge> {
    let ours_normalization = detect_model_normalization(base, ours)?;
    let theirs_normalization = detect_model_normalization(base, theirs)?;
    let target_ref = lowest_common_ancestor(
        base,
        &[
            ours_normalization.base_target_ref,
            theirs_normalization.base_target_ref,
        ],
    );

    // The synthetic WeakDom root is not serialized and cannot be the target
    // of an in-file conflict. Decline the joint normalization for a multi-root
    // asset instead of producing a frame choice the resolver cannot apply.
    if target_ref == base.root_ref() {
        return None;
    }

    transform_world_properties(
        ours,
        Rigid::from_cframe(&ours_normalization.side_delta).inverse(),
    );
    transform_world_properties(
        theirs,
        Rigid::from_cframe(&theirs_normalization.side_delta).inverse(),
    );

    let identity = identity_frame();
    let decision = if model_frames_close(
        &ours_normalization.side_delta,
        &theirs_normalization.side_delta,
    ) {
        ModelFrameDecision::Automatic(ours_normalization.side_delta)
    } else if model_frames_close(&ours_normalization.side_delta, &identity) {
        ModelFrameDecision::Automatic(theirs_normalization.side_delta)
    } else if model_frames_close(&theirs_normalization.side_delta, &identity) {
        ModelFrameDecision::Automatic(ours_normalization.side_delta)
    } else {
        ModelFrameDecision::Conflict
    };

    Some(ModelFrameMerge {
        target_ref,
        path: get_instance_path(base, target_ref),
        ours: ours_normalization,
        theirs: theirs_normalization,
        decision,
    })
}

fn detect_model_normalization(base: &WeakDom, side: &WeakDom) -> Option<ModelNormalization> {
    let (side_delta, matched_parts, supporting_parts) = dominant_delta(base, side)?;
    let base_target_ref =
        serialized_asset_root(base, lowest_common_ancestor(base, &supporting_parts));
    Some(ModelNormalization {
        matched_parts,
        supporting_parts: supporting_parts.len(),
        side_delta: side_delta.to_cframe(),
        base_target_ref,
    })
}

/// Express `side` in the dominant BasePart frame of `base`.
///
/// Returns `None` when there is not a unique majority transform, leaving the
/// DOM untouched. Call this only for model asset files (`.rbxm`/`.rbxmx`),
/// never place files where absolute world placement is authored content.
pub fn normalize_model_dom_to_base(
    base: &WeakDom,
    side: &mut WeakDom,
) -> Option<ModelNormalization> {
    let normalization = detect_model_normalization(base, side)?;
    transform_world_properties(
        side,
        Rigid::from_cframe(&normalization.side_delta).inverse(),
    );
    Some(normalization)
}
