//! In-file conflict state: after a conflicted merge, the merged file itself
//! carries the competing versions as real instances (the binary analog of
//! text conflict markers).
//!
//! Schema — a container folder (under ServerStorage for places, the root for
//! models) holding one entry per conflict:
//!
//! ```text
//! __RbxDiffMerge                    Folder, attrs: Version, ConflictCount
//!   Conflict_1                      Folder [tag RbxDiffConflictEntry]
//!                                   attrs: Kind, Path, Property?, Resolved?
//!     Target                        ObjectValue -> live instance (base state)
//!     Ours                          Folder; attrs: Deleted?, DestinationPath?
//!       __RbxDiffImpact             StringValue; exact direct patch for this choice
//!       <clone of our version>      (shallow for property conflicts,
//!                                    full subtree for delete-vs-edit)
//!     Theirs                        Folder; same shape
//!       MoveOuts                    Folder; edited descendants that escape a
//!                                    root both branches ultimately delete
//!         Move_N/Destination        ObjectValue -> live destination parent
//!         Move_N/<snapshot>         full escaped branch subtree
//!         References                old -> clone ref remapping table
//!   PivotPlan                       Folder; automatic hierarchical pivots
//!     Pivot_1                       Folder; attrs: PivotOrder,
//!                                   PivotParentOrder?, Path, Delta
//!       Target                      ObjectValue -> live boundary instance
//! ```
//!
//! Live conflicted instances are tagged `RbxDiffConflict` so any consumer —
//! the CLI, a plain Studio command bar, or the rodeo resolver — can discover
//! them via CollectionService:GetTagged. Resolution is writing the entry's
//! `Resolved` attribute to "ours"/"theirs"; `finalize` applies the winners in
//! Rust (blobs included), strips the container and tags, and leaves a clean
//! artifact. Studio never serializes the final result.

use anyhow::{bail, Result};
use rbx_dom_weak::{types::Ref, InstanceBuilder, WeakDom};
use rbx_types::{Attributes, Tags, Variant};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};

use crate::diff::{
    attribute_variant_to_property_value, variant_to_property_value, CFrameValue, PropertyValue,
};
use crate::diff_dom::{DiffDom, DomView};
use crate::dom_utils::class_is_a;
use crate::edit_script::{get_sub_property, is_sub_property, set_sub_property, Anchor, EditOp};
use crate::explorer_tree::{ExplorerTree, ExplorerTrees, ExplorerVersion};
use crate::match_instances::get_instance_path;
use crate::merge::{ConflictKind, MergeResult};
use crate::placement::{apply_pivot_plan, PivotApplication, PivotOp};
use crate::reference_value::{direct_reference, with_direct_reference_target};
use crate::rigid_groups::{Rigid, RigidGroup};

pub const CONTAINER_NAME: &str = "__RbxDiffMerge";
pub const CONFLICT_TAG: &str = "RbxDiffConflict";
pub const ENTRY_TAG: &str = "RbxDiffConflictEntry";
pub const VIRTUAL_TREES_NAME: &str = "VirtualTrees";
/// Conflict container schema version, stamped as the container's `Version`
/// attribute. Bump when the on-file layout changes shape.
pub const SCHEMA_VERSION: u32 = 6;

const VIRTUAL_TREE_CHUNK_BYTES: usize = 100_000;
const MOVE_OUTS_NAME: &str = "MoveOuts";

/// Stamp the conflict container into a merged DOM. `base` is the merged
/// result (conflicted targets at base state); the branch DOMs supply the
/// competing versions to materialize.
pub fn stamp_conflicts(
    base: &mut WeakDom,
    ours_dom: &WeakDom,
    theirs_dom: &WeakDom,
    result: &MergeResult,
) {
    stamp_conflicts_from_views(base, ours_dom, theirs_dom, result);
}

/// Stamp conflicts while retaining compact immutable branch inputs.
pub fn stamp_compact_conflicts(
    base: &mut WeakDom,
    ours_dom: &DiffDom,
    theirs_dom: &DiffDom,
    result: &MergeResult,
) {
    stamp_conflicts_from_views(base, ours_dom, theirs_dom, result);
}

fn stamp_conflicts_from_views(
    base: &mut WeakDom,
    ours_dom: &dyn DomView,
    theirs_dom: &dyn DomView,
    result: &MergeResult,
) {
    if result.conflicts.is_empty() {
        return;
    }
    let explorer_trees = result
        .explorer_trees
        .as_ref()
        .expect("conflicted merge must retain resolver explorer trees");

    let container_parent = find_container_parent(base);
    let container = base.insert(
        container_parent,
        InstanceBuilder::new("Folder")
            .with_name(CONTAINER_NAME)
            .with_property(
                "Attributes",
                Variant::Attributes(
                    Attributes::new()
                        .with("Version", Variant::Float64(SCHEMA_VERSION as f64))
                        .with(
                            "ConflictCount",
                            Variant::Float64(result.conflicts.len() as f64),
                        ),
                ),
            ),
    );

    stamp_explorer_trees(base, container, explorer_trees);

    for (index, conflict) in result.conflicts.iter().enumerate() {
        let mut attrs = Attributes::new()
            .with(
                "Kind",
                Variant::String(kind_str(&conflict.kind).to_string()),
            )
            .with("Path", Variant::String(conflict.path.clone()));
        match &conflict.kind {
            ConflictKind::Property { name } => {
                attrs = attrs.with("Property", Variant::String(name.clone()));
            }
            ConflictKind::PropertyBundle { name, properties } => {
                attrs = attrs
                    .with("Property", Variant::String(name.clone()))
                    .with("Properties", Variant::String(properties.join(",")));
            }
            _ => {}
        }

        let mut tags = Tags::new();
        tags.push(ENTRY_TAG);

        let entry = base.insert(
            container,
            InstanceBuilder::new("Folder")
                .with_name(format!("Conflict_{}", index + 1))
                .with_property("Attributes", Variant::Attributes(attrs))
                .with_property("Tags", Variant::Tags(tags)),
        );

        base.insert(
            entry,
            InstanceBuilder::new("ObjectValue")
                .with_name("Target")
                .with_property("Value", Variant::Ref(conflict.base_ref)),
        );

        if let ConflictKind::Pivot {
            ours,
            theirs,
            order,
            parent_order,
        } = &conflict.kind
        {
            if let Some(instance) = base.get_by_ref_mut(entry) {
                let mut attrs = match instance.properties.get(&"Attributes".into()) {
                    Some(Variant::Attributes(attrs)) => attrs.clone(),
                    _ => Attributes::new(),
                };
                attrs.insert("PivotOrder".to_string(), Variant::Float64(*order as f64));
                if let Some(parent_order) = parent_order {
                    attrs.insert(
                        "PivotParentOrder".to_string(),
                        Variant::Float64(*parent_order as f64),
                    );
                }
                instance
                    .properties
                    .insert("Attributes".into(), Variant::Attributes(attrs));
            }
            let ours_impact = pivot_impact(base, conflict.base_ref, explorer_trees, ours);
            let theirs_impact = pivot_impact(base, conflict.base_ref, explorer_trees, theirs);
            stamp_pivot_side(base, entry, "Ours", ours, &ours_impact);
            stamp_pivot_side(base, entry, "Theirs", theirs, &theirs_impact);
        } else {
            stamp_side(
                base,
                entry,
                "Ours",
                &conflict.ours.edits,
                &conflict.ours.pivots,
                conflict.base_ref,
                ours_dom,
                &result.ours_identity.matched,
                &result.ours_identity.reverse_matched,
                explorer_trees,
                ExplorerVersion::Ours,
            );
            stamp_side(
                base,
                entry,
                "Theirs",
                &conflict.theirs.edits,
                &conflict.theirs.pivots,
                conflict.base_ref,
                theirs_dom,
                &result.theirs_identity.matched,
                &result.theirs_identity.reverse_matched,
                explorer_trees,
                ExplorerVersion::Theirs,
            );
        }

        tag_instance(base, conflict.base_ref, CONFLICT_TAG);
    }
}

/// Persist automatic local pivots when at least one hierarchical pivot remains
/// conflicted. These records have no `Kind`, so they are not
/// resolver decisions and do not affect the conflict count. Finalization and
/// Studio combine them with selected Pivot entries and apply the complete
/// plan in `PivotOrder` after ordinary resolutions.
pub fn stamp_pivot_plan(base: &mut WeakDom, pivots: &[PivotApplication]) {
    if pivots.is_empty() {
        return;
    }
    let Some(container) = find_container(base) else {
        return;
    };
    let plan = base.insert(
        container,
        InstanceBuilder::new("Folder").with_name("PivotPlan"),
    );
    for (index, pivot) in pivots.iter().enumerate() {
        let mut attrs = Attributes::new()
            .with("PivotOrder", Variant::Float64(pivot.order as f64))
            .with("Path", Variant::String(pivot.path.clone()))
            .with("Delta", Variant::CFrame(pivot.delta));
        if let Some(parent_order) = pivot.parent_order {
            attrs = attrs.with("PivotParentOrder", Variant::Float64(parent_order as f64));
        }
        let entry = base.insert(
            plan,
            InstanceBuilder::new("Folder")
                .with_name(format!("Pivot_{}", index + 1))
                .with_property("Attributes", Variant::Attributes(attrs)),
        );
        base.insert(
            entry,
            InstanceBuilder::new("ObjectValue")
                .with_name("Target")
                .with_property("Value", Variant::Ref(pivot.target_ref)),
        );
    }
}

/// Store the complete input hierarchies as compact data rather than physical
/// clones. A shared ObjectValue table links logical ids to instances that
/// exist in the partially merged result; unmatched/deleted nodes remain valid
/// virtual rows without a concrete subject.
fn stamp_explorer_trees(base: &mut WeakDom, container: Ref, trees: &ExplorerTrees) {
    let virtual_trees = base.insert(
        container,
        InstanceBuilder::new("Folder")
            .with_name(VIRTUAL_TREES_NAME)
            .with_property(
                "Attributes",
                Variant::Attributes(Attributes::new().with("Version", Variant::Float64(1.0))),
            ),
    );

    stamp_explorer_tree(base, virtual_trees, "Base", &trees.base);
    stamp_explorer_tree(base, virtual_trees, "Ours", &trees.ours);
    stamp_explorer_tree(base, virtual_trees, "Theirs", &trees.theirs);

    let subjects = base.insert(
        virtual_trees,
        InstanceBuilder::new("Folder").with_name("Subjects"),
    );
    let mut result_subjects: Vec<(u32, Ref)> = trees
        .result_subjects
        .iter()
        .map(|(&id, &referent)| (id, referent))
        .collect();
    result_subjects.sort_unstable_by_key(|(id, _)| *id);
    for (id, referent) in result_subjects {
        base.insert(
            subjects,
            InstanceBuilder::new("ObjectValue")
                .with_name(format!("N{id}"))
                .with_property("Value", Variant::Ref(referent)),
        );
    }
}

fn stamp_explorer_tree(base: &mut WeakDom, parent: Ref, name: &str, tree: &ExplorerTree) {
    // Arrays keep a large tree compact: [logical id, parent id or 0, name,
    // class]. Chunks avoid Studio/property string limits and are concatenated
    // before JSONDecode by the resolver.
    let records: Vec<(u32, u32, &str, &str)> = tree
        .nodes
        .iter()
        .map(|node| {
            (
                node.id,
                node.parent.unwrap_or(0),
                node.name.as_str(),
                node.class_name.as_str(),
            )
        })
        .collect();
    let encoded = serde_json::to_string(&records).expect("virtual explorer tree is serializable");
    let tree_folder = base.insert(
        parent,
        InstanceBuilder::new("Folder")
            .with_name(name)
            .with_property(
                "Attributes",
                Variant::Attributes(
                    Attributes::new().with("NodeCount", Variant::Float64(tree.nodes.len() as f64)),
                ),
            ),
    );
    for (index, chunk) in utf8_chunks(&encoded, VIRTUAL_TREE_CHUNK_BYTES).enumerate() {
        base.insert(
            tree_folder,
            InstanceBuilder::new("StringValue")
                .with_name(format!("Chunk_{index:06}"))
                .with_property("Value", Variant::String(chunk.to_string())),
        );
    }
}

fn utf8_chunks(value: &str, max_bytes: usize) -> impl Iterator<Item = &str> {
    let mut start = 0;
    std::iter::from_fn(move || {
        if start >= value.len() {
            return None;
        }
        let mut end = (start + max_bytes).min(value.len());
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        let chunk = &value[start..end];
        start = end;
        Some(chunk)
    })
}

fn stamp_pivot_side(
    base: &mut WeakDom,
    entry: Ref,
    side_name: &str,
    delta: &rbx_types::CFrame,
    impact: &ImpactSide,
) {
    let side_folder = base.insert(
        entry,
        InstanceBuilder::new("Folder")
            .with_name(side_name)
            .with_property(
                "Attributes",
                Variant::Attributes(Attributes::new().with("Delta", Variant::CFrame(*delta))),
            ),
    );
    stamp_impact(base, side_folder, impact);
}

/// Materialize one side's version of the conflict under the entry folder.
fn stamp_side(
    base: &mut WeakDom,
    entry: Ref,
    side_name: &str,
    side_ops: &[EditOp],
    side_pivots: &[PivotOp],
    base_ref: Ref,
    branch_dom: &dyn DomView,
    base_to_branch: &HashMap<Ref, Ref>,
    branch_to_base: &HashMap<Ref, Ref>,
    trees: &ExplorerTrees,
    version: ExplorerVersion,
) {
    let mut side_attrs = Attributes::new();
    let mut deep_clone = false;

    match side_ops.first() {
        Some(EditOp::RemoveSubtree { .. }) => {
            side_attrs = side_attrs.with("Deleted", Variant::Bool(true));
        }
        Some(EditOp::Move { new_parent, .. }) => {
            let dest_path = match new_parent {
                Anchor::Old(parent) => get_instance_path(base, *parent),
                Anchor::Added(branch_ref) => get_instance_path(branch_dom, *branch_ref),
            };
            side_attrs = side_attrs.with("DestinationPath", Variant::String(dest_path));
        }
        _ => {}
    }
    // An edit inside a delete-vs-edit conflict needs the whole edited subtree
    if !side_pivots.is_empty()
        || side_ops
            .iter()
            .any(|op| !matches!(op, EditOp::RemoveSubtree { .. } | EditOp::Move { .. }))
    {
        deep_clone = true;
    }

    let side_folder = base.insert(
        entry,
        InstanceBuilder::new("Folder")
            .with_name(side_name)
            .with_property("Attributes", Variant::Attributes(side_attrs)),
    );

    let impact = impact_for_ops_and_pivots(
        base,
        branch_dom,
        side_ops,
        side_pivots,
        base_to_branch,
        trees,
        version,
    );
    stamp_impact(base, side_folder, &impact);

    // A branch can delete the contested root after moving edited descendants
    // out of it. There is then no branch-side root to snapshot, so persist the
    // escaped subtrees and their live destinations explicitly. All roots
    // share one clone-ref map so references between separate escapes remain
    // intact; the Original/Clone table lets finalize repair incoming refs.
    stamp_move_outs(
        base,
        side_folder,
        side_ops,
        base_ref,
        branch_dom,
        base_to_branch,
        branch_to_base,
    );

    // Move destination for finalize, when it maps to a live base instance
    if let Some(EditOp::Move {
        new_parent: Anchor::Old(parent),
        ..
    }) = side_ops.first()
    {
        let parent = *parent;
        base.insert(
            side_folder,
            InstanceBuilder::new("ObjectValue")
                .with_name("Destination")
                .with_property("Value", Variant::Ref(parent)),
        );
    }

    // Clone this side's version of the instance (skip for pure deletes/moves)
    let is_delete = matches!(side_ops.first(), Some(EditOp::RemoveSubtree { .. }));
    let is_move_only =
        !side_ops.is_empty() && side_ops.iter().all(|op| matches!(op, EditOp::Move { .. }));
    if !is_delete && !is_move_only {
        if let Some(&branch_ref) = base_to_branch.get(&base_ref) {
            let (builder, branch_to_clone) =
                clone_from_branch(branch_dom, branch_ref, branch_to_base, deep_clone);
            base.insert(side_folder, builder);
            stamp_conditional_pivots(base, side_folder, side_pivots, &branch_to_clone);
        }
    }
}

fn stamp_move_outs(
    base: &mut WeakDom,
    side_folder: Ref,
    side_ops: &[EditOp],
    conflict_root: Ref,
    branch_dom: &dyn DomView,
    base_to_branch: &HashMap<Ref, Ref>,
    branch_to_base: &HashMap<Ref, Ref>,
) {
    let moved: Vec<(Ref, Ref, Ref)> = side_ops
        .iter()
        .filter_map(|op| {
            let EditOp::Move {
                old_ref,
                new_parent: Anchor::Old(destination),
            } = op
            else {
                return None;
            };
            if !is_within(base, *old_ref, conflict_root)
                || is_within(base, *destination, conflict_root)
            {
                return None;
            }
            Some((*old_ref, *destination, *base_to_branch.get(old_ref)?))
        })
        .collect();
    if moved.is_empty() {
        return;
    }

    let mut branch_to_clone = HashMap::new();
    for (_, _, branch_ref) in &moved {
        allocate_clone_refs(branch_dom, *branch_ref, true, &mut branch_to_clone);
    }

    let move_outs = base.insert(
        side_folder,
        InstanceBuilder::new("Folder").with_name(MOVE_OUTS_NAME),
    );
    for (index, (_, destination, branch_ref)) in moved.iter().enumerate() {
        let record = base.insert(
            move_outs,
            InstanceBuilder::new("Folder").with_name(format!("Move_{}", index + 1)),
        );
        base.insert(
            record,
            InstanceBuilder::new("ObjectValue")
                .with_name("Destination")
                .with_property("Value", Variant::Ref(*destination)),
        );
        let builder = build_branch_clone(
            branch_dom,
            *branch_ref,
            branch_to_base,
            &branch_to_clone,
            true,
        );
        base.insert(record, builder);
    }

    let references = base.insert(
        move_outs,
        InstanceBuilder::new("Folder").with_name("References"),
    );
    let mut pairs: Vec<(Ref, Ref)> = branch_to_clone
        .iter()
        .filter_map(|(branch_ref, clone_ref)| {
            let original = *branch_to_base.get(branch_ref)?;
            base.get_by_ref(original).map(|_| (original, *clone_ref))
        })
        .collect();
    pairs.sort_unstable_by_key(|(original, _)| original.to_string());
    for (index, (original, clone_ref)) in pairs.into_iter().enumerate() {
        let pair = base.insert(
            references,
            InstanceBuilder::new("Folder").with_name(format!("Ref_{}", index + 1)),
        );
        base.insert(
            pair,
            InstanceBuilder::new("ObjectValue")
                .with_name("Original")
                .with_property("Value", Variant::Ref(original)),
        );
        base.insert(
            pair,
            InstanceBuilder::new("ObjectValue")
                .with_name("Clone")
                .with_property("Value", Variant::Ref(clone_ref)),
        );
    }
}

fn is_within(dom: &WeakDom, referent: Ref, ancestor: Ref) -> bool {
    let mut current = referent;
    while let Some(instance) = dom.get_by_ref(current) {
        if current == ancestor {
            return true;
        }
        current = instance.parent();
    }
    false
}

fn stamp_conditional_pivots(
    base: &mut WeakDom,
    side_folder: Ref,
    pivots: &[PivotOp],
    branch_to_clone: &HashMap<Ref, Ref>,
) {
    if pivots.is_empty() {
        return;
    }
    let plan = base.insert(
        side_folder,
        InstanceBuilder::new("Folder").with_name("PivotPlan"),
    );
    for (index, pivot) in pivots.iter().enumerate() {
        let Some(&target) = branch_to_clone.get(&pivot.side_ref) else {
            continue;
        };
        let mut attrs = Attributes::new()
            .with("PivotOrder", Variant::Float64(pivot.order as f64))
            .with("Delta", Variant::CFrame(pivot.delta));
        if let Some(parent_order) = pivot.parent_order {
            attrs = attrs.with("PivotParentOrder", Variant::Float64(parent_order as f64));
        }
        let entry = base.insert(
            plan,
            InstanceBuilder::new("Folder")
                .with_name(format!("Pivot_{}", index + 1))
                .with_property("Attributes", Variant::Attributes(attrs)),
        );
        base.insert(
            entry,
            InstanceBuilder::new("ObjectValue")
                .with_name("Target")
                .with_property("Value", Variant::Ref(target)),
        );
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImpactSide {
    operations: Vec<ImpactOperation>,
    affected_ids: Vec<u32>,
    instance_count: usize,
    property_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImpactOperation {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    node_id: Option<u32>,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    property: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    destination: Option<String>,
    instance_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    before: Option<PropertyValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    after: Option<PropertyValue>,
}

fn stamp_impact(base: &mut WeakDom, side_folder: Ref, impact: &ImpactSide) {
    let encoded = serde_json::to_string(impact).expect("conflict impact is serializable");
    base.insert(
        side_folder,
        InstanceBuilder::new("StringValue")
            .with_name("__RbxDiffImpact")
            .with_property("Value", Variant::String(encoded)),
    );
}

fn patch_value(dom: &dyn DomView, property: &str, value: Option<&Variant>) -> PropertyValue {
    match value {
        None => PropertyValue::Nil,
        Some(Variant::Ref(r)) if r.is_none() => PropertyValue::Nil,
        Some(Variant::Ref(r)) => PropertyValue::Ref {
            value: dom
                .get_by_ref(*r)
                .map(|_| get_instance_path(dom, *r))
                .unwrap_or_else(|| format!("{r}")),
        },
        Some(value) if property.starts_with("Attributes.") => {
            attribute_variant_to_property_value(value)
        }
        Some(value) => variant_to_property_value(value),
    }
}

fn impact_for_ops(
    base: &WeakDom,
    branch: &dyn DomView,
    ops: &[EditOp],
    base_to_branch: &HashMap<Ref, Ref>,
    trees: &ExplorerTrees,
    version: ExplorerVersion,
) -> ImpactSide {
    let mut operations = Vec::new();
    let mut affected = BTreeSet::new();
    let mut property_count = 0;

    let base_path = |referent| get_instance_path(base, referent);
    let side_path = |referent| {
        base_to_branch
            .get(&referent)
            .map(|branch_ref| get_instance_path(branch, *branch_ref))
            .unwrap_or_else(|| base_path(referent))
    };

    for op in ops {
        let (kind, node_id, path, property, destination, ids, before, after) = match op {
            EditOp::SetProperty {
                old_ref,
                name,
                old_value,
                value,
            } => {
                property_count += 1;
                let id = trees.id_for(ExplorerVersion::Base, *old_ref);
                (
                    "Property",
                    id,
                    side_path(*old_ref),
                    Some(name.clone()),
                    None,
                    id.into_iter().collect(),
                    Some(patch_value(base, name, old_value.as_ref())),
                    Some(patch_value(branch, name, value.as_ref())),
                )
            }
            EditOp::SetName { old_ref, name } => {
                property_count += 1;
                let id = trees.id_for(ExplorerVersion::Base, *old_ref);
                let old_name = &base.get_by_ref(*old_ref).unwrap().name;
                (
                    "Property",
                    id,
                    side_path(*old_ref),
                    Some("Name".to_string()),
                    None,
                    id.into_iter().collect(),
                    Some(PropertyValue::String {
                        value: old_name.clone(),
                    }),
                    Some(PropertyValue::String {
                        value: name.clone(),
                    }),
                )
            }
            EditOp::RemoveSubtree { old_ref } => {
                let id = trees.id_for(ExplorerVersion::Base, *old_ref);
                let ids = id
                    .map(|root| trees.subtree_ids(ExplorerVersion::Base, root))
                    .unwrap_or_default();
                (
                    "Delete",
                    id,
                    base_path(*old_ref),
                    None,
                    None,
                    ids,
                    None,
                    None,
                )
            }
            EditOp::Move {
                old_ref,
                new_parent,
            } => {
                let id = trees.id_for(ExplorerVersion::Base, *old_ref);
                let ids = id
                    .map(|root| trees.subtree_ids(version, root))
                    .unwrap_or_default();
                let destination = match new_parent {
                    Anchor::Old(parent) => Some(side_path(*parent)),
                    Anchor::Added(branch_ref) => Some(get_instance_path(branch, *branch_ref)),
                };
                (
                    "Move",
                    id,
                    base_path(*old_ref),
                    None,
                    destination,
                    ids,
                    None,
                    None,
                )
            }
            EditOp::AddSubtree { new_ref, .. } => {
                let id = trees.id_for(version, *new_ref);
                let ids = id
                    .map(|root| trees.subtree_ids(version, root))
                    .unwrap_or_default();
                (
                    "Add",
                    id,
                    get_instance_path(branch, *new_ref),
                    None,
                    None,
                    ids,
                    None,
                    None,
                )
            }
        };
        affected.extend(ids.iter().copied());
        operations.push(ImpactOperation {
            kind,
            node_id,
            path,
            property,
            destination,
            instance_count: ids.len(),
            before,
            after,
        });
    }

    ImpactSide {
        operations,
        affected_ids: affected.iter().copied().collect(),
        instance_count: affected.len(),
        property_count,
    }
}

fn impact_for_ops_and_pivots(
    base: &WeakDom,
    branch: &dyn DomView,
    ops: &[EditOp],
    pivots: &[PivotOp],
    base_to_branch: &HashMap<Ref, Ref>,
    trees: &ExplorerTrees,
    version: ExplorerVersion,
) -> ImpactSide {
    let mut impact = impact_for_ops(base, branch, ops, base_to_branch, trees, version);
    for pivot in pivots {
        let operation_impact = pivot_impact(base, pivot.target_ref, trees, &pivot.delta);
        impact.operations.extend(operation_impact.operations);
        impact.affected_ids.extend(operation_impact.affected_ids);
        impact.property_count += operation_impact.property_count;
    }
    impact.affected_ids.sort_unstable();
    impact.affected_ids.dedup();
    impact.instance_count = impact.affected_ids.len();
    impact
}

fn pivot_impact(
    base: &WeakDom,
    target: Ref,
    trees: &ExplorerTrees,
    delta: &rbx_types::CFrame,
) -> ImpactSide {
    let mut pending = vec![target];
    let mut affected = BTreeSet::new();
    let mut operations = Vec::new();
    let transform = Rigid::from_cframe(delta);
    while let Some(referent) = pending.pop() {
        let Some(instance) = base.get_by_ref(referent) else {
            continue;
        };
        pending.extend(instance.children().iter().copied());

        let property_and_frame = if class_is_a(instance.class.as_str(), "BasePart") {
            match instance.properties.get(&"CFrame".into()) {
                Some(Variant::CFrame(frame)) => Some(("CFrame", *frame)),
                _ => None,
            }
        } else if class_is_a(instance.class.as_str(), "Model")
            && !class_is_a(instance.class.as_str(), "WorldRoot")
        {
            match instance.properties.get(&"WorldPivotData".into()) {
                Some(Variant::OptionalCFrame(Some(frame))) => Some(("WorldPivotData", *frame)),
                _ => None,
            }
        } else {
            None
        };
        let (Some(id), Some((property, frame))) = (
            trees.id_for(ExplorerVersion::Base, referent),
            property_and_frame,
        ) else {
            continue;
        };
        let transformed = transform.mul(Rigid::from_cframe(&frame)).to_cframe();
        affected.insert(id);
        operations.push(ImpactOperation {
            kind: "Property",
            node_id: Some(id),
            path: get_instance_path(base, referent),
            property: Some(property.to_string()),
            destination: None,
            instance_count: 1,
            before: Some(variant_to_property_value(&Variant::CFrame(frame))),
            after: Some(variant_to_property_value(&Variant::CFrame(transformed))),
        });
    }

    let affected_ids: Vec<u32> = affected.iter().copied().collect();
    ImpactSide {
        operations,
        instance_count: affected_ids.len(),
        property_count: affected_ids.len(),
        affected_ids,
    }
}

/// Clone a branch instance for the container. Allocate all clone referents
/// before copying properties so references between descendants keep pointing
/// inside the snapshot. References outside the snapshot are remapped through
/// branch identity into the live merged DOM (or nulled when identity is
/// unknown).
fn clone_from_branch(
    branch_dom: &dyn DomView,
    branch_ref: Ref,
    branch_to_base: &HashMap<Ref, Ref>,
    deep: bool,
) -> (InstanceBuilder, HashMap<Ref, Ref>) {
    let mut branch_to_clone = HashMap::new();
    allocate_clone_refs(branch_dom, branch_ref, deep, &mut branch_to_clone);
    let builder = build_branch_clone(
        branch_dom,
        branch_ref,
        branch_to_base,
        &branch_to_clone,
        deep,
    );
    (builder, branch_to_clone)
}

fn allocate_clone_refs(
    branch_dom: &dyn DomView,
    branch_ref: Ref,
    deep: bool,
    branch_to_clone: &mut HashMap<Ref, Ref>,
) {
    if branch_to_clone.contains_key(&branch_ref) {
        return;
    }
    branch_to_clone.insert(branch_ref, Ref::new());
    if deep {
        for child in branch_dom.get_by_ref(branch_ref).unwrap().children() {
            allocate_clone_refs(branch_dom, child, true, branch_to_clone);
        }
    }
}

fn build_branch_clone(
    branch_dom: &dyn DomView,
    branch_ref: Ref,
    branch_to_base: &HashMap<Ref, Ref>,
    branch_to_clone: &HashMap<Ref, Ref>,
    deep: bool,
) -> InstanceBuilder {
    let inst = branch_dom.get_by_ref(branch_ref).unwrap();
    let mut builder = InstanceBuilder::new(inst.class())
        .with_referent(branch_to_clone[&branch_ref])
        .with_name(inst.name());
    for (name, value) in inst.properties() {
        let value = match value {
            Variant::Ref(r) if !r.is_none() => Variant::Ref(
                branch_to_clone
                    .get(r)
                    .or_else(|| branch_to_base.get(r))
                    .copied()
                    .unwrap_or_else(Ref::none),
            ),
            other => other.clone(),
        };
        builder = builder.with_property(name, value);
    }
    if deep {
        let children: Vec<InstanceBuilder> = inst
            .children()
            .map(|child| {
                build_branch_clone(branch_dom, child, branch_to_base, branch_to_clone, true)
            })
            .collect();
        builder = builder.with_children(children);
    }
    builder
}

/// Places carry the container under ServerStorage; models under the root.
fn find_container_parent(dom: &WeakDom) -> Ref {
    let root = dom.root_ref();
    dom.root()
        .children()
        .iter()
        .copied()
        .find(|&c| {
            dom.get_by_ref(c)
                .map(|i| i.class.as_str() == "ServerStorage")
                .unwrap_or(false)
        })
        .unwrap_or(root)
}

fn kind_str(kind: &ConflictKind) -> &'static str {
    match kind {
        ConflictKind::Property { .. } => "Property",
        ConflictKind::PropertyBundle { .. } => "PropertyBundle",
        ConflictKind::DeleteVsEdit => "DeleteVsEdit",
        ConflictKind::MoveTarget => "MoveTarget",
        ConflictKind::Pivot { .. } => "Pivot",
    }
}

fn tag_instance(dom: &mut WeakDom, referent: Ref, tag: &str) {
    let Some(inst) = dom.get_by_ref_mut(referent) else {
        return;
    };
    let mut tags = match inst.properties.get(&"Tags".into()) {
        Some(Variant::Tags(existing)) => existing.clone(),
        _ => Tags::new(),
    };
    if !tags.iter().any(|t| t == tag) {
        tags.push(tag);
    }
    inst.properties.insert("Tags".into(), Variant::Tags(tags));
}

fn untag_instance(dom: &mut WeakDom, referent: Ref, tag: &str) {
    let Some(inst) = dom.get_by_ref_mut(referent) else {
        return;
    };
    if let Some(Variant::Tags(existing)) = inst.properties.get(&"Tags".into()) {
        let remaining: Vec<String> = existing
            .iter()
            .filter(|t| *t != tag)
            .map(|t| t.to_string())
            .collect();
        if remaining.is_empty() {
            inst.properties.remove(&"Tags".into());
        } else {
            let mut tags = Tags::new();
            for t in &remaining {
                tags.push(t);
            }
            inst.properties.insert("Tags".into(), Variant::Tags(tags));
        }
    }
}

// ============================================================================
// Reading, marking, finalizing
// ============================================================================

#[derive(Debug)]
pub struct ConflictEntry {
    pub entry_ref: Ref,
    /// Unique entry name within the container (e.g. "Conflict_2")
    pub name: String,
    pub kind: String,
    pub path: String,
    pub property: Option<String>,
    /// Atomic serialized fields for PropertyBundle entries.
    pub properties: Vec<String>,
    /// Hierarchical pivot application order (ancestors first).
    pub pivot_order: Option<usize>,
    pub pivot_parent_order: Option<usize>,
    pub resolved: Option<String>,
    /// Rigid-group entry name this conflict belongs to, if grouped.
    pub group: Option<String>,
}

pub fn find_container(dom: &WeakDom) -> Option<Ref> {
    dom.descendants()
        .find(|inst| inst.name == CONTAINER_NAME)
        .map(|inst| inst.referent())
}

pub fn list_entries(dom: &WeakDom, container: Ref) -> Vec<ConflictEntry> {
    let Some(container_inst) = dom.get_by_ref(container) else {
        return Vec::new();
    };
    container_inst
        .children()
        .iter()
        .filter_map(|&entry_ref| {
            let inst = dom.get_by_ref(entry_ref)?;
            let attrs = match inst.properties.get(&"Attributes".into()) {
                Some(Variant::Attributes(a)) => a,
                _ => return None,
            };
            let get_str = |key: &str| attr_string(attrs, key);
            Some(ConflictEntry {
                entry_ref,
                name: inst.name.clone(),
                kind: get_str("Kind")?,
                path: get_str("Path")?,
                property: get_str("Property"),
                properties: get_str("Properties")
                    .map(|properties| {
                        properties
                            .split(',')
                            .filter(|property| !property.is_empty())
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
                pivot_order: match attrs.get("PivotOrder") {
                    Some(Variant::Float64(value)) if *value >= 0.0 => Some(*value as usize),
                    Some(Variant::Float32(value)) if *value >= 0.0 => Some(*value as usize),
                    Some(Variant::Int32(value)) if *value >= 0 => Some(*value as usize),
                    Some(Variant::Int64(value)) if *value >= 0 => Some(*value as usize),
                    _ => None,
                },
                pivot_parent_order: numeric_attr(attrs, "PivotParentOrder")
                    .filter(|value| *value >= 0.0)
                    .map(|value| value as usize),
                resolved: get_str("Resolved").filter(|s| !s.is_empty()),
                group: get_str("Group"),
            })
        })
        .collect()
}

// ============================================================================
// Machine-readable report
// ============================================================================

/// Structured view of a file's conflict state, for automation: emitted by
/// `resolve --list --json` and (for the just-written file) `merge --json`.
/// Everything here is read back from the stamped container, so it describes
/// exactly what a resolver — CLI, agent, or Studio — will act on.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictReport {
    pub schema_version: u32,
    pub conflict_count: usize,
    pub unresolved_count: usize,
    pub groups: Vec<GroupReport>,
    pub conflicts: Vec<ConflictEntryReport>,
}

impl ConflictReport {
    /// The report for a file with no conflict state.
    pub fn empty() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            conflict_count: 0,
            unresolved_count: 0,
            groups: Vec::new(),
            conflicts: Vec::new(),
        }
    }
}

/// A rigid group: several spatial conflicts that are one logical decision.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupReport {
    /// Group entry name (e.g. "Group_1") — accepted by `--take --entry`.
    pub name: String,
    pub kind: String,
    pub path: String,
    /// Member conflict entry names.
    pub members: Vec<String>,
    pub delta_ours: CFrameValue,
    pub delta_theirs: CFrameValue,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictEntryReport {
    /// Unique entry name (e.g. "Conflict_2") — the key for `--take --entry`.
    pub name: String,
    pub kind: String,
    /// Base path of the contested instance.
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pivot_order: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pivot_parent_order: Option<usize>,
    /// "ours" | "theirs" | "custom", or absent while unresolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_value: Option<PropertyValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub ours: SideReport,
    pub theirs: SideReport,
}

/// What choosing one side would do.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SideReport {
    /// This side deleted the contested subtree.
    pub deleted: bool,
    /// Where this side moved the contested instance, when it moved it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_path: Option<String>,
    /// This side's placement delta for Pivot conflicts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pivot_delta: Option<CFrameValue>,
    /// The exact patch this choice applies: operations with before/after
    /// values (the stamped `__RbxDiffImpact`, parsed). Absent only if the
    /// container was produced by an older schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact: Option<serde_json::Value>,
}

/// Build the machine-readable report for a container.
pub fn conflict_report(dom: &WeakDom, container: Ref) -> ConflictReport {
    let schema_version = dom
        .get_by_ref(container)
        .and_then(|inst| match inst.properties.get(&"Attributes".into()) {
            Some(Variant::Attributes(attrs)) => numeric_attr(attrs, "Version"),
            _ => None,
        })
        .map(|version| version as u32)
        .unwrap_or(SCHEMA_VERSION);

    let entries = list_entries(dom, container);
    let conflicts: Vec<ConflictEntryReport> = entries
        .iter()
        .map(|entry| {
            let custom_value = dom
                .get_by_ref(entry.entry_ref)
                .and_then(|inst| match inst.properties.get(&"Attributes".into()) {
                    Some(Variant::Attributes(attrs)) => attrs.get("CustomValue").cloned(),
                    _ => None,
                })
                .map(|value| match entry.property.as_deref() {
                    Some(property) if is_sub_property(property) => {
                        attribute_variant_to_property_value(&value)
                    }
                    _ => variant_to_property_value(&value),
                });
            ConflictEntryReport {
                name: entry.name.clone(),
                kind: entry.kind.clone(),
                path: entry.path.clone(),
                property: entry.property.clone(),
                properties: entry.properties.clone(),
                pivot_order: entry.pivot_order,
                pivot_parent_order: entry.pivot_parent_order,
                resolved: entry.resolved.clone(),
                custom_value,
                group: entry.group.clone(),
                ours: side_report(dom, entry.entry_ref, "Ours"),
                theirs: side_report(dom, entry.entry_ref, "Theirs"),
            }
        })
        .collect();

    let groups = dom
        .get_by_ref(container)
        .map(|inst| inst.children().to_vec())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|group_ref| {
            let inst = dom.get_by_ref(group_ref)?;
            let attrs = match inst.properties.get(&"Attributes".into()) {
                Some(Variant::Attributes(attrs)) => attrs,
                _ => return None,
            };
            let kind = attr_string(attrs, "GroupKind")?;
            let cframe = |key: &str| match attrs.get(key) {
                Some(Variant::CFrame(cf)) => Some(CFrameValue::from(*cf)),
                _ => None,
            };
            Some(GroupReport {
                members: entries
                    .iter()
                    .filter(|entry| entry.group.as_deref() == Some(inst.name.as_str()))
                    .map(|entry| entry.name.clone())
                    .collect(),
                name: inst.name.clone(),
                kind,
                path: attr_string(attrs, "Path").unwrap_or_default(),
                delta_ours: cframe("DeltaOurs")?,
                delta_theirs: cframe("DeltaTheirs")?,
            })
        })
        .collect();

    ConflictReport {
        schema_version,
        conflict_count: conflicts.len(),
        unresolved_count: conflicts.iter().filter(|c| c.resolved.is_none()).count(),
        groups,
        conflicts,
    }
}

fn side_report(dom: &WeakDom, entry_ref: Ref, side: &str) -> SideReport {
    let Some(side_folder) = child_by_name(dom, entry_ref, side) else {
        return SideReport {
            deleted: false,
            destination_path: None,
            pivot_delta: None,
            impact: None,
        };
    };
    let destination_path = dom
        .get_by_ref(side_folder)
        .and_then(|inst| match inst.properties.get(&"Attributes".into()) {
            Some(Variant::Attributes(attrs)) => attr_string(attrs, "DestinationPath"),
            _ => None,
        });
    let impact = child_by_name(dom, side_folder, "__RbxDiffImpact")
        .and_then(|value_ref| dom.get_by_ref(value_ref))
        .and_then(|inst| match inst.properties.get(&"Value".into()) {
            Some(Variant::String(encoded)) => serde_json::from_str(encoded).ok(),
            _ => None,
        });
    SideReport {
        deleted: side_attr_bool(dom, side_folder, "Deleted"),
        destination_path,
        pivot_delta: side_attr_cframe(dom, side_folder, "Delta").map(CFrameValue::from),
        impact,
    }
}

/// Stamp rigid-group metadata: a Group_N folder per group (GroupKind — no
/// Kind attribute, so list_entries/finalize never see it as a conflict) and
/// a Group attribute on each member entry. Groups are presentation +
/// fan-out metadata only; members remain the ground truth.
pub fn stamp_rigid_groups(base: &mut WeakDom, groups: &[RigidGroup]) {
    let Some(container) = find_container(base) else {
        return;
    };
    for (index, group) in groups.iter().enumerate() {
        let group_name = format!("Group_{}", index + 1);
        let attrs = Attributes::new()
            .with("GroupKind", Variant::String("RigidMove".to_string()))
            .with("Path", Variant::String(group.path.clone()))
            .with("MemberCount", Variant::Float64(group.members.len() as f64))
            .with("DeltaOurs", Variant::CFrame(group.delta_ours))
            .with("DeltaTheirs", Variant::CFrame(group.delta_theirs));
        let entry = base.insert(
            container,
            InstanceBuilder::new("Folder")
                .with_name(group_name.clone())
                .with_property("Attributes", Variant::Attributes(attrs)),
        );
        base.insert(
            entry,
            InstanceBuilder::new("ObjectValue")
                .with_name("Target")
                .with_property("Value", Variant::Ref(group.lca)),
        );

        for &member_index in &group.members {
            let member_name = format!("Conflict_{}", member_index + 1);
            let Some(member_ref) = child_by_name(base, container, &member_name) else {
                continue;
            };
            let Some(inst) = base.get_by_ref_mut(member_ref) else {
                continue;
            };
            let mut member_attrs = match inst.properties.get(&"Attributes".into()) {
                Some(Variant::Attributes(a)) => a.clone(),
                _ => continue,
            };
            member_attrs.insert("Group".to_string(), Variant::String(group_name.clone()));
            inst.properties
                .insert("Attributes".into(), Variant::Attributes(member_attrs));
        }
    }
}

/// Mark an entry resolved toward "ours" or "theirs".
pub fn mark_entry(dom: &mut WeakDom, entry_ref: Ref, side: &str) -> Result<()> {
    if side != "ours" && side != "theirs" {
        bail!("resolution side must be 'ours' or 'theirs', got '{side}'");
    }
    let Some(inst) = dom.get_by_ref_mut(entry_ref) else {
        bail!("conflict entry no longer exists");
    };
    let mut attrs = match inst.properties.get(&"Attributes".into()) {
        Some(Variant::Attributes(a)) => a.clone(),
        _ => bail!("conflict entry has no attributes"),
    };
    attrs.insert("Resolved".to_string(), Variant::String(side.to_string()));
    inst.properties
        .insert("Attributes".into(), Variant::Attributes(attrs));
    Ok(())
}

/// Mark a Property-kind entry resolved with a caller-supplied value (the
/// "edit the result yourself" resolution). The value is stored ON the entry
/// as the CustomValue attribute — the file stays self-contained — after
/// being coerced to the conflicted property's real type, using the entry's
/// Ours/Theirs clones as the type template.
pub fn mark_entry_custom(
    dom: &mut WeakDom,
    entry_ref: Ref,
    value: &serde_json::Value,
) -> Result<()> {
    let entries: Vec<ConflictEntry> = {
        let Some(inst) = dom.get_by_ref(entry_ref) else {
            bail!("conflict entry no longer exists");
        };
        let parent = inst.parent();
        list_entries(dom, parent)
    };
    let entry = entries
        .iter()
        .find(|e| e.entry_ref == entry_ref)
        .ok_or_else(|| anyhow::anyhow!("conflict entry not found"))?;
    if entry.kind != "Property" {
        bail!(
            "custom resolution only applies to Property conflicts ({} is {})",
            entry.path,
            entry.kind
        );
    }
    let prop = entry.property.as_deref().ok_or_else(|| {
        anyhow::anyhow!("{}: property conflict without Property attr", entry.path)
    })?;

    // Type template: the same property on the Ours (or Theirs) clone
    let template = ["Ours", "Theirs"]
        .iter()
        .find_map(|side| {
            let folder = child_by_name(dom, entry_ref, side)?;
            let clone_ref = first_non_value_child(dom, folder)?;
            let inst = dom.get_by_ref(clone_ref)?;
            if prop == "Name" {
                Some(Variant::String(inst.name.clone()))
            } else if is_sub_property(prop) {
                get_sub_property(inst, prop)
            } else {
                inst.properties.get(&prop.into()).cloned()
            }
        })
        .ok_or_else(|| {
            anyhow::anyhow!("{}: no clone to derive the property type from", entry.path)
        })?;

    let coerced = coerce_json_to_variant(value, &template)
        .map_err(|e| anyhow::anyhow!("{}.{}: {}", entry.path, prop, e))?;
    if !attribute_storable(&coerced) {
        bail!(
            "{}.{}: this property type does not support custom resolution",
            entry.path,
            prop
        );
    }

    let Some(inst) = dom.get_by_ref_mut(entry_ref) else {
        bail!("conflict entry no longer exists");
    };
    let mut attrs = match inst.properties.get(&"Attributes".into()) {
        Some(Variant::Attributes(a)) => a.clone(),
        _ => bail!("conflict entry has no attributes"),
    };
    attrs.insert(
        "Resolved".to_string(),
        Variant::String("custom".to_string()),
    );
    attrs.insert("CustomValue".to_string(), coerced);
    inst.properties
        .insert("Attributes".into(), Variant::Attributes(attrs));
    Ok(())
}

/// Coerce plain JSON into the template's variant type. Untyped on the wire —
/// the conflicted property's existing type is authoritative.
fn coerce_json_to_variant(value: &serde_json::Value, template: &Variant) -> Result<Variant> {
    use serde_json::Value as J;
    let num = |v: &J| -> Result<f64> {
        v.as_f64()
            .ok_or_else(|| anyhow::anyhow!("expected a number, got {v}"))
    };
    let arr = |v: &J, n: usize| -> Result<Vec<f64>> {
        let items = v
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("expected an array of {n} numbers, got {v}"))?;
        if items.len() != n {
            bail!("expected {n} numbers, got {}", items.len());
        }
        items.iter().map(|x| num(x)).collect()
    };

    Ok(match template {
        Variant::String(_) => Variant::String(
            value
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("expected a string, got {value}"))?
                .to_string(),
        ),
        Variant::Bool(_) => Variant::Bool(
            value
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("expected a bool, got {value}"))?,
        ),
        Variant::Float32(_) => Variant::Float32(num(value)? as f32),
        Variant::Float64(_) => Variant::Float64(num(value)?),
        Variant::Int32(_) => Variant::Int32(num(value)? as i32),
        Variant::Int64(_) => Variant::Int64(num(value)? as i64),
        Variant::Color3(_) => {
            let c = arr(value, 3)?;
            Variant::Color3(rbx_types::Color3::new(
                c[0] as f32,
                c[1] as f32,
                c[2] as f32,
            ))
        }
        // Part.Color serializes as Color3uint8; attributes can only hold
        // Color3, so store that and let finalize's re-coercion narrow it
        Variant::Color3uint8(_) => {
            let c = arr(value, 3)?;
            Variant::Color3(rbx_types::Color3::new(
                c[0] as f32,
                c[1] as f32,
                c[2] as f32,
            ))
        }
        Variant::Vector3(_) => {
            let v = arr(value, 3)?;
            Variant::Vector3(rbx_types::Vector3::new(
                v[0] as f32,
                v[1] as f32,
                v[2] as f32,
            ))
        }
        Variant::Vector2(_) => {
            let v = arr(value, 2)?;
            Variant::Vector2(rbx_types::Vector2::new(v[0] as f32, v[1] as f32))
        }
        Variant::CFrame(_) => {
            let c = arr(value, 12)?;
            let f = |i: usize| c[i] as f32;
            Variant::CFrame(rbx_types::CFrame::new(
                rbx_types::Vector3::new(f(0), f(1), f(2)),
                rbx_types::Matrix3::new(
                    rbx_types::Vector3::new(f(3), f(4), f(5)),
                    rbx_types::Vector3::new(f(6), f(7), f(8)),
                    rbx_types::Vector3::new(f(9), f(10), f(11)),
                ),
            ))
        }
        Variant::UDim(_) => {
            let u = arr(value, 2)?;
            Variant::UDim(rbx_types::UDim::new(u[0] as f32, u[1] as i32))
        }
        Variant::UDim2(_) => {
            let u = arr(value, 4)?;
            Variant::UDim2(rbx_types::UDim2::new(
                rbx_types::UDim::new(u[0] as f32, u[1] as i32),
                rbx_types::UDim::new(u[2] as f32, u[3] as i32),
            ))
        }
        other => bail!(
            "custom resolution is not supported for {:?}-typed properties",
            other.ty()
        ),
    })
}

/// Attribute-storable check: CustomValue must survive the file round-trip as
/// an attribute (mirrors the supported set of coerce_json_to_variant).
fn attribute_storable(value: &Variant) -> bool {
    matches!(
        value,
        Variant::String(_)
            | Variant::Bool(_)
            | Variant::Float32(_)
            | Variant::Float64(_)
            | Variant::Int32(_)
            | Variant::Int64(_)
            | Variant::Color3(_)
            | Variant::Vector3(_)
            | Variant::Vector2(_)
            | Variant::CFrame(_)
            | Variant::UDim(_)
            | Variant::UDim2(_)
    )
}

/// Apply every entry's resolution, strip the container and tags, and leave a
/// clean DOM. Errors if any entry is unresolved.
pub fn finalize(dom: &mut WeakDom) -> Result<usize> {
    let Some(container) = find_container(dom) else {
        bail!("no conflict container ({CONTAINER_NAME}) in this file");
    };
    let entries = list_entries(dom, container);

    let unresolved: Vec<&ConflictEntry> = entries.iter().filter(|e| e.resolved.is_none()).collect();
    if !unresolved.is_empty() {
        let paths: Vec<&str> = unresolved.iter().map(|e| e.path.as_str()).collect();
        bail!(
            "{} unresolved conflict(s): {}",
            unresolved.len(),
            paths.join(", ")
        );
    }

    let count = entries.len();
    let pivot_actions = read_pivot_actions(dom, container, &entries)?;

    // Every ordinary resolution is expressed in canonical boundary coordinates.
    // The complete pivot plan (automatic plus selected decisions) is applied
    // afterwards in explicit top-down order. This is semantically necessary
    // for nested rotations; overlapping rigid transforms do not commute.
    for entry in entries.iter().filter(|entry| entry.kind != "Pivot") {
        apply_entry(dom, entry)?;
    }
    for action in &pivot_actions {
        untag_instance(dom, action.target_ref, CONFLICT_TAG);
    }
    apply_pivot_plan(dom, &pivot_actions);

    dom.destroy(container);
    Ok(count)
}

fn read_pivot_actions(
    dom: &WeakDom,
    container: Ref,
    entries: &[ConflictEntry],
) -> Result<Vec<PivotApplication>> {
    let mut actions = Vec::new();

    if let Some(plan) = child_by_name(dom, container, "PivotPlan") {
        let children = dom
            .get_by_ref(plan)
            .map(|instance| instance.children().to_vec())
            .unwrap_or_default();
        for pivot in children {
            let Some(instance) = dom.get_by_ref(pivot) else {
                continue;
            };
            let Some(Variant::Attributes(attrs)) = instance.properties.get(&"Attributes".into())
            else {
                continue;
            };
            let order = numeric_attr(attrs, "PivotOrder").ok_or_else(|| {
                anyhow::anyhow!("{}: automatic pivot has no PivotOrder", instance.name)
            })? as usize;
            let delta = match attrs.get("Delta") {
                Some(Variant::CFrame(delta)) => *delta,
                _ => bail!("{}: automatic pivot has no Delta", instance.name),
            };
            let target = child_object_value(dom, pivot, "Target").ok_or_else(|| {
                anyhow::anyhow!("{}: automatic pivot has no Target", instance.name)
            })?;
            actions.push(PivotApplication {
                target_ref: target,
                path: attr_string(attrs, "Path").unwrap_or_else(|| instance.name.clone()),
                order,
                parent_order: numeric_attr(attrs, "PivotParentOrder")
                    .filter(|value| *value >= 0.0)
                    .map(|value| value as usize),
                delta,
            });
        }
    }

    for entry in entries.iter().filter(|entry| entry.kind == "Pivot") {
        let side = entry.resolved.as_deref().unwrap();
        let side_folder_name = if side == "ours" { "Ours" } else { "Theirs" };
        let target = child_object_value(dom, entry.entry_ref, "Target")
            .ok_or_else(|| anyhow::anyhow!("{}: missing Target", entry.path))?;
        let side_folder =
            child_by_name(dom, entry.entry_ref, side_folder_name).ok_or_else(|| {
                anyhow::anyhow!("{}: missing {} folder", entry.path, side_folder_name)
            })?;
        let delta = side_attr_cframe(dom, side_folder, "Delta").ok_or_else(|| {
            anyhow::anyhow!("{}: {} pivot has no Delta", entry.path, side_folder_name)
        })?;
        actions.push(PivotApplication {
            target_ref: target,
            path: entry.path.clone(),
            order: entry.pivot_order.unwrap_or(usize::MAX),
            parent_order: entry.pivot_parent_order,
            delta,
        });
    }

    // Structural conflicts may carry a pivot on their surviving side (for
    // example, delete-vs-pivot). The clone is kept canonical; selecting that
    // side contributes these conditional operations to the same ordered plan.
    for entry in entries {
        let side = entry.resolved.as_deref().unwrap();
        let side_folder_name = if side == "ours" { "Ours" } else { "Theirs" };
        let Some(side_folder) = child_by_name(dom, entry.entry_ref, side_folder_name) else {
            continue;
        };
        let Some(plan) = child_by_name(dom, side_folder, "PivotPlan") else {
            continue;
        };
        let children = dom
            .get_by_ref(plan)
            .map(|instance| instance.children().to_vec())
            .unwrap_or_default();
        for pivot in children {
            let Some(instance) = dom.get_by_ref(pivot) else {
                continue;
            };
            let Some(Variant::Attributes(attrs)) = instance.properties.get(&"Attributes".into())
            else {
                continue;
            };
            let order = numeric_attr(attrs, "PivotOrder").ok_or_else(|| {
                anyhow::anyhow!("{}: conditional pivot has no PivotOrder", instance.name)
            })? as usize;
            let delta = match attrs.get("Delta") {
                Some(Variant::CFrame(delta)) => *delta,
                _ => bail!("{}: conditional pivot has no Delta", instance.name),
            };
            let target = child_object_value(dom, pivot, "Target").ok_or_else(|| {
                anyhow::anyhow!("{}: conditional pivot has no Target", instance.name)
            })?;
            actions.push(PivotApplication {
                target_ref: target,
                path: entry.path.clone(),
                order,
                parent_order: numeric_attr(attrs, "PivotParentOrder")
                    .filter(|value| *value >= 0.0)
                    .map(|value| value as usize),
                delta,
            });
        }
    }

    Ok(actions)
}

fn numeric_attr(attrs: &Attributes, key: &str) -> Option<f64> {
    match attrs.get(key) {
        Some(Variant::Float64(value)) => Some(*value),
        Some(Variant::Float32(value)) => Some(*value as f64),
        Some(Variant::Int32(value)) => Some(*value as f64),
        Some(Variant::Int64(value)) => Some(*value as f64),
        _ => None,
    }
}

fn apply_entry(dom: &mut WeakDom, entry: &ConflictEntry) -> Result<()> {
    let side = entry.resolved.as_deref().unwrap();
    let side_folder_name = if side == "ours" { "Ours" } else { "Theirs" };

    let target = child_object_value(dom, entry.entry_ref, "Target")
        .ok_or_else(|| anyhow::anyhow!("{}: missing Target", entry.path))?;
    let side_folder = child_by_name(dom, entry.entry_ref, side_folder_name)
        .ok_or_else(|| anyhow::anyhow!("{}: missing {} folder", entry.path, side_folder_name))?;

    untag_instance(dom, target, CONFLICT_TAG);

    match entry.kind.as_str() {
        "Property" if side == "custom" => {
            let prop = entry.property.as_deref().ok_or_else(|| {
                anyhow::anyhow!("{}: property conflict without Property attr", entry.path)
            })?;
            let custom = dom
                .get_by_ref(entry.entry_ref)
                .and_then(|inst| match inst.properties.get(&"Attributes".into()) {
                    Some(Variant::Attributes(a)) => a.get("CustomValue").cloned(),
                    _ => None,
                })
                .ok_or_else(|| {
                    anyhow::anyhow!("{}: resolved custom but no CustomValue stored", entry.path)
                })?;
            // Attributes round-trip strings as BinaryString and integers as
            // f64; re-coerce against the clone-derived template on the way out
            let template = ["Ours", "Theirs"].iter().find_map(|s| {
                let folder = child_by_name(dom, entry.entry_ref, s)?;
                let clone_ref = first_non_value_child(dom, folder)?;
                let inst = dom.get_by_ref(clone_ref)?;
                if prop == "Name" {
                    Some(Variant::String(inst.name.clone()))
                } else if is_sub_property(prop) {
                    get_sub_property(inst, prop)
                } else {
                    inst.properties.get(&prop.into()).cloned()
                }
            });
            let value = match (&custom, &template) {
                (Variant::BinaryString(b), Some(Variant::String(_))) => Variant::String(
                    String::from_utf8(b.clone().into_vec())
                        .map_err(|_| anyhow::anyhow!("{}: CustomValue is not UTF-8", entry.path))?,
                ),
                (Variant::Float64(n), Some(Variant::Float32(_))) => Variant::Float32(*n as f32),
                (Variant::Float64(n), Some(Variant::Int32(_))) => Variant::Int32(*n as i32),
                (Variant::Float64(n), Some(Variant::Int64(_))) => Variant::Int64(*n as i64),
                (Variant::Color3(c), Some(Variant::Color3uint8(_))) => {
                    Variant::Color3uint8(rbx_types::Color3uint8::new(
                        (c.r * 255.0).round().clamp(0.0, 255.0) as u8,
                        (c.g * 255.0).round().clamp(0.0, 255.0) as u8,
                        (c.b * 255.0).round().clamp(0.0, 255.0) as u8,
                    ))
                }
                _ => custom,
            };

            if prop == "Name" {
                let name = match value {
                    Variant::String(s) => s,
                    other => bail!(
                        "{}: custom Name must be a string, got {:?}",
                        entry.path,
                        other.ty()
                    ),
                };
                if let Some(inst) = dom.get_by_ref_mut(target) {
                    inst.name = name;
                }
            } else if let Some(inst) = dom.get_by_ref_mut(target) {
                if !set_sub_property(inst, prop, Some(&value)) {
                    inst.properties.insert(prop.into(), value);
                }
            }
        }
        "Property" => {
            let prop = entry.property.as_deref().ok_or_else(|| {
                anyhow::anyhow!("{}: property conflict without Property attr", entry.path)
            })?;
            let clone_ref = first_non_value_child(dom, side_folder).ok_or_else(|| {
                anyhow::anyhow!("{}: missing {} clone", entry.path, side_folder_name)
            })?;

            if prop == "Name" {
                let name = dom.get_by_ref(clone_ref).unwrap().name.clone();
                if let Some(inst) = dom.get_by_ref_mut(target) {
                    inst.name = name;
                }
            } else {
                let value = dom.get_by_ref(clone_ref).and_then(|inst| {
                    if is_sub_property(prop) {
                        get_sub_property(inst, prop)
                    } else {
                        inst.properties.get(&prop.into()).cloned()
                    }
                });
                if let Some(inst) = dom.get_by_ref_mut(target) {
                    if is_sub_property(prop) {
                        set_sub_property(inst, prop, value.as_ref());
                    } else {
                        match value {
                            Some(v) => {
                                inst.properties.insert(prop.into(), v);
                            }
                            None => {
                                inst.properties.remove(&prop.into());
                            }
                        }
                    }
                }
            }
        }
        "PropertyBundle" => {
            if entry.properties.is_empty() {
                bail!("{}: property bundle has no Properties", entry.path);
            }
            let clone_ref = first_non_value_child(dom, side_folder).ok_or_else(|| {
                anyhow::anyhow!("{}: missing {} clone", entry.path, side_folder_name)
            })?;
            let values: Vec<_> = entry
                .properties
                .iter()
                .map(|property| {
                    (
                        property.clone(),
                        dom.get_by_ref(clone_ref)
                            .and_then(|instance| instance.properties.get(&property.as_str().into()))
                            .cloned(),
                    )
                })
                .collect();
            let Some(target_instance) = dom.get_by_ref_mut(target) else {
                bail!("{}: bundle target no longer exists", entry.path);
            };
            for (property, value) in values {
                match value {
                    Some(value) => {
                        target_instance.properties.insert(property.into(), value);
                    }
                    None => {
                        target_instance.properties.remove(&property.into());
                    }
                }
            }
        }
        "DeleteVsEdit" => {
            let deleted = side_attr_bool(dom, side_folder, "Deleted");
            if deleted {
                dom.destroy(target);
            } else if apply_move_outs(dom, side_folder)? {
                // Both branches delete the old container, but this side first
                // preserves edited descendants by moving them elsewhere.
                // The snapshots now occupy those destinations and all
                // incoming authored references point at their clone refs.
                dom.destroy(target);
            } else if let Some(clone_ref) = first_non_value_child(dom, side_folder) {
                // Replace the base subtree with this side's edited version
                let parent = dom.get_by_ref(target).map(|i| i.parent());
                if let Some(parent) = parent {
                    dom.transfer_within(clone_ref, parent);
                    dom.destroy(target);
                }
            }
            // An edit-side win with no clone means the edit was a move-out;
            // base content already stands, nothing to do.
        }
        "MoveTarget" => {
            let dest = child_object_value(dom, side_folder, "Destination").ok_or_else(|| {
                anyhow::anyhow!(
                    "{}: move destination is not resolvable in this file (it was inside content added by the other branch); re-merge and pick the other side, or move manually",
                    entry.path
                )
            })?;
            if dom.get_by_ref(dest).is_none() {
                bail!("{}: move destination no longer exists", entry.path);
            }
            dom.transfer_within(target, dest);
        }
        "Pivot" => {
            bail!(
                "{}: Pivot must be applied through its pivot plan",
                entry.path
            );
        }
        other => bail!("{}: unknown conflict kind '{other}'", entry.path),
    }

    Ok(())
}

/// Read a string attribute. The binary format round-trips string attribute
/// values as BinaryString, so accept both encodings.
fn attr_string(attrs: &Attributes, key: &str) -> Option<String> {
    match attrs.get(key) {
        Some(Variant::String(s)) => Some(s.clone()),
        Some(Variant::BinaryString(b)) => String::from_utf8(b.clone().into_vec()).ok(),
        _ => None,
    }
}

fn child_by_name(dom: &WeakDom, parent: Ref, name: &str) -> Option<Ref> {
    dom.get_by_ref(parent)?
        .children()
        .iter()
        .copied()
        .find(|&c| dom.get_by_ref(c).map(|i| i.name == name).unwrap_or(false))
}

fn child_object_value(dom: &WeakDom, parent: Ref, name: &str) -> Option<Ref> {
    let ov = child_by_name(dom, parent, name)?;
    match dom.get_by_ref(ov)?.properties.get(&"Value".into()) {
        Some(Variant::Ref(r)) if !r.is_none() => Some(*r),
        _ => None,
    }
}

/// The materialized clone under a side folder (skipping resolver metadata).
fn first_non_value_child(dom: &WeakDom, side_folder: Ref) -> Option<Ref> {
    dom.get_by_ref(side_folder)?
        .children()
        .iter()
        .copied()
        .find(|&c| {
            dom.get_by_ref(c)
                .map(|i| {
                    i.class.as_str() != "ObjectValue"
                        && i.name != "__RbxDiffImpact"
                        && i.name != "PivotPlan"
                        && i.name != MOVE_OUTS_NAME
                })
                .unwrap_or(false)
        })
}

fn apply_move_outs(dom: &mut WeakDom, side_folder: Ref) -> Result<bool> {
    let Some(move_outs) = child_by_name(dom, side_folder, MOVE_OUTS_NAME) else {
        return Ok(false);
    };
    let references = child_by_name(dom, move_outs, "References")
        .ok_or_else(|| anyhow::anyhow!("MoveOuts is missing its References table"))?;
    let pair_refs = dom
        .get_by_ref(references)
        .map(|instance| instance.children().to_vec())
        .unwrap_or_default();
    let mut remap = HashMap::new();
    for pair in pair_refs {
        let original = child_object_value(dom, pair, "Original")
            .ok_or_else(|| anyhow::anyhow!("move-out reference is missing Original"))?;
        let clone_ref = child_object_value(dom, pair, "Clone")
            .ok_or_else(|| anyhow::anyhow!("move-out reference is missing Clone"))?;
        remap.insert(original, clone_ref);
    }

    let records = dom
        .get_by_ref(move_outs)
        .map(|instance| instance.children().to_vec())
        .unwrap_or_default();
    let mut transfers = Vec::new();
    for record in records {
        let Some(instance) = dom.get_by_ref(record) else {
            continue;
        };
        if instance.name == "References" {
            continue;
        }
        let destination = child_object_value(dom, record, "Destination")
            .ok_or_else(|| anyhow::anyhow!("{} is missing Destination", instance.name))?;
        let snapshot = first_non_value_child(dom, record)
            .ok_or_else(|| anyhow::anyhow!("{} is missing its snapshot", instance.name))?;
        transfers.push((snapshot, destination));
    }
    for (snapshot, destination) in transfers {
        if dom.get_by_ref(destination).is_none() {
            bail!("move-out destination no longer exists");
        }
        dom.transfer_within(snapshot, destination);
    }
    remap_authored_references(dom, &remap);
    Ok(true)
}

fn remap_authored_references(dom: &mut WeakDom, remap: &HashMap<Ref, Ref>) {
    if remap.is_empty() {
        return;
    }
    let referents: Vec<Ref> = dom
        .descendants()
        .map(|instance| instance.referent())
        .collect();
    for referent in referents {
        let Some(instance) = dom.get_by_ref_mut(referent) else {
            continue;
        };
        let replacements: Vec<_> = instance
            .properties
            .iter()
            .filter_map(|(name, value)| {
                let remapped = remap_variant_references(value, remap);
                (remapped != *value).then(|| (name.clone(), remapped))
            })
            .collect();
        for (name, value) in replacements {
            instance.properties.insert(name, value);
        }
    }
}

fn remap_variant_references(value: &Variant, remap: &HashMap<Ref, Ref>) -> Variant {
    if let Some((_, target)) = direct_reference(value) {
        return remap
            .get(&target)
            .copied()
            .map(|target| with_direct_reference_target(value.clone(), target))
            .unwrap_or_else(|| value.clone());
    }
    if let Variant::Attributes(attributes) = value {
        let mut remapped = Attributes::new();
        for (name, value) in attributes {
            remapped.insert(name.clone(), remap_variant_references(value, remap));
        }
        return Variant::Attributes(remapped);
    }
    value.clone()
}

fn side_attr_bool(dom: &WeakDom, side_folder: Ref, key: &str) -> bool {
    dom.get_by_ref(side_folder)
        .and_then(|inst| match inst.properties.get(&"Attributes".into()) {
            Some(Variant::Attributes(a)) => match a.get(key) {
                Some(Variant::Bool(b)) => Some(*b),
                _ => None,
            },
            _ => None,
        })
        .unwrap_or(false)
}

fn side_attr_cframe(dom: &WeakDom, side_folder: Ref, key: &str) -> Option<rbx_types::CFrame> {
    dom.get_by_ref(side_folder)
        .and_then(|inst| match inst.properties.get(&"Attributes".into()) {
            Some(Variant::Attributes(attributes)) => match attributes.get(key) {
                Some(Variant::CFrame(value)) => Some(*value),
                _ => None,
            },
            _ => None,
        })
}
