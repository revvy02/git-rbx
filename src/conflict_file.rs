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
//!       <clone of our version>      (shallow for property conflicts,
//!                                    full subtree for delete-vs-edit)
//!     Theirs                        Folder; same shape
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
use std::collections::HashMap;

use crate::edit_script::{get_sub_property, is_sub_property, set_sub_property, Anchor, EditOp};
use crate::explorer_tree::{ExplorerTree, ExplorerTrees};
use crate::match_instances::get_instance_path;
use crate::merge::{ConflictKind, MergeResult};
use crate::model_normalize::apply_model_frame;
use crate::rigid_groups::RigidGroup;

pub const CONTAINER_NAME: &str = "__RbxDiffMerge";
pub const CONFLICT_TAG: &str = "RbxDiffConflict";
pub const ENTRY_TAG: &str = "RbxDiffConflictEntry";
pub const VIRTUAL_TREES_NAME: &str = "VirtualTrees";

const VIRTUAL_TREE_CHUNK_BYTES: usize = 100_000;

/// Stamp the conflict container into a merged DOM. `base` is the merged
/// result (conflicted targets at base state); the branch DOMs supply the
/// competing versions to materialize.
pub fn stamp_conflicts(
    base: &mut WeakDom,
    ours_dom: &WeakDom,
    theirs_dom: &WeakDom,
    result: &MergeResult,
) {
    if result.conflicts.is_empty() {
        return;
    }

    let ours_to_base: HashMap<Ref, Ref> =
        result.ours_matched.iter().map(|(b, o)| (*o, *b)).collect();
    let theirs_to_base: HashMap<Ref, Ref> = result
        .theirs_matched
        .iter()
        .map(|(b, t)| (*t, *b))
        .collect();

    let container_parent = find_container_parent(base);
    let container = base.insert(
        container_parent,
        InstanceBuilder::new("Folder")
            .with_name(CONTAINER_NAME)
            .with_property(
                "Attributes",
                Variant::Attributes(
                    Attributes::new()
                        .with("Version", Variant::Float64(2.0))
                        .with(
                            "ConflictCount",
                            Variant::Float64(result.conflicts.len() as f64),
                        ),
                ),
            ),
    );

    stamp_explorer_trees(base, container, &result.explorer_trees);

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

        if let ConflictKind::ModelFrame { ours, theirs } = &conflict.kind {
            stamp_model_frame_side(base, entry, "Ours", ours);
            stamp_model_frame_side(base, entry, "Theirs", theirs);
        } else {
            stamp_side(
                base,
                entry,
                "Ours",
                &conflict.ours,
                conflict.base_ref,
                ours_dom,
                &result.ours_matched,
                &ours_to_base,
            );
            stamp_side(
                base,
                entry,
                "Theirs",
                &conflict.theirs,
                conflict.base_ref,
                theirs_dom,
                &result.theirs_matched,
                &theirs_to_base,
            );
        }

        tag_instance(base, conflict.base_ref, CONFLICT_TAG);
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

fn stamp_model_frame_side(
    base: &mut WeakDom,
    entry: Ref,
    side_name: &str,
    delta: &rbx_types::CFrame,
) {
    base.insert(
        entry,
        InstanceBuilder::new("Folder")
            .with_name(side_name)
            .with_property(
                "Attributes",
                Variant::Attributes(Attributes::new().with("Delta", Variant::CFrame(*delta))),
            ),
    );
}

/// Materialize one side's version of the conflict under the entry folder.
fn stamp_side(
    base: &mut WeakDom,
    entry: Ref,
    side_name: &str,
    side_ops: &[EditOp],
    base_ref: Ref,
    branch_dom: &WeakDom,
    base_to_branch: &HashMap<Ref, Ref>,
    branch_to_base: &HashMap<Ref, Ref>,
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
    if side_ops
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
    let is_move_only = side_ops.iter().all(|op| matches!(op, EditOp::Move { .. }));
    if !is_delete && !is_move_only {
        if let Some(&branch_ref) = base_to_branch.get(&base_ref) {
            let builder = clone_from_branch(branch_dom, branch_ref, branch_to_base, deep_clone);
            base.insert(side_folder, builder);
        }
    }
}

/// Clone a branch instance for the container. Allocate all clone referents
/// before copying properties so references between descendants keep pointing
/// inside the snapshot. References outside the snapshot are remapped through
/// branch identity into the live merged DOM (or nulled when identity is
/// unknown).
fn clone_from_branch(
    branch_dom: &WeakDom,
    branch_ref: Ref,
    branch_to_base: &HashMap<Ref, Ref>,
    deep: bool,
) -> InstanceBuilder {
    let mut branch_to_clone = HashMap::new();
    allocate_clone_refs(branch_dom, branch_ref, deep, &mut branch_to_clone);
    build_branch_clone(
        branch_dom,
        branch_ref,
        branch_to_base,
        &branch_to_clone,
        deep,
    )
}

fn allocate_clone_refs(
    branch_dom: &WeakDom,
    branch_ref: Ref,
    deep: bool,
    branch_to_clone: &mut HashMap<Ref, Ref>,
) {
    branch_to_clone.insert(branch_ref, Ref::new());
    if deep {
        for &child in branch_dom.get_by_ref(branch_ref).unwrap().children() {
            allocate_clone_refs(branch_dom, child, true, branch_to_clone);
        }
    }
}

fn build_branch_clone(
    branch_dom: &WeakDom,
    branch_ref: Ref,
    branch_to_base: &HashMap<Ref, Ref>,
    branch_to_clone: &HashMap<Ref, Ref>,
    deep: bool,
) -> InstanceBuilder {
    let inst = branch_dom.get_by_ref(branch_ref).unwrap();
    let mut builder = InstanceBuilder::new(inst.class.as_str())
        .with_referent(branch_to_clone[&branch_ref])
        .with_name(inst.name.as_str());
    for (name, value) in &inst.properties {
        let value = match value {
            Variant::Ref(r) if !r.is_none() => {
                Variant::Ref(
                    branch_to_clone
                        .get(r)
                        .or_else(|| branch_to_base.get(r))
                        .copied()
                        .unwrap_or_else(Ref::none),
                )
            }
            other => other.clone(),
        };
        builder = builder.with_property(name.as_str(), value);
    }
    if deep {
        let children: Vec<InstanceBuilder> = inst
            .children()
            .iter()
            .map(|&child| {
                build_branch_clone(
                    branch_dom,
                    child,
                    branch_to_base,
                    branch_to_clone,
                    true,
                )
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
        ConflictKind::ModelFrame { .. } => "ModelFrame",
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
                resolved: get_str("Resolved").filter(|s| !s.is_empty()),
                group: get_str("Group"),
            })
        })
        .collect()
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
    // ModelFrame is deliberately last: every other resolution is expressed
    // in the canonical content frame, then the chosen asset placement carries
    // the complete resolved tree into world space as one operation.
    for entry in entries.iter().filter(|entry| entry.kind != "ModelFrame") {
        apply_entry(dom, entry)?;
    }
    for entry in entries.iter().filter(|entry| entry.kind == "ModelFrame") {
        apply_entry(dom, entry)?;
    }

    dom.destroy(container);
    Ok(count)
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
        "ModelFrame" => {
            let delta = side_attr_cframe(dom, side_folder, "Delta").ok_or_else(|| {
                anyhow::anyhow!("{}: {} frame has no Delta", entry.path, side_folder_name)
            })?;
            apply_model_frame(dom, target, &delta);
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

/// The materialized clone under a side folder (skipping ObjectValues).
fn first_non_value_child(dom: &WeakDom, side_folder: Ref) -> Option<Ref> {
    dom.get_by_ref(side_folder)?
        .children()
        .iter()
        .copied()
        .find(|&c| {
            dom.get_by_ref(c)
                .map(|i| i.class.as_str() != "ObjectValue")
                .unwrap_or(false)
        })
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
