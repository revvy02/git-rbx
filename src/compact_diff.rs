//! Dense identity-guided diff execution for compact DOMs.

use rbx_dom_weak::types::Ref;
use rbx_types::Variant;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::diff::{
    attr_value_eq, is_default_value, is_studio_artifact, raw_property_changes,
    semantic_changes_to_diff, DiffConfig, DiffEntry,
};
use crate::diff_dom::{DiffDom, DiffNode, DomView, InstanceView, NodeId};
use crate::edit_script::{Anchor, EditOp, InstanceIdentity, SemanticChangeSet};
use crate::value_compare::non_ref_variants_equal;

struct DenseIdentity {
    old_to_new: Vec<Option<NodeId>>,
    new_to_old: Vec<Option<NodeId>>,
}

impl DenseIdentity {
    fn from_complete(old_dom: &DiffDom, new_dom: &DiffDom, identity: &InstanceIdentity) -> Self {
        let mut old_to_new = vec![None; old_dom.len()];
        let mut new_to_old = vec![None; new_dom.len()];
        old_to_new[old_dom.root_id().index()] = Some(new_dom.root_id());
        new_to_old[new_dom.root_id().index()] = Some(old_dom.root_id());
        for (&old_ref, &new_ref) in identity.matched.iter() {
            let (Some(old_id), Some(new_id)) = (
                old_dom.id_from_source_ref(old_ref),
                new_dom.id_from_source_ref(new_ref),
            ) else {
                continue;
            };
            old_to_new[old_id.index()] = Some(new_id);
            new_to_old[new_id.index()] = Some(old_id);
        }
        Self {
            old_to_new,
            new_to_old,
        }
    }

    fn local_new_child(
        &self,
        new_dom: &DiffDom,
        old_child: NodeId,
        new_parent: NodeId,
    ) -> Option<NodeId> {
        self.old_to_new[old_child.index()]
            .filter(|&new_child| new_dom.node(new_child).parent() == Some(new_parent))
    }

    fn is_local_new_child(&self, old_dom: &DiffDom, new_child: NodeId, old_parent: NodeId) -> bool {
        self.new_to_old[new_child.index()]
            .is_some_and(|old_child| old_dom.node(old_child).parent() == Some(old_parent))
    }
}

fn property_is_semantically_absent(
    defaults: Option<&HashMap<&str, Variant>>,
    name: &str,
    value: &Variant,
) -> bool {
    is_default_value(defaults, name, value)
        || matches!(value, Variant::Ref(referent) if referent.is_none())
        || matches!(value, Variant::Attributes(attributes) if attributes.is_empty())
        || matches!(value, Variant::Tags(tags) if tags.is_empty())
}

fn container_values_equal(name: &str, old: &Variant, new: &Variant) -> bool {
    match (name, old, new) {
        ("Attributes", Variant::Attributes(old), Variant::Attributes(new)) => {
            old.len() == new.len()
                && old.iter().all(|(key, old_value)| {
                    new.get(key.as_str())
                        .is_some_and(|new_value| attr_value_eq(old_value, new_value))
                })
        }
        ("Tags", Variant::Tags(old), Variant::Tags(new)) => {
            old.len() == new.len()
                && old
                    .iter()
                    .all(|old_tag| new.iter().any(|new_tag| new_tag == old_tag))
        }
        _ => false,
    }
}

fn frozen_variants_equal(
    old_dom: &DiffDom,
    new_dom: &DiffDom,
    old: &Variant,
    new: &Variant,
    identity: &InstanceIdentity,
) -> bool {
    match (old, new) {
        (Variant::Ref(old_target), Variant::Ref(new_target)) => {
            let old_exists =
                !old_target.is_none() && old_dom.id_from_source_ref(*old_target).is_some();
            let new_exists =
                !new_target.is_none() && new_dom.id_from_source_ref(*new_target).is_some();
            match (old_exists, new_exists) {
                (false, false) => true,
                (true, false) | (false, true) => false,
                (true, true) => identity.matched.get(old_target) == Some(new_target),
            }
        }
        _ => non_ref_variants_equal(old, new),
    }
}

fn next_comparable_property<'a>(
    properties: &mut impl Iterator<Item = (&'a str, &'a Variant)>,
    config: &DiffConfig,
) -> Option<(&'a str, &'a Variant)> {
    properties.find(|(name, _)| !config.ignore_properties.contains(*name))
}

fn compact_instance_equal(
    old_dom: &DiffDom,
    new_dom: &DiffDom,
    old: DiffNode<'_>,
    new: DiffNode<'_>,
    identity: &InstanceIdentity,
    config: &DiffConfig,
) -> bool {
    if old.name() != new.name() || old.class() != new.class() {
        return false;
    }

    let database = rbx_reflection_database::get().unwrap();
    let defaults = database
        .classes
        .get(new.class())
        .map(|class| &class.default_properties);
    let mut old_properties = old.authored_properties();
    let mut new_properties = new.authored_properties();
    let mut old_property = next_comparable_property(&mut old_properties, config);
    let mut new_property = next_comparable_property(&mut new_properties, config);

    loop {
        match (old_property, new_property) {
            (None, None) => return true,
            (Some((old_name, old_value)), None) => {
                if !property_is_semantically_absent(defaults, old_name, old_value) {
                    return false;
                }
                old_property = next_comparable_property(&mut old_properties, config);
            }
            (None, Some((new_name, new_value))) => {
                if !property_is_semantically_absent(defaults, new_name, new_value) {
                    return false;
                }
                new_property = next_comparable_property(&mut new_properties, config);
            }
            (Some((old_name, old_value)), Some((new_name, new_value))) => {
                match old_name.cmp(new_name) {
                    Ordering::Less => {
                        if !property_is_semantically_absent(defaults, old_name, old_value) {
                            return false;
                        }
                        old_property = next_comparable_property(&mut old_properties, config);
                    }
                    Ordering::Greater => {
                        if !property_is_semantically_absent(defaults, new_name, new_value) {
                            return false;
                        }
                        new_property = next_comparable_property(&mut new_properties, config);
                    }
                    Ordering::Equal => {
                        let equal = if old_name == "Attributes" || old_name == "Tags" {
                            container_values_equal(old_name, old_value, new_value)
                        } else {
                            frozen_variants_equal(old_dom, new_dom, old_value, new_value, identity)
                        };
                        if !equal {
                            return false;
                        }
                        old_property = next_comparable_property(&mut old_properties, config);
                        new_property = next_comparable_property(&mut new_properties, config);
                    }
                }
            }
        }
    }
}

fn compact_changed_subtrees(
    old_dom: &DiffDom,
    new_dom: &DiffDom,
    identity: &InstanceIdentity,
    dense: &DenseIdentity,
    config: &DiffConfig,
) -> Vec<bool> {
    let mut changed = vec![false; old_dom.len()];
    for old_index in (0..old_dom.len()).rev() {
        let old_id = NodeId::from_index(old_index);
        let Some(new_id) = dense.old_to_new[old_index] else {
            continue;
        };
        let old = old_dom.node(old_id);
        let new = new_dom.node(new_id);
        let mut subtree_changed =
            !compact_instance_equal(old_dom, new_dom, old, new, identity, config);

        for old_child in old.children() {
            match dense.local_new_child(new_dom, old_child, new_id) {
                Some(_) => subtree_changed |= changed[old_child.index()],
                None => subtree_changed = true,
            }
        }
        if new
            .children()
            .any(|new_child| !dense.is_local_new_child(old_dom, new_child, old_id))
        {
            subtree_changed = true;
        }
        changed[old_index] = subtree_changed;
    }
    changed
}

struct CompactDiffContext<'a> {
    old_dom: &'a DiffDom,
    new_dom: &'a DiffDom,
    identity: &'a InstanceIdentity,
    dense: &'a DenseIdentity,
    changed: &'a [bool],
    config: &'a DiffConfig,
    moved_old: &'a HashSet<Ref>,
    moved_new: &'a HashSet<Ref>,
}

fn compact_change_pass(
    context: &CompactDiffContext<'_>,
    old_parent: NodeId,
    new_parent: NodeId,
    ops: &mut Vec<EditOp>,
) {
    let old_parent_node = context.old_dom.node(old_parent);
    let new_parent_node = context.new_dom.node(new_parent);

    for old_child in old_parent_node.children() {
        if context
            .dense
            .local_new_child(context.new_dom, old_child, new_parent)
            .is_some()
        {
            continue;
        }
        let old_ref = context.old_dom.node(old_child).source_ref();
        if context.moved_old.contains(&old_ref) {
            continue;
        }
        let instance = context.old_dom.node(old_child);
        if is_studio_artifact(
            context.old_dom,
            old_parent_node.source_ref(),
            InstanceView::Compact(instance),
        ) {
            continue;
        }
        ops.push(EditOp::RemoveSubtree { old_ref });
    }

    for new_child in new_parent_node.children() {
        if context
            .dense
            .is_local_new_child(context.old_dom, new_child, old_parent)
        {
            continue;
        }
        let new_ref = context.new_dom.node(new_child).source_ref();
        if context.moved_new.contains(&new_ref) {
            continue;
        }
        let instance = context.new_dom.node(new_child);
        if is_studio_artifact(
            context.new_dom,
            new_parent_node.source_ref(),
            InstanceView::Compact(instance),
        ) {
            continue;
        }
        ops.push(EditOp::AddSubtree {
            parent: Anchor::Old(old_parent_node.source_ref()),
            new_ref,
        });
    }

    for old_child in old_parent_node.children() {
        let Some(new_child) = context
            .dense
            .local_new_child(context.new_dom, old_child, new_parent)
        else {
            continue;
        };
        if !context.changed[old_child.index()] {
            continue;
        }
        let old_ref = context.old_dom.node(old_child).source_ref();
        let new_ref = context.new_dom.node(new_child).source_ref();
        emit_compact_instance_changes(context, old_ref, new_ref, ops);
        compact_change_pass(context, old_child, new_child, ops);
    }
}

fn emit_compact_instance_changes(
    context: &CompactDiffContext<'_>,
    old_ref: Ref,
    new_ref: Ref,
    ops: &mut Vec<EditOp>,
) {
    let old_instance = context.old_dom.get_by_ref(old_ref).unwrap();
    let new_instance = context.new_dom.get_by_ref(new_ref).unwrap();
    if old_instance.name() != new_instance.name() {
        ops.push(EditOp::SetName {
            old_ref,
            name: new_instance.name().to_string(),
        });
    }
    for change in raw_property_changes(
        context.old_dom,
        context.new_dom,
        old_ref,
        new_ref,
        context.config,
        &context.identity.matched,
    ) {
        ops.push(EditOp::SetProperty {
            old_ref,
            name: change.name,
            old_value: change.old,
            value: change.new,
        });
    }
}

fn new_side_depth(dom: &DiffDom, mut referent: Ref) -> usize {
    let mut depth = 0;
    while let Some(instance) = dom.get_by_ref(referent) {
        referent = instance.parent();
        depth += 1;
    }
    depth
}

fn anchor_for(new_ref: Ref, reverse_matched: &HashMap<Ref, Ref>) -> Anchor {
    reverse_matched
        .get(&new_ref)
        .copied()
        .map(Anchor::Old)
        .unwrap_or(Anchor::Added(new_ref))
}

/// Diff compact DOMs through identity already established for frame analysis.
///
/// A paired semantic comparison marks changed subtrees bottom-up in dense
/// node order. Emission then visits only those paths, avoiding two complete
/// cryptographic subtree-hash maps after identity is already known.
pub(crate) fn compute_compact_diff_with_identity(
    old_dom: &DiffDom,
    new_dom: &DiffDom,
    identity: &InstanceIdentity,
    config: &DiffConfig,
) -> Vec<DiffEntry> {
    let dense = DenseIdentity::from_complete(old_dom, new_dom, identity);
    let changed = compact_changed_subtrees(old_dom, new_dom, identity, &dense, config);
    let moved_old: HashSet<Ref> = identity.moves.iter().map(|(old, _)| *old).collect();
    let moved_new: HashSet<Ref> = identity.moves.iter().map(|(_, new)| *new).collect();
    let context = CompactDiffContext {
        old_dom,
        new_dom,
        identity,
        dense: &dense,
        changed: &changed,
        config,
        moved_old: &moved_old,
        moved_new: &moved_new,
    };

    let reverse_matched: HashMap<Ref, Ref> = identity
        .matched
        .iter()
        .map(|(old, new)| (*new, *old))
        .collect();
    let mut moves_by_depth = identity.moves.clone();
    moves_by_depth.sort_by_key(|(_, new_ref)| new_side_depth(new_dom, *new_ref));
    let mut ops = Vec::new();
    for &(old_ref, new_ref) in &moves_by_depth {
        let new_parent = new_dom
            .get_by_ref(new_ref)
            .map(|instance| instance.parent())
            .unwrap_or_else(Ref::none);
        ops.push(EditOp::Move {
            old_ref,
            new_parent: anchor_for(new_parent, &reverse_matched),
        });
    }

    compact_change_pass(&context, old_dom.root_id(), new_dom.root_id(), &mut ops);
    for &(old_ref, new_ref) in &identity.moves {
        emit_compact_instance_changes(&context, old_ref, new_ref, &mut ops);
        let (Some(old_id), Some(new_id)) = (
            old_dom.id_from_source_ref(old_ref),
            new_dom.id_from_source_ref(new_ref),
        ) else {
            continue;
        };
        if changed[old_id.index()] {
            compact_change_pass(&context, old_id, new_id, &mut ops);
        }
    }
    let changes = SemanticChangeSet {
        ops,
        matched: identity.matched.clone(),
        moved_destinations: moved_new.clone(),
        moves: identity.moves.clone(),
    };
    semantic_changes_to_diff(old_dom, new_dom, &changes)
}
