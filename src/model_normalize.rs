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
//! * Rigid-group detection keeps its exact pivot/part-delta path as stronger
//!   evidence for one-part models and tied descendant clusters. Descendant
//!   consensus is its fallback when the pivot is independent.
//!
//! This canonicalization is deliberately enabled by the CLI only for
//! `.rbxm`/`.rbxmx`. In a `.rbxl`/`.rbxlx`, absolute world placement is authored
//! place content and must never be normalized away.

use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::{CFrame, Variant};

use crate::hash::LazyHashCache;
use crate::match_instances::match_children;
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
    matches: &mut Vec<(Ref, Ref)>,
) {
    let result = match_children(
        base,
        side,
        base_parent,
        side_parent,
        base_hashes,
        side_hashes,
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

fn dominant_delta(base: &WeakDom, side: &WeakDom) -> Option<(Rigid, usize, usize)> {
    let base_hashes = LazyHashCache::new(base);
    let side_hashes = LazyHashCache::new(side);
    let mut matches = Vec::new();
    collect_matches(
        base,
        side,
        base.root_ref(),
        side.root_ref(),
        &base_hashes,
        &side_hashes,
        &mut matches,
    );

    struct Cluster {
        delta: Rigid,
        count: usize,
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
            cluster.count += 1;
        } else {
            clusters.push(Cluster { delta, count: 1 });
        }
    }

    clusters.sort_unstable_by_key(|cluster| std::cmp::Reverse(cluster.count));
    let best = clusters.first()?;
    let tied = clusters
        .get(1)
        .is_some_and(|runner_up| runner_up.count == best.count);
    if tied || best.count < MIN_SUPPORT || best.count * 2 <= matched_parts {
        return None;
    }
    Some((best.delta, matched_parts, best.count))
}

fn transform_world_properties(dom: &mut WeakDom, alignment: Rigid) {
    let refs: Vec<Ref> = dom
        .descendants()
        .map(|instance| instance.referent())
        .collect();

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

/// Express `side` in the dominant BasePart frame of `base`.
///
/// Returns `None` when there is not a unique majority transform, leaving the
/// DOM untouched. Call this only for model asset files (`.rbxm`/`.rbxmx`),
/// never place files where absolute world placement is authored content.
pub fn normalize_model_dom_to_base(
    base: &WeakDom,
    side: &mut WeakDom,
) -> Option<ModelNormalization> {
    let (side_delta, matched_parts, supporting_parts) = dominant_delta(base, side)?;
    transform_world_properties(side, side_delta.inverse());
    Some(ModelNormalization {
        matched_parts,
        supporting_parts,
        side_delta: side_delta.to_cframe(),
    })
}
