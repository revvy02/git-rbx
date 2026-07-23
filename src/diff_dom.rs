//! Compact immutable representation used by comparison workloads.
//!
//! `WeakDom` is the correct materialization type for editing and serializing
//! Roblox trees, but its per-instance maps and owned strings are expensive for
//! large read-only comparisons. `DiffDom` stores the same logical information
//! in dense arenas:
//!
//! * nodes, child links, and properties are contiguous;
//! * names, class names, and property names share a local string table;
//! * internal relationships use compact `NodeId`s;
//! * source referents are retained for Ref-valued properties, diagnostics, and
//!   eventual edit-plan materialization.
//!
//! The first integration stage builds this representation from a `WeakDom`.
//! A future binary decoder can populate the same arenas directly without
//! changing matching, diff, or merge semantics.

#[cfg(test)]
use rbx_dom_weak::InstanceBuilder;
use rbx_dom_weak::{types::Ref, Instance, Ustr, WeakDom};
use rbx_types::{UniqueId, Variant};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::ops::Range;
use std::slice;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct NodeId(u32);

impl NodeId {
    fn from_index(index: usize) -> Self {
        Self(u32::try_from(index).expect("DiffDom cannot contain more than u32::MAX nodes"))
    }

    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StringId(u32);

#[derive(Default)]
struct StringTable {
    values: Vec<Arc<str>>,
    by_value: HashMap<Arc<str>, StringId>,
}

impl StringTable {
    fn intern(&mut self, value: &str) -> StringId {
        if let Some(&id) = self.by_value.get(value) {
            return id;
        }

        let id = StringId(
            u32::try_from(self.values.len())
                .expect("DiffDom cannot contain more than u32::MAX unique strings"),
        );
        let value: Arc<str> = Arc::from(value);
        self.values.push(Arc::clone(&value));
        self.by_value.insert(value, id);
        id
    }

    fn finish(&mut self) {
        // The reverse index is needed only while constructing the arena.
        // Property ranges are sorted by their resolved names, so retaining
        // another hash-table entry for every unique string buys nothing at
        // comparison time.
        self.by_value = HashMap::new();
    }

    fn resolve(&self, id: StringId) -> &str {
        &self.values[id.0 as usize]
    }
}

struct Node {
    source_ref: Ref,
    parent: Option<NodeId>,
    name: StringId,
    class: Ustr,
    children: Range<u32>,
    properties: Range<u32>,
}

struct Property {
    name: Ustr,
    value: Variant,
}

/// Dense, read-mostly DOM used by matching and diff computation.
pub struct DiffDom {
    nodes: Vec<Node>,
    children: Vec<NodeId>,
    properties: Vec<Property>,
    strings: StringTable,
    by_source_ref: HashMap<Ref, NodeId>,
}

struct BinaryInstance {
    source_ref: Ref,
    name: String,
    class: Ustr,
    properties: Vec<(Ustr, Variant)>,
}

impl rbx_binary::DecodeInstance for BinaryInstance {
    fn new(class: Ustr, _property_capacity: usize) -> Self {
        Self {
            source_ref: Ref::new(),
            name: class.to_string(),
            class,
            // The reflection default-property count is only a loose upper
            // bound for serialized properties and substantially over-reserves
            // on large places. Grow from the actual PROP chunks instead.
            properties: Vec::new(),
        }
    }

    fn referent(&self) -> Ref {
        self.source_ref
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn has_property(&self, name: Ustr) -> bool {
        self.properties
            .iter()
            .any(|(property_name, _)| *property_name == name)
    }

    fn add_property(&mut self, name: Ustr, value: Variant) {
        self.properties.push((name, value));
    }
}

struct DiffDomDecodeTarget {
    nodes: Vec<Node>,
    children: Vec<NodeId>,
    properties: Vec<Property>,
    strings: StringTable,
    by_source_ref: HashMap<Ref, NodeId>,
    unique_ids: HashSet<UniqueId>,
}

impl DiffDomDecodeTarget {
    fn new() -> Self {
        let root_ref = Ref::new();
        let mut strings = StringTable::default();
        let root_name = strings.intern("DataModel");
        let root_id = NodeId::from_index(0);
        let mut by_source_ref = HashMap::new();
        by_source_ref.insert(root_ref, root_id);

        Self {
            nodes: vec![Node {
                source_ref: root_ref,
                parent: None,
                name: root_name,
                class: "DataModel".into(),
                children: 0..0,
                properties: 0..0,
            }],
            children: Vec::new(),
            properties: Vec::new(),
            strings,
            by_source_ref,
            unique_ids: HashSet::new(),
        }
    }
}

impl rbx_binary::DecodeTarget for DiffDomDecodeTarget {
    type Instance = BinaryInstance;
    type Output = DiffDom;

    fn reserve(&mut self, additional: usize) {
        self.nodes.reserve(additional);
        self.children.reserve(additional);
        self.by_source_ref.reserve(additional);
    }

    fn root_ref(&self) -> Ref {
        self.nodes[0].source_ref
    }

    fn insert(&mut self, parent: Ref, mut instance: Self::Instance) {
        let parent_id = self.by_source_ref[&parent];
        let id = NodeId::from_index(self.nodes.len());

        let child_index =
            u32::try_from(self.children.len()).expect("DiffDom child arena exceeded u32::MAX");
        let parent_children = &mut self.nodes[parent_id.index()].children;
        if parent_children.start == parent_children.end {
            *parent_children = child_index..child_index + 1;
        } else {
            assert_eq!(
                parent_children.end, child_index,
                "binary decoder did not emit siblings contiguously"
            );
            parent_children.end += 1;
        }
        self.children.push(id);

        // InstanceBuilder-to-WeakDom collection gives the final duplicate
        // property precedence. Stable sorting plus replacement preserves that
        // behavior while producing the sorted range DiffDom needs.
        instance
            .properties
            .sort_by(|(left, _), (right, _)| left.as_str().cmp(right.as_str()));
        let properties_start =
            u32::try_from(self.properties.len()).expect("DiffDom property arena exceeded u32::MAX");
        for (name, value) in instance.properties {
            let has_current_property = self.properties.len() > properties_start as usize;
            let duplicate = if has_current_property {
                self.properties
                    .last_mut()
                    .filter(|property| property.name == name)
            } else {
                None
            };
            if let Some(property) = duplicate {
                property.value = value;
            } else {
                self.properties.push(Property { name, value });
            }
        }
        let properties_end =
            u32::try_from(self.properties.len()).expect("DiffDom property arena exceeded u32::MAX");

        if let Some(property) = self.properties[properties_start as usize..properties_end as usize]
            .iter_mut()
            .find(|property| property.name.as_str() == "UniqueId")
        {
            if let Variant::UniqueId(unique_id) = &mut property.value {
                if !self.unique_ids.insert(*unique_id) {
                    let replacement =
                        UniqueId::now().expect("system clock could not generate a UniqueId");
                    self.unique_ids.insert(replacement);
                    *unique_id = replacement;
                }
            }
        }

        self.nodes.push(Node {
            source_ref: instance.source_ref,
            parent: Some(parent_id),
            name: self.strings.intern(&instance.name),
            class: instance.class,
            children: 0..0,
            properties: properties_start..properties_end,
        });
        let replaced = self.by_source_ref.insert(instance.source_ref, id);
        assert!(replaced.is_none(), "binary decoder emitted a duplicate Ref");
    }

    fn finish(mut self) -> Self::Output {
        self.strings.finish();
        DiffDom {
            nodes: self.nodes,
            children: self.children,
            properties: self.properties,
            strings: self.strings,
            by_source_ref: self.by_source_ref,
        }
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DiffDomStats {
    pub(crate) nodes: usize,
    pub(crate) child_links: usize,
    pub(crate) properties: usize,
    pub(crate) unique_strings: usize,
}

impl DiffDom {
    /// Decode a binary Roblox model or place directly into compact storage.
    pub fn from_binary_reader<R: Read>(reader: R) -> Result<Self, rbx_binary::DecodeError> {
        rbx_binary::Deserializer::new().deserialize_into(reader, DiffDomDecodeTarget::new())
    }

    pub fn from_weak_dom(dom: &WeakDom) -> Self {
        // WeakDom descendants include the synthetic root as their first item.
        let source_refs: Vec<_> = dom
            .descendants()
            .map(|instance| instance.referent())
            .collect();
        debug_assert_eq!(source_refs.first().copied(), Some(dom.root_ref()));

        let by_source_ref: HashMap<Ref, NodeId> = source_refs
            .iter()
            .enumerate()
            .map(|(index, &referent)| (referent, NodeId::from_index(index)))
            .collect();

        let mut strings = StringTable::default();
        let mut nodes = Vec::with_capacity(source_refs.len());
        let mut children = Vec::with_capacity(source_refs.len().saturating_sub(1));
        let mut properties = Vec::new();

        for &source_ref in &source_refs {
            let instance = dom
                .get_by_ref(source_ref)
                .expect("source referent disappeared while compacting WeakDom");

            let children_start =
                u32::try_from(children.len()).expect("DiffDom child arena exceeded u32::MAX");
            children.extend(
                instance
                    .children()
                    .iter()
                    .map(|referent| by_source_ref[referent]),
            );
            let children_end =
                u32::try_from(children.len()).expect("DiffDom child arena exceeded u32::MAX");

            let mut instance_properties: Vec<_> = instance
                .properties
                .iter()
                .map(|(name, value)| (name.as_str(), value))
                .collect();
            instance_properties.sort_unstable_by_key(|(name, _)| *name);

            let properties_start =
                u32::try_from(properties.len()).expect("DiffDom property arena exceeded u32::MAX");
            for (name, value) in instance_properties {
                properties.push(Property {
                    name: (*name).into(),
                    value: value.clone(),
                });
            }
            let properties_end =
                u32::try_from(properties.len()).expect("DiffDom property arena exceeded u32::MAX");

            nodes.push(Node {
                source_ref,
                parent: by_source_ref.get(&instance.parent()).copied(),
                name: strings.intern(&instance.name),
                class: instance.class,
                children: children_start..children_end,
                properties: properties_start..properties_end,
            });
        }
        strings.finish();

        Self {
            nodes,
            children,
            properties,
            strings,
            by_source_ref,
        }
    }

    /// Consume a WeakDom and transfer property payloads into compact storage.
    ///
    /// This avoids cloning large binary properties while both
    /// representations are resident and is the preferred file-loading path.
    pub fn from_weak_dom_owned(dom: WeakDom) -> Self {
        let (root_ref, mut instances) = dom.into_raw();
        let mut source_refs = Vec::with_capacity(instances.len());
        let mut pending = vec![root_ref];
        while let Some(referent) = pending.pop() {
            source_refs.push(referent);
            let instance = instances
                .get(&referent)
                .expect("WeakDom child referent disappeared while compacting");
            pending.extend(instance.children().iter().rev().copied());
        }

        let by_source_ref: HashMap<Ref, NodeId> = source_refs
            .iter()
            .enumerate()
            .map(|(index, &referent)| (referent, NodeId::from_index(index)))
            .collect();
        let property_count = instances
            .values()
            .map(|instance| instance.properties.len())
            .sum();
        let mut strings = StringTable::default();
        let mut nodes = Vec::with_capacity(source_refs.len());
        let mut children = Vec::with_capacity(source_refs.len().saturating_sub(1));
        let mut properties = Vec::with_capacity(property_count);

        for source_ref in source_refs {
            let instance = instances
                .remove(&source_ref)
                .expect("source referent disappeared while consuming WeakDom");

            let children_start =
                u32::try_from(children.len()).expect("DiffDom child arena exceeded u32::MAX");
            children.extend(
                instance
                    .children()
                    .iter()
                    .map(|referent| by_source_ref[referent]),
            );
            let children_end =
                u32::try_from(children.len()).expect("DiffDom child arena exceeded u32::MAX");

            let parent = by_source_ref.get(&instance.parent()).copied();
            let name = strings.intern(&instance.name);
            let class = instance.class;
            let mut instance_properties: Vec<_> = instance.properties.into_iter().collect();
            instance_properties
                .sort_unstable_by(|(left, _), (right, _)| left.as_str().cmp(right.as_str()));

            let properties_start =
                u32::try_from(properties.len()).expect("DiffDom property arena exceeded u32::MAX");
            for (property_name, value) in instance_properties {
                properties.push(Property {
                    name: property_name,
                    value,
                });
            }
            let properties_end =
                u32::try_from(properties.len()).expect("DiffDom property arena exceeded u32::MAX");

            nodes.push(Node {
                source_ref,
                parent,
                name,
                class,
                children: children_start..children_end,
                properties: properties_start..properties_end,
            });
        }
        strings.finish();

        Self {
            nodes,
            children,
            properties,
            strings,
            by_source_ref,
        }
    }

    pub(crate) fn root_id(&self) -> NodeId {
        NodeId(0)
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub(crate) fn node(&self, id: NodeId) -> DiffNode<'_> {
        assert!(id.index() < self.nodes.len(), "invalid DiffDom NodeId");
        DiffNode { dom: self, id }
    }

    pub(crate) fn id_from_source_ref(&self, referent: Ref) -> Option<NodeId> {
        self.by_source_ref.get(&referent).copied()
    }

    #[cfg(test)]
    pub(crate) fn nodes(&self) -> impl ExactSizeIterator<Item = DiffNode<'_>> {
        (0..self.nodes.len()).map(|index| self.node(NodeId::from_index(index)))
    }

    #[cfg(test)]
    pub(crate) fn stats(&self) -> DiffDomStats {
        DiffDomStats {
            nodes: self.nodes.len(),
            child_links: self.children.len(),
            properties: self.properties.len(),
            unique_strings: self.strings.values.len(),
        }
    }

    /// Materialize this compact snapshot as a WeakDom.
    ///
    /// Diffing does not use this path. It exists as a parity oracle while the
    /// compact comparison engine is introduced and will later be the fallback
    /// for APIs that explicitly require a mutable Roblox DOM.
    #[cfg(test)]
    fn to_weak_dom(&self) -> WeakDom {
        WeakDom::new(self.builder(self.root_id()))
    }

    #[cfg(test)]
    fn builder(&self, id: NodeId) -> InstanceBuilder {
        let node = self.node(id);
        let mut builder =
            InstanceBuilder::with_property_capacity(node.class(), node.property_count())
                .with_referent(node.source_ref())
                .with_name(node.name());
        for (name, value) in node.properties() {
            builder.add_property(name, value.clone());
        }
        for child in node.children() {
            builder.add_child(self.builder(child));
        }
        builder
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DiffNode<'a> {
    dom: &'a DiffDom,
    id: NodeId,
}

impl<'a> DiffNode<'a> {
    fn raw(self) -> &'a Node {
        &self.dom.nodes[self.id.index()]
    }

    #[cfg(test)]
    pub(crate) fn id(self) -> NodeId {
        self.id
    }

    pub(crate) fn source_ref(self) -> Ref {
        self.raw().source_ref
    }

    pub(crate) fn parent(self) -> Option<NodeId> {
        self.raw().parent
    }

    pub(crate) fn name(self) -> &'a str {
        self.dom.strings.resolve(self.raw().name)
    }

    pub(crate) fn class(self) -> &'a str {
        self.raw().class.as_str()
    }

    #[cfg(test)]
    pub(crate) fn children(self) -> impl ExactSizeIterator<Item = NodeId> + 'a {
        let range = self.raw().children.clone();
        self.dom.children[range.start as usize..range.end as usize]
            .iter()
            .copied()
    }

    pub(crate) fn property(self, name: &str) -> Option<&'a Variant> {
        let range = self.raw().properties.clone();
        let properties = &self.dom.properties[range.start as usize..range.end as usize];
        properties
            .binary_search_by(|property| property.name.as_str().cmp(name))
            .ok()
            .map(|index| &properties[index].value)
    }

    #[cfg(test)]
    pub(crate) fn property_count(self) -> usize {
        self.raw().properties.len()
    }

    #[cfg(test)]
    pub(crate) fn properties(self) -> impl ExactSizeIterator<Item = (&'a str, &'a Variant)> + 'a {
        let range = self.raw().properties.clone();
        self.dom.properties[range.start as usize..range.end as usize]
            .iter()
            .map(|property| (property.name.as_str(), &property.value))
    }
}

/// Read-only instance access shared by WeakDom and DiffDom algorithms.
#[derive(Clone, Copy)]
pub(crate) enum InstanceView<'a> {
    Weak(&'a Instance),
    Compact(DiffNode<'a>),
}

impl<'a> InstanceView<'a> {
    pub(crate) fn parent(self) -> Ref {
        match self {
            Self::Weak(instance) => instance.parent(),
            Self::Compact(instance) => instance
                .parent()
                .map(|parent| instance.dom.node(parent).source_ref())
                .unwrap_or_else(Ref::none),
        }
    }

    pub(crate) fn name(self) -> &'a str {
        match self {
            Self::Weak(instance) => &instance.name,
            Self::Compact(instance) => instance.name(),
        }
    }

    pub(crate) fn class(self) -> &'a str {
        match self {
            Self::Weak(instance) => instance.class.as_str(),
            Self::Compact(instance) => instance.class(),
        }
    }

    pub(crate) fn property(self, name: &str) -> Option<&'a Variant> {
        match self {
            Self::Weak(instance) => instance.properties.get(&name.into()),
            Self::Compact(instance) => instance.property(name),
        }
    }

    pub(crate) fn properties(self) -> PropertyIter<'a> {
        match self {
            Self::Weak(instance) => {
                PropertyIter(PropertyIterInner::Weak(instance.properties.iter()))
            }
            Self::Compact(instance) => {
                let range = instance.raw().properties.clone();
                PropertyIter(PropertyIterInner::Compact {
                    inner: instance.dom.properties[range.start as usize..range.end as usize].iter(),
                })
            }
        }
    }

    pub(crate) fn children(self) -> ChildRefIter<'a> {
        match self {
            Self::Weak(instance) => ChildRefIter::Weak(instance.children().iter()),
            Self::Compact(instance) => {
                let range = instance.raw().children.clone();
                ChildRefIter::Compact {
                    dom: instance.dom,
                    inner: instance.dom.children[range.start as usize..range.end as usize].iter(),
                }
            }
        }
    }
}

pub(crate) struct PropertyIter<'a>(PropertyIterInner<'a>);

enum PropertyIterInner<'a> {
    Weak(std::collections::hash_map::Iter<'a, Ustr, Variant>),
    Compact { inner: slice::Iter<'a, Property> },
}

impl<'a> Iterator for PropertyIter<'a> {
    type Item = (&'a str, &'a Variant);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            PropertyIterInner::Weak(inner) => {
                inner.next().map(|(name, value)| (name.as_str(), value))
            }
            PropertyIterInner::Compact { inner } => inner
                .next()
                .map(|property| (property.name.as_str(), &property.value)),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            PropertyIterInner::Weak(inner) => inner.size_hint(),
            PropertyIterInner::Compact { inner } => inner.size_hint(),
        }
    }
}

impl ExactSizeIterator for PropertyIter<'_> {}

pub(crate) enum ChildRefIter<'a> {
    Weak(slice::Iter<'a, Ref>),
    Compact {
        dom: &'a DiffDom,
        inner: slice::Iter<'a, NodeId>,
    },
}

impl<'a> Iterator for ChildRefIter<'a> {
    type Item = Ref;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Weak(inner) => inner.next().copied(),
            Self::Compact { dom, inner } => inner.next().map(|id| dom.node(*id).source_ref()),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Weak(inner) => inner.size_hint(),
            Self::Compact { inner, .. } => inner.size_hint(),
        }
    }
}

impl DoubleEndedIterator for ChildRefIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            Self::Weak(inner) => inner.next_back().copied(),
            Self::Compact { dom, inner } => inner.next_back().map(|id| dom.node(*id).source_ref()),
        }
    }
}

impl ExactSizeIterator for ChildRefIter<'_> {}

/// Minimal immutable DOM contract used by hashing, matching, and diffing.
pub(crate) trait DomView {
    fn root_ref(&self) -> Ref;
    fn get_by_ref(&self, referent: Ref) -> Option<InstanceView<'_>>;
}

/// Narrow mutation contract needed by model-frame canonicalization.
///
/// Frame factoring only rewrites existing world-space properties; it does not
/// need general tree editing. Keeping that distinction lets comparison stay
/// in compact storage without growing a second mutable DOM abstraction.
pub(crate) trait DomViewMut: DomView {
    fn set_existing_property(&mut self, referent: Ref, name: &str, value: Variant) -> bool;
    fn as_view(&self) -> &dyn DomView;
}

impl DomView for WeakDom {
    fn root_ref(&self) -> Ref {
        WeakDom::root_ref(self)
    }

    fn get_by_ref(&self, referent: Ref) -> Option<InstanceView<'_>> {
        WeakDom::get_by_ref(self, referent).map(InstanceView::Weak)
    }
}

impl DomViewMut for WeakDom {
    fn set_existing_property(&mut self, referent: Ref, name: &str, value: Variant) -> bool {
        let Some(instance) = WeakDom::get_by_ref_mut(self, referent) else {
            return false;
        };
        let key: Ustr = name.into();
        if !instance.properties.contains_key(&key) {
            return false;
        }
        instance.properties.insert(key, value);
        true
    }

    fn as_view(&self) -> &dyn DomView {
        self
    }
}

impl DomView for DiffDom {
    fn root_ref(&self) -> Ref {
        self.node(self.root_id()).source_ref()
    }

    fn get_by_ref(&self, referent: Ref) -> Option<InstanceView<'_>> {
        self.id_from_source_ref(referent)
            .map(|id| InstanceView::Compact(self.node(id)))
    }
}

impl DomViewMut for DiffDom {
    fn set_existing_property(&mut self, referent: Ref, name: &str, value: Variant) -> bool {
        let Some(id) = self.id_from_source_ref(referent) else {
            return false;
        };
        let range = self.nodes[id.index()].properties.clone();
        let properties = &self.properties[range.start as usize..range.end as usize];
        let Ok(index) = properties.binary_search_by(|property| property.name.as_str().cmp(name))
        else {
            return false;
        };
        self.properties[range.start as usize + index].value = value;
        true
    }

    fn as_view(&self) -> &dyn DomView {
        self
    }
}

pub(crate) struct DescendantRefs<'a> {
    dom: &'a dyn DomView,
    pending: Vec<Ref>,
}

impl<'a> DescendantRefs<'a> {
    pub(crate) fn new(dom: &'a dyn DomView) -> Self {
        Self {
            dom,
            pending: vec![dom.root_ref()],
        }
    }
}

impl Iterator for DescendantRefs<'_> {
    type Item = Ref;

    fn next(&mut self) -> Option<Self::Item> {
        let referent = self.pending.pop()?;
        let instance = self
            .dom
            .get_by_ref(referent)
            .expect("DomView child referent did not resolve");
        self.pending.extend(instance.children().rev());
        Some(referent)
    }
}

#[cfg(test)]
mod tests {
    use super::DiffDom;
    use crate::diff::{compute_diff_with_identity, DiffConfig};
    use crate::diff_doms;
    use crate::hash::LazyHashCache;
    use rbx_dom_weak::{InstanceBuilder, WeakDom};
    use rbx_types::{Ref, Variant};

    fn fixture() -> WeakDom {
        let target = InstanceBuilder::new("Part")
            .with_name("Repeated")
            .with_property("Anchored", Variant::Bool(true));
        let target_ref = target.referent();
        WeakDom::new(
            InstanceBuilder::new("DataModel")
                .with_name("root")
                .with_child(
                    InstanceBuilder::new("Folder")
                        .with_name("Repeated")
                        .with_child(target)
                        .with_child(
                            InstanceBuilder::new("ObjectValue")
                                .with_name("Repeated")
                                .with_property("Value", Variant::Ref(target_ref)),
                        ),
                )
                .with_child(
                    InstanceBuilder::new("Folder")
                        .with_name("Repeated")
                        .with_property("Optional", Variant::Ref(Ref::none())),
                ),
        )
    }

    #[test]
    fn compacts_tree_into_dense_arenas() {
        let dom = fixture();
        let compact = DiffDom::from_weak_dom(&dom);
        let stats = compact.stats();

        assert_eq!(stats.nodes, dom.descendants().count());
        assert_eq!(stats.child_links, stats.nodes - 1);
        assert!(stats.properties >= 3);
        assert!(
            stats.unique_strings < stats.nodes * 2,
            "repeated names and classes should be interned: {stats:?}"
        );
        assert!(!compact.is_empty());
        assert_eq!(compact.len(), stats.nodes);
    }

    #[test]
    fn preserves_referents_properties_and_child_order_when_materialized() {
        let dom = fixture();
        let compact = DiffDom::from_weak_dom(&dom);
        for node in compact.nodes() {
            assert_eq!(
                compact.id_from_source_ref(node.source_ref()),
                Some(node.id())
            );
        }

        let materialized = compact.to_weak_dom();
        assert!(diff_doms(&dom, &materialized).is_empty());
    }

    #[test]
    fn binary_decode_target_matches_weak_dom_compaction() {
        let source = fixture();
        let mut encoded = Vec::new();
        rbx_binary::to_writer(&mut encoded, &source, source.root().children()).unwrap();

        let weak = rbx_binary::from_reader(encoded.as_slice()).unwrap();
        let via_weak = DiffDom::from_weak_dom_owned(weak);
        let direct = DiffDom::from_binary_reader(encoded.as_slice()).unwrap();

        assert_eq!(direct.stats(), via_weak.stats());
        assert!(
            diff_doms(&via_weak.to_weak_dom(), &direct.to_weak_dom()).is_empty(),
            "direct binary decoding must preserve names, classes, properties, references, and topology"
        );
    }

    #[test]
    fn compact_and_weak_views_produce_identical_diffs() {
        let old = WeakDom::new(
            InstanceBuilder::new("DataModel")
                .with_name("root")
                .with_child(
                    InstanceBuilder::new("Folder").with_name("A").with_child(
                        InstanceBuilder::new("Part")
                            .with_name("Moved")
                            .with_property("Transparency", Variant::Float32(0.0)),
                    ),
                )
                .with_child(InstanceBuilder::new("Folder").with_name("B"))
                .with_child(InstanceBuilder::new("Part").with_name("Removed")),
        );
        let new = WeakDom::new(
            InstanceBuilder::new("DataModel")
                .with_name("root")
                .with_child(InstanceBuilder::new("Folder").with_name("A"))
                .with_child(
                    InstanceBuilder::new("Folder").with_name("B").with_child(
                        InstanceBuilder::new("Part")
                            .with_name("Moved")
                            .with_property("Transparency", Variant::Float32(0.5)),
                    ),
                )
                .with_child(InstanceBuilder::new("SpotLight").with_name("Added")),
        );
        let config = DiffConfig::default();
        let weak_old_hashes = LazyHashCache::new(&old);
        let weak_new_hashes = LazyHashCache::new(&new);
        let weak = compute_diff_with_identity(
            &old,
            &new,
            &weak_old_hashes,
            &weak_new_hashes,
            &config,
            None,
        );

        let compact_old = DiffDom::from_weak_dom(&old);
        let compact_new = DiffDom::from_weak_dom(&new);
        let compact_old_hashes = LazyHashCache::new_view(&compact_old);
        let compact_new_hashes = LazyHashCache::new_view(&compact_new);
        let compact = compute_diff_with_identity(
            &compact_old,
            &compact_new,
            &compact_old_hashes,
            &compact_new_hashes,
            &config,
            None,
        );

        assert_eq!(
            serde_json::to_value(&compact).unwrap(),
            serde_json::to_value(&weak).unwrap()
        );
    }
}
