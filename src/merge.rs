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
use crate::edit_script::{
    apply_ops, compute_edit_script, compute_edit_script_with_matches, Anchor, EditOp, EditScript,
    InstanceIdentity,
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
    /// Identity mapping base_ref → ours_ref for every matched instance.
    pub ours_matched: HashMap<Ref, Ref>,
    /// Identity mapping base_ref → theirs_ref for every matched instance.
    pub theirs_matched: HashMap<Ref, Ref>,
    pub(crate) explorer_trees: ExplorerTrees,
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
    merge_scripts(base, ours, theirs, &ours_script, &theirs_script, config)
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
    let ours_script = compute_edit_script_with_matches(base, ours, config, Some(ours_identity));
    let theirs_script =
        compute_edit_script_with_matches(base, theirs, config, Some(theirs_identity));
    merge_scripts(base, ours, theirs, &ours_script, &theirs_script, config)
}

fn merge_scripts(
    base: &mut WeakDom,
    ours_dom: &WeakDom,
    theirs_dom: &WeakDom,
    ours: &EditScript,
    theirs: &EditScript,
    config: &DiffConfig,
) -> MergeResult {
    let mut conflicts: Vec<MergeConflict> = Vec::new();
    let mut stats = MergeStats::default();

    // Reverse identity maps (branch ref → base ref) for cross-branch equality
    let ours_to_base: HashMap<Ref, Ref> = ours.matched.iter().map(|(b, o)| (*o, *b)).collect();
    let theirs_to_base: HashMap<Ref, Ref> = theirs.matched.iter().map(|(b, t)| (*t, *b)).collect();

    let ours_removed: HashSet<Ref> = removed_roots(&ours.ops);
    let theirs_removed: HashSet<Ref> = removed_roots(&theirs.ops);

    // Base refs whose ops conflicted — their ops are withheld on both sides
    let mut conflicted_ours: HashSet<usize> = HashSet::new();
    let mut conflicted_theirs: HashSet<usize> = HashSet::new();
    // Theirs op indices subsumed or deduped by an ours op
    let mut dropped_theirs: HashSet<usize> = HashSet::new();

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
        for target in op_base_targets(op) {
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
                conflicted_ours.insert(i); // withhold; outer remove covers it
                break;
            }
            conflicted_ours.insert(i);
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
        for target in op_base_targets(op) {
            let Some(removed_root) = ancestor_in(base, target, &ours_removed) else {
                continue;
            };
            if let EditOp::RemoveSubtree { old_ref } = op {
                if removed_root == *old_ref {
                    continue;
                }
                stats.deduped += 1;
                dropped_theirs.insert(i);
                break;
            }
            conflicted_theirs.insert(i);
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

    // ---- Same-target op pairs: dedupe identical effects, conflict otherwise
    for (i, our_op) in ours.ops.iter().enumerate() {
        if conflicted_ours.contains(&i) {
            continue;
        }
        for (j, their_op) in theirs.ops.iter().enumerate() {
            if conflicted_theirs.contains(&j) || dropped_theirs.contains(&j) {
                continue;
            }
            match (our_op, their_op) {
                (
                    EditOp::SetProperty {
                        old_ref: a,
                        name: an,
                        value: av,
                    },
                    EditOp::SetProperty {
                        old_ref: b,
                        name: bn,
                        value: bv,
                    },
                ) if a == b && an == bn => {
                    if values_equal(av, bv, &ours_to_base, &theirs_to_base) {
                        dropped_theirs.insert(j);
                        stats.deduped += 1;
                    } else {
                        conflicted_ours.insert(i);
                        conflicted_theirs.insert(j);
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
                        dropped_theirs.insert(j);
                        stats.deduped += 1;
                    } else {
                        conflicted_ours.insert(i);
                        conflicted_theirs.insert(j);
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
                    dropped_theirs.insert(j);
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
                    if anchors_equal(*ap, *bp, &ours_to_base, &theirs_to_base) {
                        dropped_theirs.insert(j);
                        stats.deduped += 1;
                    } else {
                        conflicted_ours.insert(i);
                        conflicted_theirs.insert(j);
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
                    conflicted_ours.insert(i);
                    conflicted_theirs.insert(j);
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

    // ---- Both sides added identical content under the same parent: dedupe
    for (i, our_op) in ours.ops.iter().enumerate() {
        if conflicted_ours.contains(&i) {
            continue;
        }
        let EditOp::AddSubtree {
            parent: Anchor::Old(our_parent),
            new_ref: our_new,
        } = our_op
        else {
            continue;
        };
        for (j, their_op) in theirs.ops.iter().enumerate() {
            if conflicted_theirs.contains(&j) || dropped_theirs.contains(&j) {
                continue;
            }
            let EditOp::AddSubtree {
                parent: Anchor::Old(their_parent),
                new_ref: their_new,
            } = their_op
            else {
                continue;
            };
            if our_parent == their_parent && ours_deep.get(*our_new) == theirs_deep.get(*their_new)
            {
                dropped_theirs.insert(j);
                stats.deduped += 1;
            }
        }
    }

    stats.conflicted = conflicts.len();

    // ---- Apply survivors: ours first, then theirs (targets are disjoint now)
    let ours_survivors: Vec<EditOp> = ours
        .ops
        .iter()
        .enumerate()
        .filter(|(i, _)| !conflicted_ours.contains(i))
        .map(|(_, op)| op.clone())
        .collect();
    let theirs_survivors: Vec<EditOp> = theirs
        .ops
        .iter()
        .enumerate()
        .filter(|(i, _)| !conflicted_theirs.contains(i) && !dropped_theirs.contains(i))
        .map(|(_, op)| op.clone())
        .collect();

    stats.ours_applied = ours_survivors.len();
    stats.theirs_applied = theirs_survivors.len();

    let mut explorer_trees =
        ExplorerTrees::capture(base, ours_dom, theirs_dom, &ours.matched, &theirs.matched);

    let ours_created = apply_ops(
        base,
        ours_dom,
        &ours_survivors,
        &ours.matched,
        &ours.moved_destinations,
    );
    let theirs_created = apply_ops(
        base,
        theirs_dom,
        &theirs_survivors,
        &theirs.matched,
        &theirs.moved_destinations,
    );
    explorer_trees.bind_result(base, &ours_created, &theirs_created);

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
        ours_matched: ours.matched.clone(),
        theirs_matched: theirs.matched.clone(),
        explorer_trees,
    }
}

#[allow(clippy::too_many_arguments)]
fn group_property_bundle_conflicts(
    base: &WeakDom,
    ours_dom: &WeakDom,
    theirs_dom: &WeakDom,
    ours: &EditScript,
    theirs: &EditScript,
    conflicted_ours: &mut HashSet<usize>,
    conflicted_theirs: &mut HashSet<usize>,
    dropped_theirs: &mut HashSet<usize>,
    conflicts: &mut Vec<MergeConflict>,
    stats: &mut MergeStats,
) {
    type BundleKey = (Ref, &'static str);
    type BundleOps = (SemanticPropertyBundle, Vec<usize>);

    let collect = |ops: &[EditOp], excluded: &HashSet<usize>| {
        let mut groups: HashMap<BundleKey, BundleOps> = HashMap::new();
        for (index, op) in ops.iter().enumerate() {
            if excluded.contains(&index) {
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
        let Some(our_ref) = ours.matched.get(&key.0) else {
            continue;
        };
        let Some(their_ref) = theirs.matched.get(&key.0) else {
            continue;
        };
        let (Some(our_instance), Some(their_instance)) = (
            ours_dom.get_by_ref(*our_ref),
            theirs_dom.get_by_ref(*their_ref),
        ) else {
            continue;
        };

        if semantic_bundle_values_equal(our_instance, their_instance, *bundle) {
            for &index in their_indices {
                dropped_theirs.insert(index);
            }
            stats.deduped += their_indices.len();
            continue;
        }

        for &index in our_indices {
            conflicted_ours.insert(index);
        }
        for &index in their_indices {
            conflicted_theirs.insert(index);
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
fn op_base_targets(op: &EditOp) -> Vec<Ref> {
    match op {
        EditOp::RemoveSubtree { old_ref }
        | EditOp::SetName { old_ref, .. }
        | EditOp::SetProperty { old_ref, .. } => vec![*old_ref],
        EditOp::Move {
            old_ref,
            new_parent,
        } => match new_parent {
            Anchor::Old(parent) => vec![*old_ref, *parent],
            Anchor::Added(_) => vec![*old_ref],
        },
        EditOp::AddSubtree {
            parent: Anchor::Old(parent),
            ..
        } => vec![*parent],
        EditOp::AddSubtree { .. } => Vec::new(),
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

fn find_remove(ops: &[EditOp], root: Ref, conflicted: &mut HashSet<usize>) -> Vec<EditOp> {
    for (i, op) in ops.iter().enumerate() {
        if matches!(op, EditOp::RemoveSubtree { old_ref } if *old_ref == root) {
            conflicted.insert(i);
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
fn anchors_equal(
    a: Anchor,
    b: Anchor,
    _ours_to_base: &HashMap<Ref, Ref>,
    _theirs_to_base: &HashMap<Ref, Ref>,
) -> bool {
    match (a, b) {
        (Anchor::Old(ra), Anchor::Old(rb)) => ra == rb,
        _ => false,
    }
}
