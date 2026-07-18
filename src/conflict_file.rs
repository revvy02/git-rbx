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

use crate::edit_script::{Anchor, EditOp};
use crate::match_instances::get_instance_path;
use crate::merge::{ConflictKind, MergeResult};

pub const CONTAINER_NAME: &str = "__RbxDiffMerge";
pub const CONFLICT_TAG: &str = "RbxDiffConflict";
pub const ENTRY_TAG: &str = "RbxDiffConflictEntry";

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
    let theirs_to_base: HashMap<Ref, Ref> =
        result.theirs_matched.iter().map(|(b, t)| (*t, *b)).collect();

    let container_parent = find_container_parent(base);
    let container = base.insert(
        container_parent,
        InstanceBuilder::new("Folder")
            .with_name(CONTAINER_NAME)
            .with_property(
                "Attributes",
                Variant::Attributes(
                    Attributes::new()
                        .with("Version", Variant::Float64(1.0))
                        .with("ConflictCount", Variant::Float64(result.conflicts.len() as f64)),
                ),
            ),
    );

    for (index, conflict) in result.conflicts.iter().enumerate() {
        let mut attrs = Attributes::new()
            .with("Kind", Variant::String(kind_str(&conflict.kind).to_string()))
            .with("Path", Variant::String(conflict.path.clone()));
        if let ConflictKind::Property { name } = &conflict.kind {
            attrs = attrs.with("Property", Variant::String(name.clone()));
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

        stamp_side(
            base, entry, "Ours", &conflict.ours, conflict.base_ref,
            ours_dom, &result.ours_matched, &ours_to_base,
        );
        stamp_side(
            base, entry, "Theirs", &conflict.theirs, conflict.base_ref,
            theirs_dom, &result.theirs_matched, &theirs_to_base,
        );

        tag_instance(base, conflict.base_ref, CONFLICT_TAG);
    }
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
    if let Some(EditOp::Move { new_parent: Anchor::Old(parent), .. }) = side_ops.first() {
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

/// Clone a branch instance for the container. Ref-valued properties are
/// remapped into merged-DOM refs where identity allows (else nulled).
fn clone_from_branch(
    branch_dom: &WeakDom,
    branch_ref: Ref,
    branch_to_base: &HashMap<Ref, Ref>,
    deep: bool,
) -> InstanceBuilder {
    let inst = branch_dom.get_by_ref(branch_ref).unwrap();
    let mut builder = InstanceBuilder::new(inst.class.as_str()).with_name(inst.name.as_str());
    for (name, value) in &inst.properties {
        let value = match value {
            Variant::Ref(r) if !r.is_none() => {
                Variant::Ref(branch_to_base.get(r).copied().unwrap_or_else(Ref::none))
            }
            other => other.clone(),
        };
        builder = builder.with_property(name.as_str(), value);
    }
    if deep {
        let children: Vec<InstanceBuilder> = inst
            .children()
            .iter()
            .map(|&child| clone_from_branch(branch_dom, child, branch_to_base, true))
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
        ConflictKind::DeleteVsEdit => "DeleteVsEdit",
        ConflictKind::MoveTarget => "MoveTarget",
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
    pub resolved: Option<String>,
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
                resolved: get_str("Resolved").filter(|s| !s.is_empty()),
            })
        })
        .collect()
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
    inst.properties.insert("Attributes".into(), Variant::Attributes(attrs));
    Ok(())
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
        bail!("{} unresolved conflict(s): {}", unresolved.len(), paths.join(", "));
    }

    let count = entries.len();
    for entry in entries {
        apply_entry(dom, &entry)?;
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
        "Property" => {
            let prop = entry
                .property
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("{}: property conflict without Property attr", entry.path))?;
            let clone_ref = first_non_value_child(dom, side_folder)
                .ok_or_else(|| anyhow::anyhow!("{}: missing {} clone", entry.path, side_folder_name))?;

            if prop == "Name" {
                let name = dom.get_by_ref(clone_ref).unwrap().name.clone();
                if let Some(inst) = dom.get_by_ref_mut(target) {
                    inst.name = name;
                }
            } else {
                let value = dom
                    .get_by_ref(clone_ref)
                    .and_then(|inst| inst.properties.get(&prop.into()).cloned());
                if let Some(inst) = dom.get_by_ref_mut(target) {
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
