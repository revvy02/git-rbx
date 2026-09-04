//! The canonical serialized diff: an edit script that can be replayed onto
//! the old version to produce the new one, addressed by stable ids.
//!
//! In memory, ops point at rbx_dom refs, which are random per load and
//! meaningless outside the process. Here every instance is named by a small
//! integer from two manifests (old and new), captured while both DOMs are
//! loaded; an instance matched across versions has the same id in both.
//! Consumers resolve an id by walking the manifest, which is unambiguous
//! even with duplicate sibling names.
//!
//! Adds carry their whole subtree with every authored property, since
//! nothing else could reconstruct them. Removes name only the root. Edits
//! carry both values so the document is reviewable and invertible.

use rbx_dom_weak::types::Ref;
use rbx_types::Variant;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::diff::{
    attribute_variant_to_property_value, is_default_value, should_compare_property,
    variant_to_property_value, CFrameValue, DiffConfig, PropertyValue,
};
use crate::diff_dom::DomView;
use crate::edit_script::{Anchor, EditOp, SemanticChangeSet};
use crate::explorer_tree::{capture_tree, ExplorerTree};
use crate::match_instances::get_instance_path;

pub const DOCUMENT_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestNode {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<u32>,
    pub name: String,
    pub class: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddedInstance {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<u32>,
    pub name: String,
    pub class: String,
    pub properties: BTreeMap<String, PropertyValue>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum DocumentOp {
    /// Create `subtree` (pre-order, root first) under `parent`.
    #[serde(rename_all = "camelCase")]
    Add {
        id: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent: Option<u32>,
        instance_count: usize,
        subtree: Vec<AddedInstance>,
    },
    /// Delete the old subtree rooted at `id`.
    #[serde(rename_all = "camelCase")]
    Remove { id: u32, instance_count: usize },
    /// Move old instance `id` from parent `from` to parent `to`.
    #[serde(rename_all = "camelCase")]
    Reparent {
        id: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        from: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        to: Option<u32>,
    },
    #[serde(rename_all = "camelCase")]
    SetName { id: u32, before: String, after: String },
    #[serde(rename_all = "camelCase")]
    SetProperty {
        id: u32,
        property: String,
        before: PropertyValue,
        after: PropertyValue,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPivot {
    pub id: u32,
    pub order: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_order: Option<usize>,
    pub delta: CFrameValue,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DocumentCounts {
    pub added: usize,
    pub removed: usize,
    pub modified: usize,
    pub reparented: usize,
    pub pivoted: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffDocument {
    pub schema: u32,
    pub old: Vec<ManifestNode>,
    pub new: Vec<ManifestNode>,
    pub counts: DocumentCounts,
    pub ops: Vec<DocumentOp>,
    pub pivots: Vec<DocumentPivot>,
}

fn manifest(tree: ExplorerTree) -> Vec<ManifestNode> {
    tree.nodes
        .into_iter()
        .map(|node| ManifestNode {
            id: node.id,
            parent: node.parent,
            name: node.name,
            class: node.class_name,
        })
        .collect()
}

/// Typed value with Ref targets carrying the target's manifest id, so a
/// consumer can resolve references without parsing paths.
fn value_json(
    dom: &dyn DomView,
    ids: &HashMap<Ref, u32>,
    property: &str,
    value: Option<&Variant>,
) -> PropertyValue {
    match value {
        None => PropertyValue::Nil,
        Some(Variant::Ref(r)) if r.is_none() => PropertyValue::Nil,
        Some(Variant::Ref(r)) => PropertyValue::Ref {
            value: dom
                .get_by_ref(*r)
                .map(|_| get_instance_path(dom, *r))
                .unwrap_or_else(|| format!("{r}")),
            id: ids.get(r).copied(),
        },
        Some(value) if property.starts_with("Attributes.") => {
            attribute_variant_to_property_value(value)
        }
        Some(value) => variant_to_property_value(value),
    }
}

/// Every authored, non-default property of one instance, with container
/// properties expanded per key exactly as edits are.
fn authored_properties(
    dom: &dyn DomView,
    ids: &HashMap<Ref, u32>,
    referent: Ref,
    config: &DiffConfig,
) -> BTreeMap<String, PropertyValue> {
    let instance = dom.get_by_ref(referent).unwrap();
    let class = instance.class();
    let database = rbx_reflection_database::get().unwrap();
    let defaults = database
        .classes
        .get(class)
        .map(|cd| &cd.default_properties);

    let mut out = BTreeMap::new();
    for (name, value) in instance.properties() {
        if config.ignore_properties.contains(name) || !should_compare_property(class, name) {
            continue;
        }
        match value {
            Variant::Attributes(attrs) if name == "Attributes" => {
                for (key, attr) in attrs.iter() {
                    out.insert(
                        format!("Attributes.{key}"),
                        attribute_variant_to_property_value(attr),
                    );
                }
            }
            Variant::Tags(tags) if name == "Tags" => {
                for tag in tags.iter() {
                    out.insert(
                        format!("Tags.{tag}"),
                        PropertyValue::String {
                            value: tag.to_string(),
                        },
                    );
                }
            }
            Variant::Ref(r) if r.is_none() => {}
            _ => {
                if is_default_value(defaults, name, value) {
                    continue;
                }
                out.insert(name.to_string(), value_json(dom, ids, name, Some(value)));
            }
        }
    }
    out
}

fn subtree_preorder(dom: &dyn DomView, root: Ref) -> Vec<Ref> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(referent) = stack.pop() {
        let Some(instance) = dom.get_by_ref(referent) else {
            continue;
        };
        out.push(referent);
        let children: Vec<Ref> = instance.children().collect();
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
    out
}


/// Serialize edit ops against two DOMs whose instances are already numbered.
/// Shared by the diff document and the per-side conflict impacts, so both
/// speak the same op vocabulary.
pub(crate) fn ops_from_edit_ops(
    old: &dyn DomView,
    new: &dyn DomView,
    edit_ops: &[EditOp],
    old_ids: &HashMap<Ref, u32>,
    new_ids: &HashMap<Ref, u32>,
    config: &DiffConfig,
) -> (Vec<DocumentOp>, DocumentCounts) {
    let anchor_id = |anchor: &Anchor| match anchor {
        Anchor::Old(r) => old_ids.get(r).copied(),
        Anchor::Added(r) => new_ids.get(r).copied(),
    };
    let old_parent_id = |referent: Ref| {
        old.get_by_ref(referent)
            .and_then(|instance| old_ids.get(&instance.parent()).copied())
    };

    let mut ops = Vec::with_capacity(edit_ops.len());
    let mut counts = DocumentCounts::default();
    let mut modified = HashSet::new();

    for op in edit_ops {
        match op {
            EditOp::AddSubtree { parent, new_ref } => {
                let Some(id) = new_ids.get(new_ref).copied() else {
                    continue;
                };
                let parent = anchor_id(parent);
                let subtree: Vec<AddedInstance> = subtree_preorder(new, *new_ref)
                    .into_iter()
                    .filter_map(|referent| {
                        let instance = new.get_by_ref(referent)?;
                        let node_id = new_ids.get(&referent).copied()?;
                        let node_parent = if referent == *new_ref {
                            parent
                        } else {
                            new_ids.get(&instance.parent()).copied()
                        };
                        Some(AddedInstance {
                            id: node_id,
                            parent: node_parent,
                            name: instance.name().to_string(),
                            class: instance.class().to_string(),
                            properties: authored_properties(new, new_ids, referent, config),
                        })
                    })
                    .collect();
                counts.added += 1;
                ops.push(DocumentOp::Add {
                    id,
                    parent,
                    instance_count: subtree.len(),
                    subtree,
                });
            }
            EditOp::RemoveSubtree { old_ref } => {
                let Some(id) = old_ids.get(old_ref).copied() else {
                    continue;
                };
                counts.removed += 1;
                ops.push(DocumentOp::Remove {
                    id,
                    instance_count: subtree_preorder(old, *old_ref).len(),
                });
            }
            EditOp::Reparent {
                old_ref,
                new_parent,
            } => {
                let Some(id) = old_ids.get(old_ref).copied() else {
                    continue;
                };
                counts.reparented += 1;
                ops.push(DocumentOp::Reparent {
                    id,
                    from: old_parent_id(*old_ref),
                    to: anchor_id(new_parent),
                });
            }
            EditOp::SetName { old_ref, name } => {
                let Some(id) = old_ids.get(old_ref).copied() else {
                    continue;
                };
                modified.insert(id);
                let before = old
                    .get_by_ref(*old_ref)
                    .map(|instance| instance.name().to_string())
                    .unwrap_or_default();
                ops.push(DocumentOp::SetName {
                    id,
                    before,
                    after: name.clone(),
                });
            }
            EditOp::SetProperty {
                old_ref,
                name,
                old_value,
                value,
            } => {
                let Some(id) = old_ids.get(old_ref).copied() else {
                    continue;
                };
                modified.insert(id);
                ops.push(DocumentOp::SetProperty {
                    id,
                    property: name.clone(),
                    before: value_json(old, old_ids, name, old_value.as_ref()),
                    after: value_json(new, new_ids, name, value.as_ref()),
                });
            }
        }
    }
    counts.modified = modified.len();
    (ops, counts)
}

/// Serialize a change set computed from `old` to `new`.
pub fn build(
    old: &dyn DomView,
    new: &dyn DomView,
    changes: &SemanticChangeSet,
    config: &DiffConfig,
) -> DiffDocument {
    let mut next_id = 1;
    let (old_tree, old_ids) = capture_tree(old, &HashMap::new(), &mut next_id);
    let known: HashMap<Ref, u32> = changes
        .identity
        .reverse_matched
        .iter()
        .filter_map(|(new_ref, old_ref)| old_ids.get(old_ref).map(|id| (*new_ref, *id)))
        .collect();
    let (new_tree, new_ids) = capture_tree(new, &known, &mut next_id);

    let (ops, mut counts) = ops_from_edit_ops(old, new, &changes.ops, &old_ids, &new_ids, config);

    let pivots: Vec<DocumentPivot> = changes
        .pivots
        .iter()
        .filter_map(|pivot| {
            Some(DocumentPivot {
                id: old_ids.get(&pivot.target_ref).copied()?,
                order: pivot.order,
                parent_order: pivot.parent_order,
                delta: CFrameValue::from(pivot.delta),
            })
        })
        .collect();
    counts.pivoted = pivots.len();

    DiffDocument {
        schema: DOCUMENT_SCHEMA,
        old: manifest(old_tree),
        new: manifest(new_tree),
        counts,
        ops,
        pivots,
    }
}

impl DocumentOp {
    /// The instance an op addresses: the added root, or the old instance.
    pub fn id(&self) -> u32 {
        match self {
            DocumentOp::Add { id, .. }
            | DocumentOp::Remove { id, .. }
            | DocumentOp::Reparent { id, .. }
            | DocumentOp::SetName { id, .. }
            | DocumentOp::SetProperty { id, .. } => *id,
        }
    }

    pub fn is_property_edit(&self) -> bool {
        matches!(self, DocumentOp::SetName { .. } | DocumentOp::SetProperty { .. })
    }
}
