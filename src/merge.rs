//! Three-way merge: combine two edit scripts computed against a common base.
//!
//! Both branches' scripts address instances by base-DOM refs, so combining is
//! set logic over op targets: ops touching different targets compose, ops with
//! the identical effect dedupe, ops with different effects on the same target
//! conflict. Conflicted ops are NOT applied — the base content is kept and the
//! conflict is reported for a resolver to decide.

use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::{CFrame, Variant};
use std::collections::{HashMap, HashSet};
use tracing::info;

use crate::diff::DiffConfig;
use crate::diff_dom::{DiffDom, DomView};
use crate::edit_script::{
    apply_ops_filtered, compute_semantic_changes_with_caches, Anchor, DomCaches, EditOp,
    EditScript, InstanceIdentity,
};
use crate::explorer_tree::ExplorerTrees;
use crate::hash::DeepHashCache;
use crate::match_instances::get_instance_path;
use crate::model_normalize::pivot_deltas_close;
use crate::placement::PivotOp;
use crate::property_semantics::{
    semantic_bundle_values_equal, semantic_property_bundle, SemanticPropertyBundle,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictKind {
    /// Both sides set the same property (or name) to different values.
    Property { name: String },
    /// Both sides changed fields that form one indivisible serialized value.
    PropertyBundle {
        name: String,
        properties: Vec<String>,
    },
    /// One side removed a subtree the other side edited/moved into or out of.
    DeleteVsEdit,
    /// Both sides moved the same instance to different parents, or a move
    /// destination can't be proven equal across branches.
    MoveTarget,
    /// Both branches placed an otherwise canonical hierarchy boundary
    /// differently. A root delta is world-relative; a nested delta is
    /// relative to the nearest participating ancestor placement.
    Pivot {
        ours: CFrame,
        theirs: CFrame,
        /// Stable top-down order among hierarchical frame boundaries.
        order: usize,
        /// Nearest ancestor that also has a placement decision.
        parent_order: Option<usize>,
    },
}

#[derive(Debug)]
pub struct ConflictSide {
    pub edits: Vec<EditOp>,
    pub pivots: Vec<PivotOp>,
}

impl ConflictSide {
    fn edits(edits: Vec<EditOp>) -> Self {
        Self {
            edits,
            pivots: Vec::new(),
        }
    }

    fn pivots(pivots: Vec<PivotOp>) -> Self {
        Self {
            edits: Vec::new(),
            pivots,
        }
    }

    /// Number of primitive semantic operations represented by this choice.
    pub fn len(&self) -> usize {
        self.edits.len() + self.pivots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty() && self.pivots.is_empty()
    }
}

#[derive(Debug)]
pub struct MergeConflict {
    pub kind: ConflictKind,
    /// Base-DOM ref of the contested instance (subtree root for DeleteVsEdit).
    pub base_ref: Ref,
    /// Path of the contested instance in the base DOM.
    pub path: String,
    pub ours: ConflictSide,
    pub theirs: ConflictSide,
}

#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeStats {
    pub ours_applied: usize,
    pub theirs_applied: usize,
    pub deduped: usize,
    pub conflicted: usize,
}

pub struct MergeResult {
    pub conflicts: Vec<MergeConflict>,
    pub stats: MergeStats,
    /// Non-conflicting primitive placements. These are materialized after
    /// ordinary canonical-coordinate edits, in top-down order.
    pub pivots: Vec<PivotOp>,
    pub ours_identity: InstanceIdentity,
    pub theirs_identity: InstanceIdentity,
    pub(crate) explorer_trees: Option<ExplorerTrees>,
}

/// Three-way merge: mutate `base` into the merged result, applying every
/// non-conflicting change from both branches. Conflicted targets keep their
/// base content and are reported in the result.
pub fn merge_doms(
    base: &mut WeakDom,
    ours: &WeakDom,
    theirs: &WeakDom,
    config: &DiffConfig,
) -> MergeResult {
    merge_with_shared_caches(base, ours, theirs, config, None, None, &[], &[], false)
}

/// Three-way merge with compact immutable branch inputs.
///
/// The base remains a `WeakDom` because it is the materialized result. Branch
/// comparison and subtree payload access only require `DomView`.
pub fn merge_compact_doms(
    base: &mut WeakDom,
    ours: &DiffDom,
    theirs: &DiffDom,
    config: &DiffConfig,
) -> MergeResult {
    merge_with_shared_caches(base, ours, theirs, config, None, None, &[], &[], false)
}

/// Three-way merge using instance identities captured before a
/// representation-only normalization pass. This prevents canonical CFrames
/// from making duplicate siblings reshuffle during the real merge.
pub fn merge_doms_with_matches(
    base: &mut WeakDom,
    ours: &WeakDom,
    theirs: &WeakDom,
    config: &DiffConfig,
    ours_identity: &InstanceIdentity,
    theirs_identity: &InstanceIdentity,
) -> MergeResult {
    merge_with_shared_caches(
        base,
        ours,
        theirs,
        config,
        Some(ours_identity),
        Some(theirs_identity),
        &[],
        &[],
        true,
    )
}

/// Compact-branch variant of [`merge_doms_with_matches`].
pub fn merge_compact_doms_with_matches(
    base: &mut WeakDom,
    ours: &DiffDom,
    theirs: &DiffDom,
    config: &DiffConfig,
    ours_identity: &InstanceIdentity,
    theirs_identity: &InstanceIdentity,
) -> MergeResult {
    merge_with_shared_caches(
        base,
        ours,
        theirs,
        config,
        Some(ours_identity),
        Some(theirs_identity),
        &[],
        &[],
        true,
    )
}

/// Three-way merge with identities and primitive placements captured by
/// hierarchical normalization.
#[allow(clippy::too_many_arguments)]
pub fn merge_compact_doms_with_matches_and_pivots(
    base: &mut WeakDom,
    ours: &DiffDom,
    theirs: &DiffDom,
    config: &DiffConfig,
    ours_identity: &InstanceIdentity,
    theirs_identity: &InstanceIdentity,
    ours_pivots: &[PivotOp],
    theirs_pivots: &[PivotOp],
) -> MergeResult {
    merge_with_shared_caches(
        base,
        ours,
        theirs,
        config,
        Some(ours_identity),
        Some(theirs_identity),
        ours_pivots,
        theirs_pivots,
        false,
    )
}

/// Shared entry: plan both branch scripts against ONE set of base caches
/// (halving base-side hashing), then combine. The base caches only live for
/// the immutable planning scope; the branch deep caches stay warm for the
/// combiner's add-dedup join.
#[allow(clippy::too_many_arguments)]
fn merge_with_shared_caches(
    base: &mut WeakDom,
    ours_dom: &dyn DomView,
    theirs_dom: &dyn DomView,
    config: &DiffConfig,
    ours_identity: Option<&InstanceIdentity>,
    theirs_identity: Option<&InstanceIdentity>,
    ours_pivots: &[PivotOp],
    theirs_pivots: &[PivotOp],
    capture_explorer: bool,
) -> MergeResult {
    let (ours_script, theirs_script, ours_caches, theirs_caches) = {
        let base_view: &WeakDom = base;
        let base_caches = DomCaches::new(base_view, &config.ignore_properties);
        let ours_caches = DomCaches::new(ours_dom, &config.ignore_properties);
        let theirs_caches = DomCaches::new(theirs_dom, &config.ignore_properties);
        let mut ours_script = compute_semantic_changes_with_caches(
            base_view,
            ours_dom,
            config,
            ours_identity,
            &base_caches,
            &ours_caches,
        );
        let mut theirs_script = compute_semantic_changes_with_caches(
            base_view,
            theirs_dom,
            config,
            theirs_identity,
            &base_caches,
            &theirs_caches,
        );
        ours_script.pivots.extend_from_slice(ours_pivots);
        theirs_script.pivots.extend_from_slice(theirs_pivots);
        (ours_script, theirs_script, ours_caches, theirs_caches)
    };
    merge_scripts(
        base,
        ours_dom,
        theirs_dom,
        &ours_script,
        &theirs_script,
        capture_explorer,
        &ours_caches.deep,
        &theirs_caches.deep,
    )
}

#[allow(clippy::too_many_arguments)]
fn merge_scripts(
    base: &mut WeakDom,
    ours_dom: &dyn DomView,
    theirs_dom: &dyn DomView,
    ours: &EditScript,
    theirs: &EditScript,
    capture_explorer: bool,
    ours_deep: &DeepHashCache<'_>,
    theirs_deep: &DeepHashCache<'_>,
) -> MergeResult {
    let mut conflicts: Vec<MergeConflict> = Vec::new();
    let mut stats = MergeStats::default();

    let ours_removed: HashSet<Ref> = removed_roots(&ours.ops);
    let theirs_removed: HashSet<Ref> = removed_roots(&theirs.ops);

    // Base refs whose ops conflicted — their ops are withheld on both sides
    let mut conflicted_ours = vec![false; ours.ops.len()];
    let mut conflicted_theirs = vec![false; theirs.ops.len()];
    // Theirs op indices subsumed or deduped by an ours op
    let mut dropped_theirs = vec![false; theirs.ops.len()];

    // A subtree deletion is one semantic choice even when the surviving side
    // changed many descendants. Group those edits by deleted root so the
    // resolver can present (and apply) the whole branch decision once instead
    // of emitting one identical DeleteVsEdit row per low-level edit op.
    let mut ours_edits_vs_theirs_delete: HashMap<Ref, usize> = HashMap::new();
    let mut theirs_edits_vs_ours_delete: HashMap<Ref, usize> = HashMap::new();

    // Moves indexed by their moved instance, for the symmetric-evacuation
    // check below.
    let ours_move_anchor: HashMap<Ref, Anchor> = move_anchors(&ours.ops);
    let theirs_move_anchor: HashMap<Ref, Anchor> = move_anchors(&theirs.ops);

    // Pair up identical additions before any conflict pass runs: the pairing
    // depends only on content, and the per-instance equivalence it yields
    // lets every later equality check treat "each branch's own copy of the
    // same new content" as one logical target. The theirs-side ops are only
    // MARKED deduped later, once conflict passes have had their say.
    let (added_equiv, added_pairs) = pair_identical_adds(
        ours_dom,
        theirs_dom,
        &ours.ops,
        &theirs.ops,
        ours_deep,
        theirs_deep,
    );

    // Instances BOTH branches moved to the same destination (a live base
    // parent or corresponding spots in identical added groups) are alive on
    // both sides no matter what happens to their base surroundings. Ops on
    // or below them must not read as edits inside a deleted subtree: the
    // delete-vs-edit ancestry walk stops at these before it can reach a
    // removed root, and every such op falls through to same-target dedupe.
    let evacuated: HashSet<Ref> = ours_move_anchor
        .iter()
        .filter(|(old_ref, ours_anchor)| {
            theirs_move_anchor
                .get(old_ref)
                .is_some_and(|their_anchor| anchors_equal(**ours_anchor, *their_anchor, &added_equiv))
        })
        .map(|(old_ref, _)| *old_ref)
        .collect();

    // ---- Delete-vs-edit: an op on one side targeting inside the other side's
    // removed subtree (removes of removes compose instead)
    for (i, op) in ours.ops.iter().enumerate() {
        for target in op_base_targets(op).into_iter().flatten() {
            let Some(removed_root) = ancestor_in(base, target, &theirs_removed, &evacuated)
            else {
                continue;
            };
            if let EditOp::RemoveSubtree { old_ref } = op {
                // Identical removes are handled by pair dedupe below; only a
                // remove STRICTLY inside the other side's removed subtree is
                // subsumed by it (else both sides withhold and nothing removes)
                if removed_root == *old_ref {
                    continue;
                }
                stats.deduped += 1;
                conflicted_ours[i] = true; // withhold; outer remove covers it
                break;
            }
            conflicted_ours[i] = true;
            if let Some(&conflict_index) = ours_edits_vs_theirs_delete.get(&removed_root) {
                conflicts[conflict_index].ours.edits.push(op.clone());
            } else {
                let their_op = find_remove(&theirs.ops, removed_root, &mut conflicted_theirs);
                let conflict_index = conflicts.len();
                conflicts.push(MergeConflict {
                    kind: ConflictKind::DeleteVsEdit,
                    base_ref: removed_root,
                    path: get_instance_path(base, removed_root),
                    ours: ConflictSide::edits(vec![op.clone()]),
                    theirs: ConflictSide::edits(their_op),
                });
                ours_edits_vs_theirs_delete.insert(removed_root, conflict_index);
            }
            break;
        }
    }
    for (i, op) in theirs.ops.iter().enumerate() {
        for target in op_base_targets(op).into_iter().flatten() {
            let Some(removed_root) = ancestor_in(base, target, &ours_removed, &evacuated) else {
                continue;
            };
            if let EditOp::RemoveSubtree { old_ref } = op {
                if removed_root == *old_ref {
                    continue;
                }
                stats.deduped += 1;
                dropped_theirs[i] = true;
                break;
            }
            conflicted_theirs[i] = true;
            if let Some(&conflict_index) = theirs_edits_vs_ours_delete.get(&removed_root) {
                conflicts[conflict_index].theirs.edits.push(op.clone());
            } else {
                let our_op = find_remove(&ours.ops, removed_root, &mut conflicted_ours);
                let conflict_index = conflicts.len();
                conflicts.push(MergeConflict {
                    kind: ConflictKind::DeleteVsEdit,
                    base_ref: removed_root,
                    path: get_instance_path(base, removed_root),
                    ours: ConflictSide::edits(our_op),
                    theirs: ConflictSide::edits(vec![op.clone()]),
                });
                theirs_edits_vs_ours_delete.insert(removed_root, conflict_index);
            }
            break;
        }
    }

    // If both branches remove the contested root but one first moves edited
    // descendants out, the nominally-common remove belongs to the conflict
    // on both sides. Mark every duplicate occurrence now: the regular
    // same-target pass skips operations already claimed by a conflict.
    for conflict in &mut conflicts {
        if conflict.kind != ConflictKind::DeleteVsEdit {
            continue;
        }
        let root = conflict.base_ref;
        let ours_removes = find_remove(&ours.ops, root, &mut conflicted_ours);
        let theirs_removes = find_remove(&theirs.ops, root, &mut conflicted_theirs);
        if !conflict
            .ours
            .edits
            .iter()
            .any(|op| matches!(op, EditOp::RemoveSubtree { old_ref } if *old_ref == root))
        {
            conflict.ours.edits.extend(ours_removes);
        }
        if !conflict
            .theirs
            .edits
            .iter()
            .any(|op| matches!(op, EditOp::RemoveSubtree { old_ref } if *old_ref == root))
        {
            conflict.theirs.edits.extend(theirs_removes);
        }
    }
    let pivots = merge_pivots(
        base,
        ours,
        theirs,
        &ours_removed,
        &theirs_removed,
        &evacuated,
        &mut conflicted_ours,
        &mut conflicted_theirs,
        &mut ours_edits_vs_theirs_delete,
        &mut theirs_edits_vs_ours_delete,
        &mut conflicts,
        &mut stats,
    );

    // Serialized support fields can form one authored value. In particular,
    // MeshContent and InitialSize must never be resolved independently: that
    // can attach one mesh's source extent to another mesh and visibly explode
    // its scale. Compare each branch's complete bundle state, then either
    // dedupe it or emit one atomic decision containing all affected ops.
    group_property_bundle_conflicts(
        base,
        ours_dom,
        theirs_dom,
        ours,
        theirs,
        &mut conflicted_ours,
        &mut conflicted_theirs,
        &mut dropped_theirs,
        &mut conflicts,
        &mut stats,
    );

    // ---- Both sides added identical content under the same parent: mark the
    // theirs-side copies deduped. The pairing itself was computed up front;
    // pairs that a conflict pass claimed in the meantime keep both ops for
    // the conflict snapshot instead.
    for &(i, j) in &added_pairs {
        if conflicted_ours[i] || conflicted_theirs[j] || dropped_theirs[j] {
            continue;
        }
        dropped_theirs[j] = true;
        stats.deduped += 1;
    }

    // ---- Same-target op pairs: dedupe identical effects, conflict otherwise.
    // Indexing by the base instance avoids comparing every ours op with every
    // theirs op; the remaining per-target groups are normally only one or two
    // entries wide.
    let mut theirs_by_target: HashMap<Ref, Vec<usize>> = HashMap::new();
    for (index, op) in theirs.ops.iter().enumerate() {
        if let Some(target) = op_primary_target(op) {
            theirs_by_target.entry(target).or_default().push(index);
        }
    }
    for (i, our_op) in ours.ops.iter().enumerate() {
        if conflicted_ours[i] {
            continue;
        }
        let Some(target) = op_primary_target(our_op) else {
            continue;
        };
        let Some(their_indices) = theirs_by_target.get(&target) else {
            continue;
        };
        for &j in their_indices {
            if conflicted_theirs[j] || dropped_theirs[j] {
                continue;
            }
            let their_op = &theirs.ops[j];
            match (our_op, their_op) {
                (
                    EditOp::SetProperty {
                        old_ref: a,
                        name: an,
                        value: av,
                        ..
                    },
                    EditOp::SetProperty {
                        old_ref: b,
                        name: bn,
                        value: bv,
                        ..
                    },
                ) if a == b && an == bn => {
                    if values_equal(
                        av,
                        bv,
                        &ours.identity.reverse_matched,
                        &theirs.identity.reverse_matched,
                        &added_equiv,
                    ) {
                        dropped_theirs[j] = true;
                        stats.deduped += 1;
                    } else {
                        conflicted_ours[i] = true;
                        conflicted_theirs[j] = true;
                        conflicts.push(MergeConflict {
                            kind: ConflictKind::Property { name: an.clone() },
                            base_ref: *a,
                            path: get_instance_path(base, *a),
                            ours: ConflictSide::edits(vec![our_op.clone()]),
                            theirs: ConflictSide::edits(vec![their_op.clone()]),
                        });
                    }
                }
                (
                    EditOp::SetName {
                        old_ref: a,
                        name: an,
                    },
                    EditOp::SetName {
                        old_ref: b,
                        name: bn,
                    },
                ) if a == b => {
                    if an == bn {
                        dropped_theirs[j] = true;
                        stats.deduped += 1;
                    } else {
                        conflicted_ours[i] = true;
                        conflicted_theirs[j] = true;
                        conflicts.push(MergeConflict {
                            kind: ConflictKind::Property {
                                name: "Name".to_string(),
                            },
                            base_ref: *a,
                            path: get_instance_path(base, *a),
                            ours: ConflictSide::edits(vec![our_op.clone()]),
                            theirs: ConflictSide::edits(vec![their_op.clone()]),
                        });
                    }
                }
                (EditOp::RemoveSubtree { old_ref: a }, EditOp::RemoveSubtree { old_ref: b })
                    if a == b =>
                {
                    // A common outer deletion is not independently safe when
                    // one branch first moves edited descendants out of that
                    // subtree. The delete-vs-edit decision owns the complete
                    // branch outcome: keep the base subtree alive for the
                    // resolver, and include both nominally-identical removes
                    // in the existing conflict instead of applying either.
                    let conflict_index = ours_edits_vs_theirs_delete
                        .get(a)
                        .or_else(|| theirs_edits_vs_ours_delete.get(a))
                        .copied();
                    if let Some(conflict_index) = conflict_index {
                        conflicted_ours[i] = true;
                        conflicted_theirs[j] = true;
                        let conflict = &mut conflicts[conflict_index];
                        if !conflict.ours.edits.iter().any(
                            |op| matches!(op, EditOp::RemoveSubtree { old_ref } if old_ref == a),
                        ) {
                            conflict.ours.edits.push(our_op.clone());
                        }
                        if !conflict.theirs.edits.iter().any(
                            |op| matches!(op, EditOp::RemoveSubtree { old_ref } if old_ref == b),
                        ) {
                            conflict.theirs.edits.push(their_op.clone());
                        }
                    } else {
                        dropped_theirs[j] = true;
                        stats.deduped += 1;
                    }
                }
                (
                    EditOp::Move {
                        old_ref: a,
                        new_parent: ap,
                    },
                    EditOp::Move {
                        old_ref: b,
                        new_parent: bp,
                    },
                ) if a == b => {
                    if anchors_equal(*ap, *bp, &added_equiv) {
                        dropped_theirs[j] = true;
                        stats.deduped += 1;
                    } else {
                        conflicted_ours[i] = true;
                        conflicted_theirs[j] = true;
                        conflicts.push(MergeConflict {
                            kind: ConflictKind::MoveTarget,
                            base_ref: *a,
                            path: get_instance_path(base, *a),
                            ours: ConflictSide::edits(vec![our_op.clone()]),
                            theirs: ConflictSide::edits(vec![their_op.clone()]),
                        });
                    }
                }
                (EditOp::Move { old_ref: a, .. }, EditOp::RemoveSubtree { old_ref: b })
                | (EditOp::RemoveSubtree { old_ref: b }, EditOp::Move { old_ref: a, .. })
                    if a == b =>
                {
                    conflicted_ours[i] = true;
                    conflicted_theirs[j] = true;
                    conflicts.push(MergeConflict {
                        kind: ConflictKind::DeleteVsEdit,
                        base_ref: *a,
                        path: get_instance_path(base, *a),
                        ours: ConflictSide::edits(vec![our_op.clone()]),
                        theirs: ConflictSide::edits(vec![their_op.clone()]),
                    });
                }
                _ => {}
            }
        }
    }

    // Conflict entry names are persisted in the file and may be selected by
    // CLI automation. Planner hash-table order must not renumber them.
    conflicts.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| conflict_kind_key(&left.kind).cmp(&conflict_kind_key(&right.kind)))
    });
    stats.conflicted = conflicts.len();

    // ---- Apply survivors: ours first, then theirs (targets are disjoint now)
    stats.ours_applied += conflicted_ours
        .iter()
        .filter(|excluded| !**excluded)
        .count();
    let theirs_excluded: Vec<bool> = conflicted_theirs
        .iter()
        .zip(&dropped_theirs)
        .map(|(conflicted, dropped)| *conflicted || *dropped)
        .collect();
    stats.theirs_applied += theirs_excluded
        .iter()
        .filter(|excluded| !**excluded)
        .count();

    let mut explorer_trees = (capture_explorer || !conflicts.is_empty()).then(|| {
        ExplorerTrees::capture(
            base,
            ours_dom,
            theirs_dom,
            &ours.identity.matched,
            &theirs.identity.matched,
        )
    });

    let ours_created =
        apply_ops_filtered(base, ours_dom, &ours.ops, &ours.identity, &conflicted_ours);
    let theirs_created = apply_ops_filtered(
        base,
        theirs_dom,
        &theirs.ops,
        &theirs.identity,
        &theirs_excluded,
    );
    if let Some(explorer_trees) = &mut explorer_trees {
        explorer_trees.bind_result(base, &ours_created, &theirs_created);
    }

    info!(
        ours_applied = stats.ours_applied,
        theirs_applied = stats.theirs_applied,
        deduped = stats.deduped,
        conflicts = stats.conflicted,
        "merge complete"
    );

    MergeResult {
        conflicts,
        stats,
        pivots,
        ours_identity: ours.identity.clone(),
        theirs_identity: theirs.identity.clone(),
        explorer_trees,
    }
}

fn merge_pivots(
    base: &WeakDom,
    ours: &EditScript,
    theirs: &EditScript,
    ours_removed: &HashSet<Ref>,
    theirs_removed: &HashSet<Ref>,
    evacuated: &HashSet<Ref>,
    conflicted_ours: &mut [bool],
    conflicted_theirs: &mut [bool],
    ours_edits_vs_theirs_delete: &mut HashMap<Ref, usize>,
    theirs_edits_vs_ours_delete: &mut HashMap<Ref, usize>,
    conflicts: &mut Vec<MergeConflict>,
    stats: &mut MergeStats,
) -> Vec<PivotOp> {
    let theirs_by_target: HashMap<Ref, &PivotOp> = theirs
        .pivots
        .iter()
        .map(|pivot| (pivot.target_ref, pivot))
        .collect();
    debug_assert_eq!(theirs_by_target.len(), theirs.pivots.len());

    let mut consumed_theirs = HashSet::new();
    let mut merged = Vec::with_capacity(ours.pivots.len() + theirs.pivots.len());
    for ours_pivot in &ours.pivots {
        if let Some(removed_root) =
            ancestor_in(base, ours_pivot.target_ref, theirs_removed, evacuated)
        {
            if let Some(&conflict_index) = ours_edits_vs_theirs_delete.get(&removed_root) {
                conflicts[conflict_index]
                    .ours
                    .pivots
                    .push(ours_pivot.clone());
            } else {
                let their_op = find_remove(&theirs.ops, removed_root, conflicted_theirs);
                let conflict_index = conflicts.len();
                conflicts.push(MergeConflict {
                    kind: ConflictKind::DeleteVsEdit,
                    base_ref: removed_root,
                    path: get_instance_path(base, removed_root),
                    ours: ConflictSide::pivots(vec![ours_pivot.clone()]),
                    theirs: ConflictSide::edits(their_op),
                });
                ours_edits_vs_theirs_delete.insert(removed_root, conflict_index);
            }
            continue;
        }
        let Some(theirs_pivot) = theirs_by_target.get(&ours_pivot.target_ref) else {
            merged.push(ours_pivot.clone());
            stats.ours_applied += 1;
            continue;
        };
        consumed_theirs.insert(ours_pivot.target_ref);
        debug_assert_eq!(ours_pivot.order, theirs_pivot.order);
        debug_assert_eq!(ours_pivot.parent_order, theirs_pivot.parent_order);
        if pivot_deltas_close(&ours_pivot.delta, &theirs_pivot.delta) {
            merged.push(ours_pivot.clone());
            stats.ours_applied += 1;
            stats.deduped += 1;
        } else {
            conflicts.push(MergeConflict {
                kind: ConflictKind::Pivot {
                    ours: ours_pivot.delta,
                    theirs: theirs_pivot.delta,
                    order: ours_pivot.order,
                    parent_order: ours_pivot.parent_order,
                },
                base_ref: ours_pivot.target_ref,
                path: get_instance_path(base, ours_pivot.target_ref),
                ours: ConflictSide::pivots(vec![ours_pivot.clone()]),
                theirs: ConflictSide::pivots(vec![(*theirs_pivot).clone()]),
            });
        }
    }
    for theirs_pivot in &theirs.pivots {
        if consumed_theirs.contains(&theirs_pivot.target_ref) {
            continue;
        }
        if let Some(removed_root) =
            ancestor_in(base, theirs_pivot.target_ref, ours_removed, evacuated)
        {
            if let Some(&conflict_index) = theirs_edits_vs_ours_delete.get(&removed_root) {
                conflicts[conflict_index]
                    .theirs
                    .pivots
                    .push(theirs_pivot.clone());
            } else {
                let our_op = find_remove(&ours.ops, removed_root, conflicted_ours);
                let conflict_index = conflicts.len();
                conflicts.push(MergeConflict {
                    kind: ConflictKind::DeleteVsEdit,
                    base_ref: removed_root,
                    path: get_instance_path(base, removed_root),
                    ours: ConflictSide::edits(our_op),
                    theirs: ConflictSide::pivots(vec![theirs_pivot.clone()]),
                });
                theirs_edits_vs_ours_delete.insert(removed_root, conflict_index);
            }
            continue;
        }
        merged.push(theirs_pivot.clone());
        stats.theirs_applied += 1;
    }
    merged.sort_unstable_by_key(|pivot| pivot.order);
    merged
}

fn conflict_kind_key(kind: &ConflictKind) -> (u8, &str) {
    match kind {
        ConflictKind::Property { name } => (0, name),
        ConflictKind::PropertyBundle { name, .. } => (1, name),
        ConflictKind::DeleteVsEdit => (2, ""),
        ConflictKind::MoveTarget => (3, ""),
        ConflictKind::Pivot { .. } => (4, ""),
    }
}

#[allow(clippy::too_many_arguments)]
fn group_property_bundle_conflicts(
    base: &WeakDom,
    ours_dom: &dyn DomView,
    theirs_dom: &dyn DomView,
    ours: &EditScript,
    theirs: &EditScript,
    conflicted_ours: &mut [bool],
    conflicted_theirs: &mut [bool],
    dropped_theirs: &mut [bool],
    conflicts: &mut Vec<MergeConflict>,
    stats: &mut MergeStats,
) {
    type BundleKey = (Ref, &'static str);
    type BundleOps = (SemanticPropertyBundle, Vec<usize>);

    let collect = |ops: &[EditOp], excluded: &[bool]| {
        let mut groups: HashMap<BundleKey, BundleOps> = HashMap::new();
        for (index, op) in ops.iter().enumerate() {
            if excluded[index] {
                continue;
            }
            let EditOp::SetProperty { old_ref, name, .. } = op else {
                continue;
            };
            let Some(instance) = base.get_by_ref(*old_ref) else {
                continue;
            };
            let Some(bundle) = semantic_property_bundle(instance.class.as_str(), name) else {
                continue;
            };
            groups
                .entry((*old_ref, bundle.name))
                .or_insert_with(|| (bundle, Vec::new()))
                .1
                .push(index);
        }
        groups
    };

    let ours_groups = collect(&ours.ops, conflicted_ours);
    let theirs_groups = collect(&theirs.ops, conflicted_theirs);
    let mut common: Vec<_> = ours_groups
        .keys()
        .filter(|key| theirs_groups.contains_key(key))
        .copied()
        .collect();
    common.sort_by(|(a_ref, a_name), (b_ref, b_name)| {
        get_instance_path(base, *a_ref)
            .cmp(&get_instance_path(base, *b_ref))
            .then_with(|| a_name.cmp(b_name))
    });

    for key in common {
        let (bundle, our_indices) = &ours_groups[&key];
        let (_, their_indices) = &theirs_groups[&key];
        let Some(our_ref) = ours.identity.matched.get(&key.0) else {
            continue;
        };
        let Some(their_ref) = theirs.identity.matched.get(&key.0) else {
            continue;
        };
        let (Some(our_instance), Some(their_instance)) = (
            ours_dom.get_by_ref(*our_ref),
            theirs_dom.get_by_ref(*their_ref),
        ) else {
            continue;
        };

        if semantic_bundle_values_equal(&our_instance, &their_instance, *bundle) {
            for &index in their_indices {
                dropped_theirs[index] = true;
            }
            stats.deduped += their_indices.len();
            continue;
        }

        for &index in our_indices {
            conflicted_ours[index] = true;
        }
        for &index in their_indices {
            conflicted_theirs[index] = true;
        }
        conflicts.push(MergeConflict {
            kind: ConflictKind::PropertyBundle {
                name: bundle.name.to_string(),
                properties: bundle
                    .properties
                    .iter()
                    .map(|property| (*property).to_string())
                    .collect(),
            },
            base_ref: key.0,
            path: get_instance_path(base, key.0),
            ours: ConflictSide::edits(
                our_indices
                    .iter()
                    .map(|&index| ours.ops[index].clone())
                    .collect(),
            ),
            theirs: ConflictSide::edits(
                their_indices
                    .iter()
                    .map(|&index| theirs.ops[index].clone())
                    .collect(),
            ),
        });
    }
}

/// The base-DOM instances an op touches: the primary target, plus the
/// destination parent for moves and adds (a move into a subtree the other
/// side removed is just as conflicting as an edit inside it).
fn op_primary_target(op: &EditOp) -> Option<Ref> {
    match op {
        EditOp::RemoveSubtree { old_ref }
        | EditOp::Move { old_ref, .. }
        | EditOp::SetName { old_ref, .. }
        | EditOp::SetProperty { old_ref, .. } => Some(*old_ref),
        EditOp::AddSubtree { .. } => None,
    }
}

fn op_base_targets(op: &EditOp) -> [Option<Ref>; 2] {
    match op {
        EditOp::RemoveSubtree { old_ref }
        | EditOp::SetName { old_ref, .. }
        | EditOp::SetProperty { old_ref, .. } => [Some(*old_ref), None],
        EditOp::Move {
            old_ref,
            new_parent,
        } => match new_parent {
            Anchor::Old(parent) => [Some(*old_ref), Some(*parent)],
            Anchor::Added(_) => [Some(*old_ref), None],
        },
        EditOp::AddSubtree {
            parent: Anchor::Old(parent),
            ..
        } => [Some(*parent), None],
        EditOp::AddSubtree { .. } => [None, None],
    }
}

/// Move destinations by moved instance, for cross-branch identical-move checks.
fn move_anchors(ops: &[EditOp]) -> HashMap<Ref, Anchor> {
    ops.iter()
        .filter_map(|op| match op {
            EditOp::Move {
                old_ref,
                new_parent,
            } => Some((*old_ref, *new_parent)),
            _ => None,
        })
        .collect()
}

fn removed_roots(ops: &[EditOp]) -> HashSet<Ref> {
    ops.iter()
        .filter_map(|op| match op {
            EditOp::RemoveSubtree { old_ref } => Some(*old_ref),
            _ => None,
        })
        .collect()
}

/// Walk up from `target` in the base DOM; return the first ancestor (or the
/// target itself) present in `roots`. The walk stops at symmetrically
/// evacuated instances first: content both branches moved to the same live
/// destination is not "inside" any removed subtree it started under.
fn ancestor_in(
    base: &WeakDom,
    target: Ref,
    roots: &HashSet<Ref>,
    evacuated: &HashSet<Ref>,
) -> Option<Ref> {
    let mut current = target;
    while let Some(inst) = base.get_by_ref(current) {
        if evacuated.contains(&current) {
            return None;
        }
        if roots.contains(&current) {
            return Some(current);
        }
        current = inst.parent();
    }
    None
}

fn find_remove(ops: &[EditOp], root: Ref, conflicted: &mut [bool]) -> Vec<EditOp> {
    let mut found = None;
    for (i, op) in ops.iter().enumerate() {
        if matches!(op, EditOp::RemoveSubtree { old_ref } if *old_ref == root) {
            conflicted[i] = true;
            found.get_or_insert_with(|| op.clone());
        }
    }
    found.into_iter().collect()
}

/// Join both branches' additions by (parent, deep hash): identical content
/// added under the same base parent is one logical addition made twice.
/// Returns the theirs→ours per-instance equivalence across every paired
/// subtree plus the (ours op index, theirs op index) pairs themselves.
/// Hash-indexed FIFO queues keep the join one-to-one and positional.
fn pair_identical_adds(
    ours_dom: &dyn DomView,
    theirs_dom: &dyn DomView,
    ours_ops: &[EditOp],
    theirs_ops: &[EditOp],
    ours_deep: &DeepHashCache<'_>,
    theirs_deep: &DeepHashCache<'_>,
) -> (HashMap<Ref, Ref>, Vec<(usize, usize)>) {
    let mut added_equiv: HashMap<Ref, Ref> = HashMap::new();
    let mut pairs: Vec<(usize, usize)> = Vec::new();

    let mut theirs_adds: HashMap<(Ref, blake3::Hash), std::collections::VecDeque<usize>> =
        HashMap::new();
    for (index, op) in theirs_ops.iter().enumerate() {
        let EditOp::AddSubtree {
            parent: Anchor::Old(parent),
            new_ref,
        } = op
        else {
            continue;
        };
        theirs_adds
            .entry((*parent, theirs_deep.get(*new_ref)))
            .or_default()
            .push_back(index);
    }
    for (i, our_op) in ours_ops.iter().enumerate() {
        let EditOp::AddSubtree {
            parent: Anchor::Old(our_parent),
            new_ref: our_new,
        } = our_op
        else {
            continue;
        };
        let key = (*our_parent, ours_deep.get(*our_new));
        let Some(their_indices) = theirs_adds.get_mut(&key) else {
            continue;
        };
        let Some(j) = their_indices.pop_front() else {
            continue;
        };
        let EditOp::AddSubtree {
            new_ref: their_new, ..
        } = &theirs_ops[j]
        else {
            unreachable!("theirs_adds only indexes AddSubtree ops");
        };
        record_added_equivalence(
            ours_dom,
            theirs_dom,
            *our_new,
            *their_new,
            ours_deep,
            theirs_deep,
            &mut added_equiv,
        );
        pairs.push((i, j));
    }
    (added_equiv, pairs)
}

/// Walk two deduplicated added subtrees in parallel, recording
/// theirs-new-ref → ours-new-ref for every corresponding instance. The
/// subtrees have equal deep hashes, so children pair one-to-one by hash;
/// identical siblings pair positionally (both sides walk document order).
fn record_added_equivalence(
    ours_dom: &dyn DomView,
    theirs_dom: &dyn DomView,
    ours_ref: Ref,
    theirs_ref: Ref,
    ours_deep: &DeepHashCache<'_>,
    theirs_deep: &DeepHashCache<'_>,
    added_equiv: &mut HashMap<Ref, Ref>,
) {
    added_equiv.insert(theirs_ref, ours_ref);
    let (Some(ours_inst), Some(theirs_inst)) = (
        ours_dom.get_by_ref(ours_ref),
        theirs_dom.get_by_ref(theirs_ref),
    ) else {
        return;
    };
    let mut ours_children: Vec<(Ref, blake3::Hash, bool)> = ours_inst
        .children()
        .map(|child| (child, ours_deep.get(child), false))
        .collect();
    for theirs_child in theirs_inst.children() {
        let hash = theirs_deep.get(theirs_child);
        let Some((ours_child, _, consumed)) = ours_children
            .iter_mut()
            .find(|(_, ours_hash, consumed)| !consumed && *ours_hash == hash)
        else {
            continue;
        };
        *consumed = true;
        record_added_equivalence(
            ours_dom,
            theirs_dom,
            *ours_child,
            theirs_child,
            ours_deep,
            theirs_deep,
            added_equiv,
        );
    }
}

/// Cross-branch value equality. Ref values compare through the base identity
/// (same logical target), or through the deduplicated-add equivalence when
/// both branches point into their own copy of identical added content;
/// anything unmappable is conservatively unequal.
fn values_equal(
    a: &Option<Variant>,
    b: &Option<Variant>,
    ours_to_base: &HashMap<Ref, Ref>,
    theirs_to_base: &HashMap<Ref, Ref>,
    added_equiv: &HashMap<Ref, Ref>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(va), Some(vb))
            if crate::reference_value::direct_reference(va).is_some()
                || crate::reference_value::direct_reference(vb).is_some() =>
        {
            let (Some((kind_a, ra)), Some((kind_b, rb))) = (
                crate::reference_value::direct_reference(va),
                crate::reference_value::direct_reference(vb),
            ) else {
                return false;
            };
            if kind_a != kind_b {
                return false;
            }
            if ra.is_none() && rb.is_none() {
                return true;
            }
            match (ours_to_base.get(&ra), theirs_to_base.get(&rb)) {
                (Some(ba), Some(bb)) => ba == bb,
                (None, None) => added_equiv.get(&rb) == Some(&ra),
                _ => false,
            }
        }
        (Some(va), Some(vb)) => crate::value_compare::non_ref_variants_equal(va, vb),
        _ => false,
    }
}

/// Move destinations compare equal when both map to the same base instance,
/// or when both are corresponding positions inside deduplicated added
/// subtrees (each branch moved the target into its own identical copy).
fn anchors_equal(a: Anchor, b: Anchor, added_equiv: &HashMap<Ref, Ref>) -> bool {
    match (a, b) {
        (Anchor::Old(ra), Anchor::Old(rb)) => ra == rb,
        (Anchor::Added(ra), Anchor::Added(rb)) => added_equiv.get(&rb) == Some(&ra),
        _ => false,
    }
}
