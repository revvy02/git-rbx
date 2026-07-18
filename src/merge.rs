//! Three-way merge: combine two edit scripts computed against a common base.
//!
//! Both branches' scripts address instances by base-DOM refs, so combining is
//! set logic over op targets: ops touching different targets compose, ops with
//! the identical effect dedupe, ops with different effects on the same target
//! conflict. Conflicted ops are NOT applied — the base content is kept and the
//! conflict is reported for a resolver to decide.

use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::Variant;
use std::collections::{HashMap, HashSet};
use tracing::info;

use crate::diff::DiffConfig;
use crate::edit_script::{apply_ops, compute_edit_script, Anchor, EditOp, EditScript};
use crate::hash::DeepHashCache;
use crate::match_instances::get_instance_path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictKind {
    /// Both sides set the same property (or name) to different values.
    Property { name: String },
    /// One side removed a subtree the other side edited/moved into or out of.
    DeleteVsEdit,
    /// Both sides moved the same instance to different parents, or a move
    /// destination can't be proven equal across branches.
    MoveTarget,
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
            let their_op = find_remove(&theirs.ops, removed_root, &mut conflicted_theirs);
            conflicts.push(MergeConflict {
                kind: ConflictKind::DeleteVsEdit,
                base_ref: removed_root,
                path: get_instance_path(base, removed_root),
                ours: vec![op.clone()],
                theirs: their_op,
            });
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
            let our_op = find_remove(&ours.ops, removed_root, &mut conflicted_ours);
            conflicts.push(MergeConflict {
                kind: ConflictKind::DeleteVsEdit,
                base_ref: removed_root,
                path: get_instance_path(base, removed_root),
                ours: our_op,
                theirs: vec![op.clone()],
            });
            break;
        }
    }

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
                    EditOp::SetProperty { old_ref: a, name: an, value: av },
                    EditOp::SetProperty { old_ref: b, name: bn, value: bv },
                ) if a == b && an == bn => {
                    if values_equal(ours_dom, theirs_dom, av, bv, &ours_to_base, &theirs_to_base) {
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
                    EditOp::SetName { old_ref: a, name: an },
                    EditOp::SetName { old_ref: b, name: bn },
                ) if a == b => {
                    if an == bn {
                        dropped_theirs.insert(j);
                        stats.deduped += 1;
                    } else {
                        conflicted_ours.insert(i);
                        conflicted_theirs.insert(j);
                        conflicts.push(MergeConflict {
                            kind: ConflictKind::Property { name: "Name".to_string() },
                            base_ref: *a,
                            path: get_instance_path(base, *a),
                            ours: vec![our_op.clone()],
                            theirs: vec![their_op.clone()],
                        });
                    }
                }
                (
                    EditOp::RemoveSubtree { old_ref: a },
                    EditOp::RemoveSubtree { old_ref: b },
                ) if a == b => {
                    dropped_theirs.insert(j);
                    stats.deduped += 1;
                }
                (
                    EditOp::Move { old_ref: a, new_parent: ap },
                    EditOp::Move { old_ref: b, new_parent: bp },
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
                (
                    EditOp::Move { old_ref: a, .. },
                    EditOp::RemoveSubtree { old_ref: b },
                )
                | (
                    EditOp::RemoveSubtree { old_ref: b },
                    EditOp::Move { old_ref: a, .. },
                ) if a == b => {
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
        let EditOp::AddSubtree { parent: Anchor::Old(our_parent), new_ref: our_new } = our_op else {
            continue;
        };
        for (j, their_op) in theirs.ops.iter().enumerate() {
            if conflicted_theirs.contains(&j) || dropped_theirs.contains(&j) {
                continue;
            }
            let EditOp::AddSubtree { parent: Anchor::Old(their_parent), new_ref: their_new } = their_op else {
                continue;
            };
            if our_parent == their_parent
                && ours_deep.get(*our_new) == theirs_deep.get(*their_new)
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

    apply_ops(base, ours_dom, &ours_survivors, &ours.matched, &ours.moved_destinations);
    apply_ops(base, theirs_dom, &theirs_survivors, &theirs.matched, &theirs.moved_destinations);

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
        EditOp::Move { old_ref, new_parent } => match new_parent {
            Anchor::Old(parent) => vec![*old_ref, *parent],
            Anchor::Added(_) => vec![*old_ref],
        },
        EditOp::AddSubtree { parent: Anchor::Old(parent), .. } => vec![*parent],
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
    ours_dom: &WeakDom,
    theirs_dom: &WeakDom,
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
        (Some(va), Some(vb)) => {
            let mut ha = blake3::Hasher::new();
            crate::hash::hash_variant(ours_dom, &mut ha, va);
            let mut hb = blake3::Hasher::new();
            crate::hash::hash_variant(theirs_dom, &mut hb, vb);
            ha.finalize() == hb.finalize()
        }
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
