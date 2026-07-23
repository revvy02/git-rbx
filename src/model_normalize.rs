//! Factor model-asset placement into hierarchical frame decisions.
//!
//! Roblox serializes `BasePart.CFrame` and `Model.WorldPivotData` in world
//! space. A model asset therefore has no explicit distinction between moving
//! the asset (or a nested Model) and editing all of the affected world-space
//! values. Treating the whole file as one flat voting population works for a
//! simple asset, but breaks down for nested Models: a large child can outvote
//! its parent, and independent parent/child moves can be mistaken for two
//! incompatible choices for one global frame.
//!
//! This module recovers the missing hierarchy instead:
//!
//! * Every serialized top-level instance is a frame boundary. Every nested
//!   `Model` is another boundary. A non-Model top-level instance is a useful
//!   pseudo-boundary for loose model assets.
//! * Detection runs bottom-up. Directly-owned matched BaseParts each provide
//!   one world-delta vote, while each immediate child boundary provides at
//!   most one vote regardless of how many parts it contains. This prevents a
//!   deep model's part count from redefining every ancestor's frame.
//! * A unique strict majority of at least two content units establishes a
//!   boundary frame. The boundary's own `WorldPivotData` may break a tie
//!   between otherwise viable content clusters, but it is never counted as a
//!   content unit: Studio permits a pivot edit without moving descendants, and
//!   a one-part Model is indistinguishable from a part-plus-pivot edit. This
//!   also prevents an ambiguously matched one-part duplicate from inventing a
//!   large local frame.
//! * Detected absolute frames are inherited top-down through boundaries that
//!   have no independent evidence. Each boundary's authored local frame is
//!   then `absolute * parent_absolute^-1`. This is the crucial factorization:
//!   an outer move and an inner move become two composable decisions instead
//!   of two competing global guesses.
//! * Canonicalization applies the inverse effective absolute frame exactly
//!   once to every world-space property, using its nearest boundary. Added
//!   descendants inherit their matched ancestor's frame. Local properties
//!   such as `Attachment.CFrame` and `BasePart.PivotOffset` are untouched.
//! * Root placement is removed first, then matching is rebuilt before nested
//!   canonicalization. That post-root identity map is pinned for the merge.
//!   Otherwise nested canonicalization can make duplicate siblings newly
//!   identical and accidentally reshuffle which instance is which.
//! * The merge keeps an ordered, top-down plan of local frame applications.
//!   If all frame decisions are automatic, the plan can be applied to every
//!   canonical branch before conflict data is stamped. If any frame decision
//!   conflicts, automatic and selected frames must all be applied together,
//!   top-down, after ordinary conflict resolution. That ordering is required
//!   for rotations because rigid transforms do not commute.
//!
//! Two-way model-asset diffs use the same hierarchy. Each non-identity local
//! frame becomes one explicit `model_frame` diff entry, while canonicalized
//! descendants contribute only their residual authored property edits. The
//! CLI also uses this for place files: world placement remains authored and
//! visible as the explicit frame entry instead of disappearing into thousands
//! of descendant CFrame edits.

use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::{CFrame, Variant};
use std::collections::{HashMap, HashSet};

use crate::diff::DiffConfig;
use crate::diff_dom::{DescendantRefs, DiffDom, DomView, DomViewMut};
use crate::dom_utils::class_is_a;
use crate::edit_script::{
    compute_instance_identity, compute_semantic_changes_with_identity, EditScript, InstanceIdentity,
};
use crate::hash::{DeepHashCache, LazyHashCache};
use crate::match_instances::{get_instance_path, Matcher};
use crate::rigid_groups::Rigid;
use crate::value_compare::cframes_equal;

const MIN_CONTENT_SUPPORT: usize = 2;

#[derive(Debug, Clone)]
pub struct ModelNormalization {
    /// Matched BaseParts below the selected serialized root boundary.
    pub matched_parts: usize,
    /// Matched BaseParts that ultimately support the selected root transform.
    pub supporting_parts: usize,
    /// World transform taking base content into the side's content frame.
    pub side_delta: CFrame,
    /// Serialized top-level base-DOM root receiving the normalization.
    pub base_target_ref: Ref,
}

/// One inferred rigid movement reported by a two-way model-asset diff.
#[derive(Debug, Clone)]
pub struct ModelFrameChange {
    pub base_ref: Ref,
    pub side_ref: Ref,
    pub path: String,
    /// Stable top-down order. Ancestors always sort first.
    pub order: usize,
    /// Nearest ancestor that also has a reported frame change.
    pub parent_order: Option<usize>,
    /// Local transform relative to `parent_order` (or world for a root).
    pub delta: CFrame,
}

/// Canonicalization state for an ordinary two-way model-asset diff.
#[derive(Debug, Clone)]
pub struct ModelFrameDiff {
    pub frames: Vec<ModelFrameChange>,
    /// Number of boundaries that independently established a frame.
    pub detected: usize,
    /// Identity captured after root alignment and before nested
    /// canonicalization. Diffing must keep this mapping pinned.
    pub identity: InstanceIdentity,
}

#[derive(Debug, Clone)]
pub enum ModelFrameDecision {
    /// Normal three-way semantics selected this local frame without conflict.
    Automatic(CFrame),
    /// Both branches changed this boundary's local frame differently.
    Conflict,
}

/// One independently mergeable Model boundary in the hierarchical frame plan.
#[derive(Debug, Clone)]
pub struct ModelFrame {
    pub target_ref: Ref,
    pub ours_ref: Option<Ref>,
    pub theirs_ref: Option<Ref>,
    pub path: String,
    /// Stable top-down application order. Ancestors always sort first.
    pub order: usize,
    /// Nearest ancestor that also has a frame decision.
    pub parent_order: Option<usize>,
    /// Local frame taking the parent boundary's effective frame to this side.
    pub ours: CFrame,
    pub theirs: CFrame,
    pub decision: ModelFrameDecision,
}

/// An automatic local frame persisted when another boundary remains conflicted.
#[derive(Debug, Clone)]
pub struct ModelFrameApplication {
    pub target_ref: Ref,
    pub path: String,
    pub order: usize,
    pub parent_order: Option<usize>,
    pub delta: CFrame,
}

#[derive(Debug, Clone)]
pub struct ModelFrameMerge {
    /// Non-identity local frame decisions, in top-down order.
    pub frames: Vec<ModelFrame>,
    /// Number of boundaries that established their own frame on each side.
    pub ours_detected: usize,
    pub theirs_detected: usize,
    /// Instance identity established before spatial canonicalization.
    pub ours_identity: InstanceIdentity,
    pub theirs_identity: InstanceIdentity,
}

impl ModelFrameMerge {
    pub fn has_conflicts(&self) -> bool {
        self.frames
            .iter()
            .any(|frame| matches!(frame.decision, ModelFrameDecision::Conflict))
    }

    pub fn automatic_applications(&self) -> Vec<ModelFrameApplication> {
        self.frames
            .iter()
            .filter_map(|frame| match frame.decision {
                ModelFrameDecision::Automatic(delta) => Some(ModelFrameApplication {
                    target_ref: frame.target_ref,
                    path: frame.path.clone(),
                    order: frame.order,
                    parent_order: frame.parent_order,
                    delta,
                }),
                ModelFrameDecision::Conflict => None,
            })
            .collect()
    }

    pub fn apply_automatic_to_base(&self, dom: &mut WeakDom) {
        apply_model_frame_plan(dom, &self.automatic_applications());
    }

    pub fn apply_automatic_to_ours(&self, dom: &mut WeakDom) {
        let frames: Vec<_> = self
            .automatic_applications()
            .into_iter()
            .filter_map(|mut application| {
                let target = self
                    .frames
                    .iter()
                    .find(|frame| frame.order == application.order)?
                    .ours_ref?;
                application.target_ref = target;
                Some(application)
            })
            .collect();
        apply_model_frame_plan(dom, &frames);
    }

    pub fn apply_automatic_to_theirs(&self, dom: &mut WeakDom) {
        let frames: Vec<_> = self
            .automatic_applications()
            .into_iter()
            .filter_map(|mut application| {
                let target = self
                    .frames
                    .iter()
                    .find(|frame| frame.order == application.order)?
                    .theirs_ref?;
                application.target_ref = target;
                Some(application)
            })
            .collect();
        apply_model_frame_plan(dom, &frames);
    }

    pub fn apply_automatic_to_compact_ours(&self, dom: &mut DiffDom) {
        apply_automatic_to_branch(self, dom, true);
    }

    pub fn apply_automatic_to_compact_theirs(&self, dom: &mut DiffDom) {
        apply_automatic_to_branch(self, dom, false);
    }
}

fn apply_automatic_to_branch(merge: &ModelFrameMerge, dom: &mut dyn DomViewMut, ours: bool) {
    let frames: Vec<_> = merge
        .automatic_applications()
        .into_iter()
        .filter_map(|mut application| {
            let frame = merge
                .frames
                .iter()
                .find(|frame| frame.order == application.order)?;
            application.target_ref = if ours {
                frame.ours_ref?
            } else {
                frame.theirs_ref?
            };
            Some(application)
        })
        .collect();
    apply_model_frame_plan_view(dom, &frames);
}

#[derive(Clone)]
struct Boundary {
    base_ref: Ref,
    side_ref: Option<Ref>,
    parent: Option<usize>,
    children: Vec<usize>,
    direct_parts: Vec<(Rigid, Ref)>,
    pivot_delta: Option<Rigid>,
    candidate: Option<Candidate>,
    effective: Rigid,
    local: Rigid,
}

#[derive(Clone)]
struct Candidate {
    delta: Rigid,
    supporting_parts: usize,
}

struct HierarchyDetection {
    boundaries: Vec<Boundary>,
    side_boundary_by_ref: HashMap<Ref, usize>,
    matched_parts: usize,
}

fn collect_matches(
    matcher: &Matcher<'_>,
    base_parent: Ref,
    side_parent: Ref,
    matches: &mut HashMap<Ref, Ref>,
) {
    let result = matcher.match_children(base_parent, side_parent);
    for &(base_ref, side_ref) in &result.matched {
        matches.insert(base_ref, side_ref);
        collect_matches(matcher, base_ref, side_ref, matches);
    }
}

fn matched_refs(base: &dyn DomView, side: &dyn DomView) -> HashMap<Ref, Ref> {
    let base_hashes = LazyHashCache::new_view(base);
    let side_hashes = LazyHashCache::new_view(side);
    let ignored = HashSet::new();
    let base_deep = DeepHashCache::new(base, &ignored);
    let side_deep = DeepHashCache::new(side, &ignored);
    let matcher = Matcher::new(
        base,
        side,
        &base_hashes,
        &side_hashes,
        &base_deep,
        &side_deep,
    );
    let mut matches = HashMap::new();
    collect_matches(&matcher, base.root_ref(), side.root_ref(), &mut matches);
    matches
}

fn cframe_property(dom: &dyn DomView, referent: Ref) -> Option<CFrame> {
    let instance = dom.get_by_ref(referent)?;
    match instance.property("CFrame") {
        Some(Variant::CFrame(cframe)) => Some(*cframe),
        _ => None,
    }
}

fn pivot_property(dom: &dyn DomView, referent: Ref) -> Option<CFrame> {
    let instance = dom.get_by_ref(referent)?;
    if !class_is_a(instance.class(), "Model") || class_is_a(instance.class(), "WorldRoot") {
        return None;
    }
    match instance.property("WorldPivotData") {
        Some(Variant::OptionalCFrame(Some(cframe))) => Some(*cframe),
        _ => None,
    }
}

fn select_candidate(
    direct_parts: &[(Rigid, Ref)],
    child_candidates: &[Option<Candidate>],
    pivot_delta: Option<Rigid>,
) -> Option<Candidate> {
    struct Cluster {
        comparison_delta: Rigid,
        consensus_delta: Rigid,
        units: usize,
        supporting_parts: usize,
    }

    let total_units = direct_parts.len() + child_candidates.len();
    if total_units == 0 {
        return None;
    }

    let mut clusters: Vec<Cluster> = Vec::new();
    let mut add_vote = |delta: Rigid, supporting_parts: usize| {
        if let Some(cluster) = clusters
            .iter_mut()
            .find(|cluster| Rigid::close(&cluster.comparison_delta, &delta))
        {
            cluster.units += 1;
            cluster.consensus_delta = cluster.consensus_delta.weighted_average(
                cluster.supporting_parts,
                delta,
                supporting_parts,
            );
            cluster.supporting_parts += supporting_parts;
        } else {
            clusters.push(Cluster {
                comparison_delta: delta,
                consensus_delta: delta,
                units: 1,
                supporting_parts,
            });
        }
    };

    for (delta, _) in direct_parts {
        add_vote(*delta, 1);
    }
    for candidate in child_candidates.iter().flatten() {
        add_vote(candidate.delta, candidate.supporting_parts);
    }

    clusters.sort_unstable_by_key(|cluster| std::cmp::Reverse(cluster.units));
    let best = clusters.first()?;
    let tied = clusters
        .get(1)
        .is_some_and(|runner_up| runner_up.units == best.units);
    // Content alone wins only with an unambiguous strict majority and at
    // least two independent units. A pivot is not a second content unit.
    if !tied && best.units >= MIN_CONTENT_SUPPORT && best.units * 2 > total_units {
        return Some(Candidate {
            delta: best.consensus_delta,
            supporting_parts: best.supporting_parts,
        });
    }

    // A matching pivot may break a content tie between viable clusters, but
    // only when adding that corroborating observation produces a strict
    // majority. This deliberately rejects one-part and diffuse matches.
    let pivot = pivot_delta?;
    let mut corroborated = clusters
        .iter()
        .filter(|cluster| Rigid::close(&cluster.comparison_delta, &pivot))
        .filter(|cluster| cluster.units >= MIN_CONTENT_SUPPORT)
        .filter(|cluster| (cluster.units + 1) * 2 > total_units + 1);
    let selected = corroborated.next()?;
    if corroborated.next().is_some() {
        return None;
    }
    Some(Candidate {
        delta: selected.consensus_delta,
        supporting_parts: selected.supporting_parts,
    })
}

fn detect_hierarchy_from_identity(
    base: &dyn DomView,
    side: &dyn DomView,
    base_to_side: &HashMap<Ref, Ref>,
) -> HierarchyDetection {
    let mut boundaries = Vec::new();
    let mut side_boundary_by_ref = HashMap::new();
    let mut matched_parts = 0;

    // Build boundaries and assign their directly-owned BaseParts together.
    // Carrying the active boundary through one preorder traversal avoids
    // materializing an ordered match vector, rebuilding the identity map,
    // and walking every matched part back up through its ancestors.
    let mut pending = base
        .get_by_ref(base.root_ref())
        .into_iter()
        .flat_map(|root| root.children().rev())
        .map(|referent| (referent, None, true))
        .collect::<Vec<_>>();
    while let Some((base_ref, parent_boundary, force_boundary)) = pending.pop() {
        let Some(base_instance) = base.get_by_ref(base_ref) else {
            continue;
        };
        let creates_boundary = force_boundary
            || (class_is_a(base_instance.class(), "Model")
                && !class_is_a(base_instance.class(), "WorldRoot"));
        let active_boundary = if creates_boundary {
            let side_ref = base_to_side.get(&base_ref).copied();
            let pivot_delta = side_ref.and_then(|side_ref| {
                let base_pivot = pivot_property(base, base_ref)?;
                let side_pivot = pivot_property(side, side_ref)?;
                Some(if cframes_equal(base_pivot, side_pivot) {
                    Rigid::identity()
                } else {
                    Rigid::delta_cframes(&side_pivot, &base_pivot)
                })
            });
            let index = boundaries.len();
            boundaries.push(Boundary {
                base_ref,
                side_ref,
                parent: parent_boundary,
                children: Vec::new(),
                direct_parts: Vec::new(),
                pivot_delta,
                candidate: None,
                effective: Rigid::identity(),
                local: Rigid::identity(),
            });
            if let Some(parent) = parent_boundary {
                boundaries[parent].children.push(index);
            }
            if let Some(side_ref) = side_ref {
                side_boundary_by_ref.insert(side_ref, index);
            }
            Some(index)
        } else {
            parent_boundary
        };

        if let (Some(owner), Some(&side_ref)) = (active_boundary, base_to_side.get(&base_ref)) {
            if let Some(side_instance) = side.get_by_ref(side_ref) {
                if class_is_a(base_instance.class(), "BasePart")
                    && class_is_a(side_instance.class(), "BasePart")
                {
                    if let (Some(base_cframe), Some(side_cframe)) = (
                        cframe_property(base, base_ref),
                        cframe_property(side, side_ref),
                    ) {
                        let delta = if cframes_equal(base_cframe, side_cframe) {
                            Rigid::identity()
                        } else {
                            Rigid::delta_cframes(&side_cframe, &base_cframe)
                        };
                        boundaries[owner].direct_parts.push((delta, base_ref));
                        matched_parts += 1;
                    }
                }
            }
        }

        pending.extend(
            base_instance
                .children()
                .rev()
                .map(|child| (child, active_boundary, false)),
        );
    }

    // Boundaries are emitted in preorder, so reverse order is bottom-up.
    for index in (0..boundaries.len()).rev() {
        let child_candidates: Vec<Option<Candidate>> = boundaries[index]
            .children
            .iter()
            .map(|&child| boundaries[child].candidate.clone())
            .collect();
        let candidate = select_candidate(
            &boundaries[index].direct_parts,
            &child_candidates,
            boundaries[index].pivot_delta,
        );
        boundaries[index].candidate = candidate;
    }

    // Resolve inheritance and factor absolute motion into local boundary
    // transforms. Preorder guarantees the parent effective frame is ready.
    for index in 0..boundaries.len() {
        let parent_effective = boundaries[index]
            .parent
            .map(|parent| boundaries[parent].effective)
            .unwrap_or_else(Rigid::identity);
        let effective = boundaries[index]
            .candidate
            .as_ref()
            .map(|candidate| candidate.delta)
            .unwrap_or(parent_effective);
        boundaries[index].effective = effective;
        boundaries[index].local = effective.mul(parent_effective.inverse());
    }

    HierarchyDetection {
        boundaries,
        side_boundary_by_ref,
        matched_parts,
    }
}

fn detect_hierarchy(base: &dyn DomView, side: &dyn DomView) -> HierarchyDetection {
    let identity = matched_refs(base, side);
    detect_hierarchy_from_identity(base, side, &identity)
}

fn transform_world_refs(dom: &mut dyn DomViewMut, refs: Vec<(Ref, Rigid)>) {
    for (referent, alignment) in refs {
        // Rewriting a CFrame through an identity f64 round-trip can still
        // change its exact f32 representation. Besides being needless, doing
        // this across an unchanged place defeats deep-hash pruning and forces
        // the display diff to walk the entire world.
        if Rigid::close(&alignment, &Rigid::identity()) {
            continue;
        }
        let replacement = {
            let Some(instance) = dom.get_by_ref(referent) else {
                continue;
            };
            if class_is_a(instance.class(), "BasePart") {
                match instance.property("CFrame") {
                    Some(Variant::CFrame(cframe)) => Some((
                        "CFrame",
                        Variant::CFrame(alignment.mul(Rigid::from_cframe(cframe)).to_cframe()),
                    )),
                    _ => None,
                }
            } else if class_is_a(instance.class(), "Model")
                && !class_is_a(instance.class(), "WorldRoot")
            {
                match instance.property("WorldPivotData") {
                    Some(Variant::OptionalCFrame(Some(cframe))) => Some((
                        "WorldPivotData",
                        Variant::OptionalCFrame(Some(
                            alignment.mul(Rigid::from_cframe(cframe)).to_cframe(),
                        )),
                    )),
                    _ => None,
                }
            } else {
                None
            }
        };
        if let Some((name, value)) = replacement {
            let updated = dom.set_existing_property(referent, name, value);
            debug_assert!(updated);
        }
    }
}

fn snapshot_world_properties(dom: &dyn DomView) -> Vec<(Ref, &'static str, Variant)> {
    let mut snapshot = Vec::new();
    for referent in DescendantRefs::new(dom) {
        let instance = dom
            .get_by_ref(referent)
            .expect("DomView descendant disappeared while taking frame snapshot");
        if class_is_a(instance.class(), "BasePart") {
            if let Some(value) = instance.property("CFrame") {
                snapshot.push((referent, "CFrame", value.clone()));
            }
        }
        if class_is_a(instance.class(), "Model") && !class_is_a(instance.class(), "WorldRoot") {
            if let Some(value) = instance.property("WorldPivotData") {
                snapshot.push((referent, "WorldPivotData", value.clone()));
            }
        }
    }
    snapshot
}

fn restore_world_properties(dom: &mut dyn DomViewMut, snapshot: Vec<(Ref, &'static str, Variant)>) {
    for (referent, property, value) in snapshot {
        let updated = dom.set_existing_property(referent, property, value);
        debug_assert!(updated);
    }
}

fn transform_world_properties_below(dom: &mut dyn DomViewMut, root: Ref, alignment: Rigid) {
    let mut pending = vec![root];
    let mut refs = Vec::new();
    while let Some(referent) = pending.pop() {
        let Some(instance) = dom.get_by_ref(referent) else {
            continue;
        };
        pending.extend(instance.children());
        refs.push((referent, alignment));
    }
    transform_world_refs(dom, refs);
}

fn canonicalize_side(dom: &mut dyn DomViewMut, detection: &HierarchyDetection) {
    let mut pending: Vec<(Ref, Option<Rigid>)> = dom
        .get_by_ref(dom.root_ref())
        .into_iter()
        .flat_map(|root| root.children())
        .map(|referent| (referent, None))
        .collect();
    let mut refs = Vec::new();
    while let Some((referent, inherited)) = pending.pop() {
        let active = detection
            .side_boundary_by_ref
            .get(&referent)
            .map(|&boundary| detection.boundaries[boundary].effective)
            .or(inherited);
        let Some(instance) = dom.get_by_ref(referent) else {
            continue;
        };
        pending.extend(instance.children().map(|child| (child, active)));
        if let Some(frame) = active {
            refs.push((referent, frame.inverse()));
        }
    }
    transform_world_refs(dom, refs);
}

fn root_prefixes(detection: &HierarchyDetection) -> Vec<Rigid> {
    let mut prefixes = Vec::with_capacity(detection.boundaries.len());
    for boundary in &detection.boundaries {
        let prefix = match boundary.parent {
            Some(parent) => prefixes[parent],
            None => boundary
                .candidate
                .as_ref()
                .map(|candidate| candidate.delta)
                .unwrap_or_else(Rigid::identity),
        };
        prefixes.push(prefix);
    }
    prefixes
}

fn canonicalize_roots(
    dom: &mut dyn DomViewMut,
    detection: &HierarchyDetection,
    prefixes: &[Rigid],
) {
    for (index, boundary) in detection.boundaries.iter().enumerate() {
        if boundary.parent.is_some() {
            continue;
        }
        let Some(side_root) = boundary.side_ref else {
            continue;
        };
        transform_world_properties_below(dom, side_root, prefixes[index].inverse());
    }
}

/// Convert a local frame detected after root canonicalization back into the
/// raw branch's coordinate system. For a nested boundary this is conjugation
/// by the removed root prefix; for a root it is direct composition.
fn raw_local_frame(boundary: &Boundary, root_prefix: Rigid) -> Rigid {
    // Root canonicalization is serialized through f32 before the second
    // detection pass. Do not fold that representation residue back into the
    // authored frame we just removed.
    let local = if Rigid::close(&boundary.local, &Rigid::identity()) {
        Rigid::identity()
    } else {
        boundary.local
    };
    if boundary.parent.is_none() {
        root_prefix.mul(local)
    } else {
        root_prefix.mul(local).mul(root_prefix.inverse())
    }
}

fn script_identity(script: EditScript) -> InstanceIdentity {
    script.identity
}

/// Apply a frame to every world-space property in one boundary subtree.
pub fn apply_model_frame(dom: &mut WeakDom, target: Ref, frame: &CFrame) {
    transform_world_properties_below(dom, target, Rigid::from_cframe(frame));
}

/// Compose local frame decisions, then write each world-space property once
/// using the effective frame of its nearest participating boundary.
pub fn apply_model_frame_plan(dom: &mut WeakDom, frames: &[ModelFrameApplication]) {
    apply_model_frame_plan_view(dom, frames);
}

fn apply_model_frame_plan_view(dom: &mut dyn DomViewMut, frames: &[ModelFrameApplication]) {
    let mut ordered: Vec<_> = frames.iter().collect();
    ordered.sort_unstable_by_key(|frame| frame.order);
    let mut effective_by_order: HashMap<usize, Rigid> = HashMap::new();
    let mut effective_by_target = HashMap::new();
    for frame in ordered {
        let parent = frame
            .parent_order
            .and_then(|order| effective_by_order.get(&order).copied())
            .unwrap_or_else(Rigid::identity);
        let effective = Rigid::from_cframe(&frame.delta).mul(parent);
        effective_by_order.insert(frame.order, effective);
        effective_by_target.insert(frame.target_ref, effective);
    }

    let mut pending: Vec<(Ref, Option<Rigid>)> = dom
        .get_by_ref(dom.root_ref())
        .into_iter()
        .flat_map(|root| root.children())
        .map(|referent| (referent, None))
        .collect();
    let mut refs = Vec::new();
    while let Some((referent, inherited)) = pending.pop() {
        let active = effective_by_target.get(&referent).copied().or(inherited);
        let Some(instance) = dom.get_by_ref(referent) else {
            continue;
        };
        pending.extend(instance.children().map(|child| (child, active)));
        if let Some(effective) = active {
            refs.push((referent, effective));
        }
    }
    transform_world_refs(dom, refs);
}

/// Legacy whole-DOM helper retained for callers that explicitly have one
/// asset-wide frame. Hierarchical merge code applies boundary plans instead.
pub fn apply_model_frame_to_dom(dom: &mut WeakDom, frame: &CFrame) {
    let roots: Vec<Ref> = dom.root().children().to_vec();
    for root in roots {
        apply_model_frame(dom, root, frame);
    }
}

pub fn model_frames_close(a: &CFrame, b: &CFrame) -> bool {
    Rigid::close(&Rigid::from_cframe(a), &Rigid::from_cframe(b))
}

fn identity_frame() -> CFrame {
    Rigid::identity().to_cframe()
}

/// Canonicalize one side of a two-way model-asset diff and return one local
/// frame change for every affected boundary.
///
/// Root alignment happens before authoritative matching for the same reason
/// as three-way normalization: two saves of the same asset can be arbitrarily
/// far apart, and duplicate-heavy content needs a common frame before stable
/// identity can be established. The resulting identity is then frozen while
/// nested frames are removed.
pub fn normalize_model_diff_frames(base: &WeakDom, side: &mut WeakDom) -> Option<ModelFrameDiff> {
    let state = prepare_model_diff_frames_view(base, side);
    (!state.frames.is_empty()).then_some(state)
}

/// Establish complete identity and optionally canonicalize inferred frames.
///
/// Compact diffing consumes the identity even when no frame was inferred, so
/// the internal result is deliberately not optional. The public API retains
/// its existing `None` result when there are no model frames.
pub(crate) fn prepare_model_diff_frames_view(
    base: &dyn DomView,
    side: &mut dyn DomViewMut,
) -> ModelFrameDiff {
    let initial_identity = compute_instance_identity(base, side.as_view(), &DiffConfig::default());
    let initial = detect_hierarchy_from_identity(base, side.as_view(), &initial_identity.matched);
    let prefixes = root_prefixes(&initial);
    let roots_changed = initial
        .boundaries
        .iter()
        .enumerate()
        .any(|(index, boundary)| {
            boundary.parent.is_none() && !Rigid::close(&prefixes[index], &Rigid::identity())
        });

    // A nested-only move leaves the DOM untouched during identity discovery,
    // so its initial detection is already authoritative. Reusing it avoids a
    // second place-wide boundary tree and world-property snapshot. Only a
    // changed serialized root requires mutation, rematching, and restoration.
    let (identity, mut detection, mut raw) = if roots_changed {
        let raw = snapshot_world_properties(side.as_view());
        canonicalize_roots(side, &initial, &prefixes);
        let identity = compute_instance_identity(base, side.as_view(), &DiffConfig::default());
        let detection = detect_hierarchy_from_identity(base, side.as_view(), &identity.matched);
        (identity, detection, Some(raw))
    } else {
        (initial_identity, initial, None)
    };

    let identity_frame = identity_frame();
    let mut nearest_frame = vec![None; detection.boundaries.len()];
    let mut frames = Vec::new();
    for (order, boundary) in detection.boundaries.iter().enumerate() {
        let local = raw_local_frame(boundary, prefixes[order]).to_cframe();
        let parent_order = boundary.parent.and_then(|parent| nearest_frame[parent]);
        if model_frames_close(&local, &identity_frame) {
            nearest_frame[order] = parent_order;
            continue;
        }
        let Some(side_ref) = boundary.side_ref else {
            if let Some(raw) = raw.take() {
                restore_world_properties(side, raw);
            }
            return ModelFrameDiff {
                frames: Vec::new(),
                detected: detection
                    .boundaries
                    .iter()
                    .filter(|boundary| boundary.candidate.is_some())
                    .count(),
                identity,
            };
        };
        frames.push(ModelFrameChange {
            base_ref: boundary.base_ref,
            side_ref,
            path: get_instance_path(side.as_view(), side_ref),
            order,
            parent_order,
            delta: local,
        });
        nearest_frame[order] = Some(order);
    }

    if frames.is_empty() {
        if let Some(raw) = raw {
            restore_world_properties(side, raw);
        }
        return ModelFrameDiff {
            frames,
            detected: detection
                .boundaries
                .iter()
                .filter(|boundary| boundary.candidate.is_some())
                .count(),
            identity,
        };
    }

    // Detection above happened in root-aligned coordinates. Restore the raw
    // file, convert effective frames back to that coordinate system, and
    // remove each effective frame exactly once from its owned properties.
    for (index, boundary) in detection.boundaries.iter_mut().enumerate() {
        boundary.effective = prefixes[index].mul(boundary.effective);
    }
    if let Some(raw) = raw {
        restore_world_properties(side, raw);
    }
    canonicalize_side(side, &detection);

    ModelFrameDiff {
        frames,
        detected: detection
            .boundaries
            .iter()
            .filter(|boundary| boundary.candidate.is_some())
            .count(),
        identity,
    }
}

pub(crate) fn normalize_model_diff_frames_view(
    base: &dyn DomView,
    side: &mut dyn DomViewMut,
) -> Option<ModelFrameDiff> {
    let state = prepare_model_diff_frames_view(base, side);
    (!state.frames.is_empty()).then_some(state)
}

/// Canonicalize both branches and return independently mergeable local frame
/// decisions for every affected boundary.
pub fn normalize_model_merge_frames(
    base: &WeakDom,
    ours: &mut WeakDom,
    theirs: &mut WeakDom,
) -> Option<ModelFrameMerge> {
    normalize_model_merge_frames_view(base, ours, theirs)
}

/// Compact-branch variant of [`normalize_model_merge_frames`].
pub fn normalize_model_merge_compact_frames(
    base: &WeakDom,
    ours: &mut DiffDom,
    theirs: &mut DiffDom,
) -> Option<ModelFrameMerge> {
    normalize_model_merge_frames_view(base, ours, theirs)
}

fn normalize_model_merge_frames_view(
    base: &dyn DomView,
    ours: &mut dyn DomViewMut,
    theirs: &mut dyn DomViewMut,
) -> Option<ModelFrameMerge> {
    let ours_raw = snapshot_world_properties(ours);
    let theirs_raw = snapshot_world_properties(theirs);
    // First remove only serialized-root placement. This gives duplicate-heavy
    // assets a stable coordinate system for their authoritative identity map.
    let ours_initial = detect_hierarchy(base, ours);
    let theirs_initial = detect_hierarchy(base, theirs);
    let ours_prefixes = root_prefixes(&ours_initial);
    let theirs_prefixes = root_prefixes(&theirs_initial);
    canonicalize_roots(ours, &ours_initial, &ours_prefixes);
    canonicalize_roots(theirs, &theirs_initial, &theirs_prefixes);

    // Rebuild matching after root alignment, then retain this mapping through
    // nested canonicalization and the merge itself.
    let ours_script =
        compute_semantic_changes_with_identity(base, ours.as_view(), &DiffConfig::default(), None);
    let theirs_script = compute_semantic_changes_with_identity(
        base,
        theirs.as_view(),
        &DiffConfig::default(),
        None,
    );
    let ours_identity = script_identity(ours_script);
    let theirs_identity = script_identity(theirs_script);
    let mut ours_detection = detect_hierarchy_from_identity(base, ours, &ours_identity.matched);
    let mut theirs_detection =
        detect_hierarchy_from_identity(base, theirs, &theirs_identity.matched);
    if ours_detection.boundaries.len() != theirs_detection.boundaries.len() {
        return None;
    }

    let identity = identity_frame();
    let mut frames = Vec::new();
    let mut nearest_frame = vec![None; ours_detection.boundaries.len()];
    for (order, (ours_boundary, theirs_boundary)) in ours_detection
        .boundaries
        .iter()
        .zip(&theirs_detection.boundaries)
        .enumerate()
    {
        if ours_boundary.base_ref != theirs_boundary.base_ref {
            return None;
        }
        let ours_local = raw_local_frame(ours_boundary, ours_prefixes[order]).to_cframe();
        let theirs_local = raw_local_frame(theirs_boundary, theirs_prefixes[order]).to_cframe();
        let parent_order = ours_boundary
            .parent
            .and_then(|parent| nearest_frame[parent]);
        if model_frames_close(&ours_local, &identity)
            && model_frames_close(&theirs_local, &identity)
        {
            nearest_frame[order] = parent_order;
            continue;
        }
        let decision = if model_frames_close(&ours_local, &theirs_local) {
            ModelFrameDecision::Automatic(ours_local)
        } else if model_frames_close(&ours_local, &identity) {
            ModelFrameDecision::Automatic(theirs_local)
        } else if model_frames_close(&theirs_local, &identity) {
            ModelFrameDecision::Automatic(ours_local)
        } else {
            ModelFrameDecision::Conflict
        };
        frames.push(ModelFrame {
            target_ref: ours_boundary.base_ref,
            ours_ref: ours_boundary.side_ref,
            theirs_ref: theirs_boundary.side_ref,
            path: get_instance_path(base, ours_boundary.base_ref),
            order,
            parent_order,
            ours: ours_local,
            theirs: theirs_local,
            decision,
        });
        nearest_frame[order] = Some(order);
    }

    if frames.is_empty() {
        return None;
    }

    // Return to the raw branches and canonicalize each property once with its
    // combined raw absolute frame. The root-aligned copies above existed only
    // to establish stable identity and nested evidence.
    for (index, boundary) in ours_detection.boundaries.iter_mut().enumerate() {
        boundary.effective = ours_prefixes[index].mul(boundary.effective);
    }
    for (index, boundary) in theirs_detection.boundaries.iter_mut().enumerate() {
        boundary.effective = theirs_prefixes[index].mul(boundary.effective);
    }
    restore_world_properties(ours, ours_raw);
    restore_world_properties(theirs, theirs_raw);
    canonicalize_side(ours, &ours_detection);
    canonicalize_side(theirs, &theirs_detection);
    Some(ModelFrameMerge {
        frames,
        ours_detected: ours_detection
            .boundaries
            .iter()
            .filter(|boundary| boundary.candidate.is_some())
            .count(),
        theirs_detected: theirs_detection
            .boundaries
            .iter()
            .filter(|boundary| boundary.candidate.is_some())
            .count(),
        ours_identity,
        theirs_identity,
    })
}

/// Express `side` in the serialized root frame of `base`.
///
/// Nested boundaries participate in bottom-up evidence, but only one
/// top-level root frame is removed. Nested-local movement therefore remains a
/// normal authored difference in a two-way diff. Multi-root assets are left
/// untouched because they do not have one representable asset frame.
pub fn normalize_model_dom_to_base(
    base: &WeakDom,
    side: &mut WeakDom,
) -> Option<ModelNormalization> {
    let detection = detect_hierarchy(base, side);
    let roots: Vec<&Boundary> = detection
        .boundaries
        .iter()
        .filter(|boundary| boundary.parent.is_none())
        .collect();
    let [root] = roots.as_slice() else {
        return None;
    };
    let candidate = root.candidate.as_ref()?;
    let side_root = root.side_ref?;

    transform_world_properties_below(side, side_root, candidate.delta.inverse());
    Some(ModelNormalization {
        matched_parts: detection.matched_parts,
        supporting_parts: candidate.supporting_parts,
        side_delta: candidate.delta.to_cframe(),
        base_target_ref: root.base_ref,
    })
}
