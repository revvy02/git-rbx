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
    apply_ops_filtered, compute_edit_script, compute_semantic_changes_with_identity, Anchor,
    EditOp, EditScript, InstanceIdentity,
};
use crate::explorer_tree::ExplorerTrees;
use crate::hash::DeepHashCache;
use crate::match_instances::get_instance_path;
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
    /// Both branches placed an otherwise canonical model asset in different
    /// world frames. The deltas take canonical/base content to each side.
    ModelFrame {
        ours: CFrame,
        theirs: CFrame,
        /// Stable top-down order among hierarchical frame boundaries.
        order: usize,
        /// Nearest ancestor that also has a frame decision.
        parent_order: Option<usize>,
    },
}

#[derive(Debug)]
pub struct MergeConflict {
    pub kind: ConflictKind,
    /// Base-DOM ref of the contested instance (subtree root for DeleteVsEdit).
    pub base_ref: Ref,
    /// Path of the contested instance in the base DOM.
    pub path: String,
    pub ours: Vec<EditOp>,
    pub theirs: Vec<EditOp>,
}

#[derive(Debug, Default)]
pub struct MergeStats {
    pub ours_applied: usize,
    pub theirs_applied: usize,
    pub deduped: usize,
    pub conflicted: usize,
}

pub struct MergeResult {
    pub conflicts: Vec<MergeConflict>,
    pub stats: MergeStats,
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
    let ours_script = compute_edit_script(base, ours, config);
    let theirs_script = compute_edit_script(base, theirs, config);
    merge_scripts(
        base,
        ours,
        theirs,
        &ours_script,
        &theirs_script,
        config,
        false,
    )
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
    let ours_script = compute_semantic_changes_with_identity(base, ours, config, None);
    let theirs_script = compute_semantic_changes_with_identity(base, theirs, config, None);
    merge_scripts(
        base,
        ours,
        theirs,
        &ours_script,
        &theirs_script,
        config,
        false,
    )
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
    let ours_script =
        compute_semantic_changes_with_identity(base, ours, config, Some(ours_identity));
    let theirs_script =
        compute_semantic_changes_with_identity(base, theirs, config, Some(theirs_identity));
    merge_scripts(
        base,
        ours,
        theirs,
        &ours_script,
        &theirs_script,
        config,
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
    let ours_script =
        compute_semantic_changes_with_identity(base, ours, config, Some(ours_identity));
    let theirs_script =
        compute_semantic_changes_with_identity(base, theirs, config, Some(theirs_identity));
    merge_scripts(
        base,
        ours,
        theirs,
        &ours_script,
        &theirs_script,
        config,
        true,
    )
}

fn merge_scripts(
    base: &mut WeakDom,
    ours_dom: &dyn DomView,
    theirs_dom: &dyn DomView,
    ours: &EditScript,
    theirs: &EditScript,
    config: &DiffConfig,
    capture_explorer: bool,
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

    let ours_deep = DeepHashCache::new(ours_dom, &config.ignore_properties);
    let theirs_deep = DeepHashCache::new(theirs_dom, &config.ignore_properties);

    // ---- Delete-vs-edit: an op on one side targeting inside the other side's
    // removed subtree (removes of removes compose instead)
    for (i, op) in ours.ops.iter().enumerate() {
        for target in op_base_targets(op).into_iter().flatten() {
            let Some(removed_root) = ancestor_in(base, target, &theirs_removed) else {
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
                conflicts[conflict_index].ours.push(op.clone());
            } else {
                let their_op = find_remove(&theirs.ops, removed_root, &mut conflicted_theirs);
                let conflict_index = conflicts.len();
                conflicts.push(MergeConflict {
                    kind: ConflictKind::DeleteVsEdit,
                    base_ref: removed_root,
                    path: get_instance_path(base, removed_root),
                    ours: vec![op.clone()],
                    theirs: their_op,
                });
                ours_edits_vs_theirs_delete.insert(removed_root, conflict_index);
            }
            break;
        }
    }
    for (i, op) in theirs.ops.iter().enumerate() {
        for target in op_base_targets(op).into_iter().flatten() {
            let Some(removed_root) = ancestor_in(base, target, &ours_removed) else {
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
                conflicts[conflict_index].theirs.push(op.clone());
            } else {
                let our_op = find_remove(&ours.ops, removed_root, &mut conflicted_ours);
                let conflict_index = conflicts.len();
                conflicts.push(MergeConflict {
                    kind: ConflictKind::DeleteVsEdit,
                    base_ref: removed_root,
                    path: get_instance_path(base, removed_root),
                    ours: our_op,
                    theirs: vec![op.clone()],
                });
                theirs_edits_vs_ours_delete.insert(removed_root, conflict_index);
            }
            break;
        }
    }

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
                            ours: vec![our_op.clone()],
                            theirs: vec![their_op.clone()],
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
                            ours: vec![our_op.clone()],
                            theirs: vec![their_op.clone()],
                        });
                    }
                }
                (EditOp::RemoveSubtree { old_ref: a }, EditOp::RemoveSubtree { old_ref: b })
                    if a == b =>
                {
                    dropped_theirs[j] = true;
                    stats.deduped += 1;
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
                    if anchors_equal(*ap, *bp) {
                        dropped_theirs[j] = true;
                        stats.deduped += 1;
                    } else {
                        conflicted_ours[i] = true;
                        conflicted_theirs[j] = true;
                        conflicts.push(MergeConflict {
                            kind: ConflictKind::MoveTarget,
                            base_ref: *a,
                            path: get_instance_path(base, *a),
                            ours: vec![our_op.clone()],
                            theirs: vec![their_op.clone()],
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
                        ours: vec![our_op.clone()],
                        theirs: vec![their_op.clone()],
                    });
                }
                _ => {}
            }
        }
    }

    // ---- Both sides added identical content under the same parent: dedupe.
    // Hash-indexed queues make this a one-to-one join instead of a nested
    // scan that could accidentally let one ours addition consume several
    // identical theirs additions.
    let mut theirs_adds: HashMap<(Ref, blake3::Hash), Vec<usize>> = HashMap::new();
    for (index, op) in theirs.ops.iter().enumerate() {
        if conflicted_theirs[index] || dropped_theirs[index] {
            continue;
        }
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
            .push(index);
    }
    for (i, our_op) in ours.ops.iter().enumerate() {
        if conflicted_ours[i] {
            continue;
        }
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
        if let Some(j) = their_indices.pop() {
            dropped_theirs[j] = true;
            stats.deduped += 1;
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
    stats.ours_applied = conflicted_ours.iter().filter(|excluded| !**excluded).count();
    let theirs_excluded: Vec<bool> = conflicted_theirs
        .iter()
        .zip(&dropped_theirs)
        .map(|(conflicted, dropped)| *conflicted || *dropped)
        .collect();
    stats.theirs_applied = theirs_excluded
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

    let ours_created = apply_ops_filtered(
        base,
        ours_dom,
        &ours.ops,
        &ours.identity,
        &conflicted_ours,
    );
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
        ours_identity: ours.identity.clone(),
        theirs_identity: theirs.identity.clone(),
        explorer_trees,
    }
}

fn conflict_kind_key(kind: &ConflictKind) -> (u8, &str) {
    match kind {
        ConflictKind::Property { name } => (0, name),
        ConflictKind::PropertyBundle { name, .. } => (1, name),
        ConflictKind::DeleteVsEdit => (2, ""),
        ConflictKind::MoveTarget => (3, ""),
        ConflictKind::ModelFrame { .. } => (4, ""),
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
            ours: our_indices
                .iter()
                .map(|&index| ours.ops[index].clone())
                .collect(),
            theirs: their_indices
                .iter()
                .map(|&index| theirs.ops[index].clone())
                .collect(),
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

fn removed_roots(ops: &[EditOp]) -> HashSet<Ref> {
    ops.iter()
        .filter_map(|op| match op {
            EditOp::RemoveSubtree { old_ref } => Some(*old_ref),
            _ => None,
        })
        .collect()
}

/// Walk up from `target` in the base DOM; return the first ancestor (or the
/// target itself) present in `roots`.
fn ancestor_in(base: &WeakDom, target: Ref, roots: &HashSet<Ref>) -> Option<Ref> {
    let mut current = target;
    while let Some(inst) = base.get_by_ref(current) {
        if roots.contains(&current) {
            return Some(current);
        }
        current = inst.parent();
    }
    None
}

fn find_remove(ops: &[EditOp], root: Ref, conflicted: &mut [bool]) -> Vec<EditOp> {
    for (i, op) in ops.iter().enumerate() {
        if matches!(op, EditOp::RemoveSubtree { old_ref } if *old_ref == root) {
            conflicted[i] = true;
            return vec![op.clone()];
        }
    }
    Vec::new()
}

/// Cross-branch value equality. Ref values compare through the base identity
/// (same logical target); anything unmappable is conservatively unequal.
fn values_equal(
    a: &Option<Variant>,
    b: &Option<Variant>,
    ours_to_base: &HashMap<Ref, Ref>,
    theirs_to_base: &HashMap<Ref, Ref>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(Variant::Ref(ra)), Some(Variant::Ref(rb))) => {
            if ra.is_none() && rb.is_none() {
                return true;
            }
            match (ours_to_base.get(ra), theirs_to_base.get(rb)) {
                (Some(ba), Some(bb)) => ba == bb,
                _ => false,
            }
        }
        (Some(va), Some(vb)) => crate::value_compare::non_ref_variants_equal(va, vb),
        _ => false,
    }
}

/// Move destinations compare equal only when both map to the same base
/// instance. Added-subtree anchors can't be equated across branches.
fn anchors_equal(a: Anchor, b: Anchor) -> bool {
    match (a, b) {
        (Anchor::Old(ra), Anchor::Old(rb)) => ra == rb,
        _ => false,
    }
}
