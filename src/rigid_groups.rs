//! Rigid-transform conflict grouping: many per-part CFrame conflicts that
//! share one world-space delta per side are one decision, not hundreds.
//!
//! A Studio drag of a selection writes `T * cframe` into every selected part
//! — the serialized file flattens that single edit into N absolute CFrames.
//! This module recovers the factorization. A Model's WorldPivotData is an
//! ownership boundary, but not a transform anchor: Studio can change a model
//! pivot without moving its descendants. The dominant shared delta across a
//! model's BaseParts identifies the actual content move, and the model-pivot
//! conflict joins that decision even when its own delta is unrelated.
//! BasePart conflicts without a model-pivot owner still cluster by their world
//! delta, which covers loose selections.
//! Other properties named CFrame (Attachment, Pose, constraints, etc.) are
//! not world placements and are deliberately excluded.
//!
//! Detection is inference; application never is — resolving a group fans out
//! to its members' exact stored branch values, so no transform math ever
//! touches the merged result.

use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::{CFrame, Matrix3, Variant, Vector3};
use std::collections::{HashMap, HashSet};

use crate::edit_script::EditOp;
use crate::match_instances::get_instance_path;
use crate::merge::{ConflictKind, MergeConflict};

/// Absolute position tolerance (studs) and rotation-component tolerance for
/// two deltas to count as the same rigid transform. Inputs are f32; deltas
/// are computed in f64, so slack only needs to cover f32 quantization.
const POSITION_EPSILON: f64 = 5e-3;
const ROTATION_EPSILON: f64 = 1e-4;

/// Minimum members for a cluster to be worth reporting as a group.
const MIN_GROUP_SIZE: usize = 2;

#[derive(Debug)]
pub struct RigidGroup {
    /// Indices into the merge result's conflict list.
    pub members: Vec<usize>,
    /// Lowest common ancestor of the members' contested instances.
    pub lca: Ref,
    pub path: String,
    pub delta_ours: CFrame,
    pub delta_theirs: CFrame,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SpatialKind {
    ModelPivot,
    BasePart,
}

struct SpatialConflict {
    conflict_index: usize,
    base_ref: Ref,
    kind: SpatialKind,
    delta_ours: Rigid,
    delta_theirs: Rigid,
}

// f64 rigid transform: rows of the rotation matrix + position.
#[derive(Clone, Copy)]
pub(crate) struct Rigid {
    r: [[f64; 3]; 3],
    p: [f64; 3],
}

impl Rigid {
    pub(crate) fn from_cframe(cf: &CFrame) -> Self {
        let row = |v: &Vector3| [v.x as f64, v.y as f64, v.z as f64];
        Rigid {
            r: [&cf.orientation.x, &cf.orientation.y, &cf.orientation.z].map(row),
            p: [
                cf.position.x as f64,
                cf.position.y as f64,
                cf.position.z as f64,
            ],
        }
    }

    pub(crate) fn to_cframe(self) -> CFrame {
        let v = |row: [f64; 3]| Vector3::new(row[0] as f32, row[1] as f32, row[2] as f32);
        CFrame::new(
            v(self.p),
            Matrix3::new(v(self.r[0]), v(self.r[1]), v(self.r[2])),
        )
    }

    pub(crate) fn mul(self, other: Rigid) -> Rigid {
        let mut r = [[0.0; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..3).map(|k| self.r[i][k] * other.r[k][j]).sum();
            }
        }
        let rotate = |m: &[[f64; 3]; 3], v: [f64; 3]| {
            [0, 1, 2].map(|i| m[i][0] * v[0] + m[i][1] * v[1] + m[i][2] * v[2])
        };
        let rp = rotate(&self.r, other.p);
        Rigid {
            r,
            p: [self.p[0] + rp[0], self.p[1] + rp[1], self.p[2] + rp[2]],
        }
    }

    pub(crate) fn inverse(self) -> Rigid {
        let rt = [
            [self.r[0][0], self.r[1][0], self.r[2][0]],
            [self.r[0][1], self.r[1][1], self.r[2][1]],
            [self.r[0][2], self.r[1][2], self.r[2][2]],
        ];
        let p = [0, 1, 2]
            .map(|i| -(rt[i][0] * self.p[0] + rt[i][1] * self.p[1] + rt[i][2] * self.p[2]));
        Rigid { r: rt, p }
    }

    /// `new * base⁻¹` — the world transform taking base placement to new.
    pub(crate) fn delta(new: Rigid, base: Rigid) -> Rigid {
        new.mul(base.inverse())
    }

    pub(crate) fn close(a: &Rigid, b: &Rigid) -> bool {
        for i in 0..3 {
            if (a.p[i] - b.p[i]).abs() > POSITION_EPSILON {
                return false;
            }
            for j in 0..3 {
                if (a.r[i][j] - b.r[i][j]).abs() > ROTATION_EPSILON {
                    return false;
                }
            }
        }
        true
    }
}

/// The CFrame payload of a spatial property value, if it has one.
fn spatial_cframe(value: &Variant) -> Option<&CFrame> {
    match value {
        Variant::CFrame(cf) => Some(cf),
        Variant::OptionalCFrame(Some(cf)) => Some(cf),
        _ => None,
    }
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

fn spatial_kind(class_name: &str, property_name: &str) -> Option<SpatialKind> {
    match property_name {
        "CFrame" if class_is_a(class_name, "BasePart") => Some(SpatialKind::BasePart),
        "WorldPivotData"
            if class_is_a(class_name, "Model") && !class_is_a(class_name, "WorldRoot") =>
        {
            Some(SpatialKind::ModelPivot)
        }
        _ => None,
    }
}

/// The (base, side) CFrames for a single-property conflict, when it is a
/// spatial property readable on all three.
fn spatial_conflict(
    base: &WeakDom,
    conflict_index: usize,
    conflict: &MergeConflict,
) -> Option<SpatialConflict> {
    let ConflictKind::Property { name } = &conflict.kind else {
        return None;
    };
    let base_instance = base.get_by_ref(conflict.base_ref)?;
    let kind = spatial_kind(base_instance.class.as_str(), name)?;
    let side_value = |ops: &[EditOp]| match ops {
        [EditOp::SetProperty {
            value: Some(value), ..
        }] => spatial_cframe(value).map(Rigid::from_cframe),
        _ => None,
    };
    let ours = side_value(&conflict.ours)?;
    let theirs = side_value(&conflict.theirs)?;
    let base_value = base_instance
        .properties
        .get(&name.as_str().into())
        .and_then(spatial_cframe)
        .map(Rigid::from_cframe)?;
    Some(SpatialConflict {
        conflict_index,
        base_ref: conflict.base_ref,
        kind,
        delta_ours: Rigid::delta(ours, base_value),
        delta_theirs: Rigid::delta(theirs, base_value),
    })
}

fn same_deltas(a: &SpatialConflict, b: &SpatialConflict) -> bool {
    Rigid::close(&a.delta_ours, &b.delta_ours) && Rigid::close(&a.delta_theirs, &b.delta_theirs)
}

fn ancestors(dom: &WeakDom, mut r: Ref) -> Vec<Ref> {
    let mut chain = vec![r];
    while let Some(inst) = dom.get_by_ref(r) {
        let parent = inst.parent();
        if parent.is_none() {
            break;
        }
        chain.push(parent);
        r = parent;
    }
    chain
}

fn lowest_common_ancestor(dom: &WeakDom, refs: &[Ref]) -> Ref {
    let mut lca = refs[0];
    for &other in &refs[1..] {
        let chain: HashSet<Ref> = ancestors(dom, lca).into_iter().collect();
        let mut node = other;
        loop {
            if chain.contains(&node) {
                lca = node;
                break;
            }
            match dom.get_by_ref(node).map(|i| i.parent()) {
                Some(parent) if !parent.is_none() => node = parent,
                _ => break,
            }
        }
    }
    lca
}

fn is_descendant_or_same(dom: &WeakDom, mut node: Ref, ancestor: Ref) -> bool {
    loop {
        if node == ancestor {
            return true;
        }
        let Some(instance) = dom.get_by_ref(node) else {
            return false;
        };
        let parent = instance.parent();
        if parent.is_none() {
            return false;
        }
        node = parent;
    }
}

fn build_group(
    base: &WeakDom,
    conflicts: &[MergeConflict],
    mut members: Vec<usize>,
    delta_ours: Rigid,
    delta_theirs: Rigid,
) -> RigidGroup {
    members.sort_unstable();
    let refs: Vec<Ref> = members.iter().map(|&i| conflicts[i].base_ref).collect();
    let lca = lowest_common_ancestor(base, &refs);
    RigidGroup {
        members,
        lca,
        path: get_instance_path(base, lca),
        delta_ours: delta_ours.to_cframe(),
        delta_theirs: delta_theirs.to_cframe(),
    }
}

/// Recover rigid moves from BasePart consensus, using Model pivots only to
/// establish ownership boundaries. `base` is the merged DOM — conflicted
/// targets keep base content, so their properties still hold base values here.
pub fn detect_rigid_groups(base: &WeakDom, conflicts: &[MergeConflict]) -> Vec<RigidGroup> {
    let spatial: Vec<SpatialConflict> = conflicts
        .iter()
        .enumerate()
        .filter_map(|(index, conflict)| spatial_conflict(base, index, conflict))
        .collect();

    let pivot_indices: Vec<usize> = spatial
        .iter()
        .enumerate()
        .filter_map(|(index, conflict)| (conflict.kind == SpatialKind::ModelPivot).then_some(index))
        .collect();

    let mut claimed: HashSet<usize> = HashSet::new();
    let mut groups = Vec::new();

    // Preserve the strongest case first: a pivot and descendants that all
    // carry the same delta. This is exact evidence of one PivotTo-style move.
    let exact_roots: Vec<usize> = pivot_indices
        .iter()
        .copied()
        .filter(|&pivot_index| {
            let pivot = &spatial[pivot_index];
            !pivot_indices.iter().copied().any(|other_index| {
                let other = &spatial[other_index];
                other.base_ref != pivot.base_ref
                    && same_deltas(other, pivot)
                    && is_descendant_or_same(base, pivot.base_ref, other.base_ref)
            })
        })
        .collect();

    for pivot_index in exact_roots {
        let pivot = &spatial[pivot_index];
        let members: Vec<usize> = spatial
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| {
                (same_deltas(pivot, candidate)
                    && is_descendant_or_same(base, candidate.base_ref, pivot.base_ref))
                .then_some(index)
            })
            .collect();
        if members.len() < MIN_GROUP_SIZE {
            continue;
        }
        claimed.extend(members.iter().copied());
        groups.push(build_group(
            base,
            conflicts,
            members
                .into_iter()
                .map(|index| spatial[index].conflict_index)
                .collect(),
            pivot.delta_ours,
            pivot.delta_theirs,
        ));
    }

    struct PartCluster {
        delta_ours: Rigid,
        delta_theirs: Rigid,
        /// Indices into `spatial`, not the merge-conflict list.
        members: Vec<usize>,
    }
    let mut clusters: Vec<PartCluster> = Vec::new();

    for (index, conflict) in spatial.iter().enumerate() {
        if claimed.contains(&index) || conflict.kind != SpatialKind::BasePart {
            continue;
        }
        let existing = clusters.iter_mut().find(|c| {
            Rigid::close(&c.delta_ours, &conflict.delta_ours)
                && Rigid::close(&c.delta_theirs, &conflict.delta_theirs)
        });
        match existing {
            Some(cluster) => cluster.members.push(index),
            None => clusters.push(PartCluster {
                delta_ours: conflict.delta_ours,
                delta_theirs: conflict.delta_theirs,
                members: vec![index],
            }),
        }
    }

    // Map each pivot to the unique largest rigid cluster among its descendant
    // parts. Requiring at least two agreeing parts prevents an arbitrary pivot
    // from blessing a one-part transform as a model move.
    let mut pivot_cluster: HashMap<usize, usize> = HashMap::new();
    for &pivot_index in &pivot_indices {
        if claimed.contains(&pivot_index) {
            continue;
        }
        let pivot_ref = spatial[pivot_index].base_ref;
        let mut counts: Vec<(usize, usize)> = clusters
            .iter()
            .enumerate()
            .map(|(cluster_index, cluster)| {
                let count = cluster
                    .members
                    .iter()
                    .filter(|&&member_index| {
                        is_descendant_or_same(base, spatial[member_index].base_ref, pivot_ref)
                    })
                    .count();
                (cluster_index, count)
            })
            .filter(|(_, count)| *count >= MIN_GROUP_SIZE)
            .collect();
        counts.sort_unstable_by_key(|(_, count)| std::cmp::Reverse(*count));
        if let Some(&(best_cluster, best_count)) = counts.first() {
            let tied = counts.get(1).is_some_and(|(_, count)| *count == best_count);
            if !tied {
                pivot_cluster.insert(pivot_index, best_cluster);
            }
        }
    }

    for (cluster_index, cluster) in clusters.iter().enumerate() {
        // Outermost pivots assigned to this cluster own separate model groups.
        // This keeps two sibling models separate even if both were moved by
        // exactly the same transform.
        let owners: Vec<usize> = pivot_indices
            .iter()
            .copied()
            .filter(|pivot_index| pivot_cluster.get(pivot_index) == Some(&cluster_index))
            .filter(|&pivot_index| {
                let pivot_ref = spatial[pivot_index].base_ref;
                !pivot_indices.iter().copied().any(|ancestor_index| {
                    pivot_cluster.get(&ancestor_index) == Some(&cluster_index)
                        && spatial[ancestor_index].base_ref != pivot_ref
                        && is_descendant_or_same(base, pivot_ref, spatial[ancestor_index].base_ref)
                })
            })
            .collect();

        for owner_index in owners {
            let owner_ref = spatial[owner_index].base_ref;
            let mut members: Vec<usize> = cluster
                .members
                .iter()
                .copied()
                .filter(|&member_index| {
                    is_descendant_or_same(base, spatial[member_index].base_ref, owner_ref)
                })
                .collect();

            // Nested model pivots join the outer decision when their own
            // descendant consensus identifies the same content move. Their
            // individual pivot deltas are intentionally irrelevant.
            members.extend(pivot_indices.iter().copied().filter(|pivot_index| {
                pivot_cluster.get(pivot_index) == Some(&cluster_index)
                    && is_descendant_or_same(base, spatial[*pivot_index].base_ref, owner_ref)
            }));
            members.sort_unstable();
            members.dedup();

            let part_count = members
                .iter()
                .filter(|&&index| spatial[index].kind == SpatialKind::BasePart)
                .count();
            if part_count < MIN_GROUP_SIZE {
                continue;
            }

            claimed.extend(members.iter().copied());
            groups.push(build_group(
                base,
                conflicts,
                members
                    .into_iter()
                    .map(|index| spatial[index].conflict_index)
                    .collect(),
                cluster.delta_ours,
                cluster.delta_theirs,
            ));
        }
    }

    // No usable WorldPivotData ownership boundary is available for these
    // parts, so retain world-delta clustering for loose selections.
    for cluster in clusters {
        let members: Vec<usize> = cluster
            .members
            .into_iter()
            .filter(|index| !claimed.contains(index))
            .map(|index| spatial[index].conflict_index)
            .collect();
        if members.len() >= MIN_GROUP_SIZE {
            groups.push(build_group(
                base,
                conflicts,
                members,
                cluster.delta_ours,
                cluster.delta_theirs,
            ));
        }
    }

    groups.sort_by_key(|group| group.members[0]);
    groups
}
