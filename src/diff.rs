//! Two-phase tree diffing.
//!
//! Phase 1: Recursively match instances across both DOMs, building a global ref mapping.
//! Phase 2: Compare properties using the ref mapping for Ref property comparison.
//! Ref properties pointing to matched instances are considered equal (same logical target).
//! Identity discovery is complete, so an unmapped target is a different logical instance.

use crate::diff_dom::{DomView, InstanceView};
use crate::edit_script::{Anchor, EditOp, SemanticChangeSet};
use crate::hash::LazyHashCache;
use crate::match_instances::{
    get_instance_path, get_instance_path_segments, join_instance_path,
};
use crate::property_semantics::get_authored_properties;
use crate::value_compare::non_ref_variants_equal;
use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::{CFrame, ContentType, Variant};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// A single difference found between two DOMs.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DiffEntry {
    /// Instance was added (only in new DOM)
    Added {
        new_ref: String,
        path: String,
        /// Structured presentation path; omitted from machine output.
        #[serde(skip)]
        path_segments: Vec<(Ref, String)>,
        class: String,
    },
    /// Instance was removed (only in old DOM)
    Removed {
        old_ref: String,
        path: String,
        /// Structured presentation path; omitted from machine output.
        #[serde(skip)]
        path_segments: Vec<(Ref, String)>,
        class: String,
    },
    /// Instance was modified (properties changed)
    Modified {
        old_ref: String,
        new_ref: String,
        path: String,
        /// Structured presentation path; omitted from machine output.
        #[serde(skip)]
        path_segments: Vec<(Ref, String)>,
        class: String,
        property_changes: Vec<PropertyChange>,
    },
    /// Instance was moved to a different parent (same logical instance).
    /// `path` is the new location. Property edits remain separate `Modified`
    /// entries so every diff entry represents one primitive operation.
    Moved {
        old_ref: String,
        new_ref: String,
        old_path: String,
        path: String,
        /// Structured presentation path; omitted from machine output.
        #[serde(skip)]
        path_segments: Vec<(Ref, String)>,
        class: String,
    },
    /// A Model boundary and its world-space descendants were pivoted together.
    /// This is an inferred rigid transform, not a Roblox property change;
    /// `WorldPivotData` edits that differ from the content still appear as
    /// ordinary property changes.
    Pivoted {
        old_ref: String,
        new_ref: String,
        path: String,
        /// Structured presentation path; omitted from machine output.
        #[serde(skip)]
        path_segments: Vec<(Ref, String)>,
        class: String,
        /// Stable top-down order among pivot operations.
        order: usize,
        /// `order` of the nearest ancestor frame, when nested.
        parent_order: Option<usize>,
        /// Local transform relative to `parent_order`, or world-space when
        /// this entry has no participating parent.
        delta: CFrameValue,
    },
}

/// Roblox's canonical CFrame component representation:
/// x, y, z, followed by the row-major 3×3 rotation matrix.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(transparent)]
pub struct CFrameValue {
    pub components: [f32; 12],
}

impl From<CFrame> for CFrameValue {
    fn from(value: CFrame) -> Self {
        Self {
            components: [
                value.position.x,
                value.position.y,
                value.position.z,
                value.orientation.x.x,
                value.orientation.x.y,
                value.orientation.x.z,
                value.orientation.y.x,
                value.orientation.y.y,
                value.orientation.y.z,
                value.orientation.z.x,
                value.orientation.z.y,
                value.orientation.z.z,
            ],
        }
    }
}

/// A property change within an instance.
#[derive(Debug, Clone, Serialize)]
pub struct PropertyChange {
    pub name: String,
    pub old_value: Option<PropertyValue>,
    pub new_value: Option<PropertyValue>,
}

/// Typed property value for structured output.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum PropertyValue {
    Nil,
    Bool {
        value: bool,
    },
    Int32 {
        value: i32,
    },
    Int64 {
        value: i64,
    },
    Float32 {
        value: f32,
    },
    Float64 {
        value: f64,
    },
    String {
        value: String,
    },
    BinaryString {
        len: usize,
    },
    Ref {
        value: String,
    },
    Vector2 {
        x: f32,
        y: f32,
    },
    Vector3 {
        x: f32,
        y: f32,
        z: f32,
    },
    CFrame(CFrameValue),
    Color3 {
        r: f32,
        g: f32,
        b: f32,
    },
    BrickColor {
        value: u16,
    },
    Enum {
        value: u32,
    },
    UDim {
        scale: f32,
        offset: i32,
    },
    UDim2 {
        x_scale: f32,
        x_offset: i32,
        y_scale: f32,
        y_offset: i32,
    },
    NumberRange {
        min: f32,
        max: f32,
    },
    NumberSequence {
        keypoints: Vec<NumberKeypoint>,
    },
    ColorSequence {
        keypoints: Vec<ColorKeypoint>,
    },
    Rect {
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
    },
    Other {
        type_name: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct NumberKeypoint {
    pub time: f32,
    pub value: f32,
    pub envelope: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColorKeypoint {
    pub time: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

/// Configuration for diffing.
pub struct DiffConfig {
    /// Properties to ignore when comparing
    pub ignore_properties: HashSet<String>,
}

impl Default for DiffConfig {
    fn default() -> Self {
        let mut ignore = HashSet::new();
        // Always ignore non-deterministic properties
        ignore.insert("UniqueId".to_string());
        ignore.insert("HistoryId".to_string());
        ignore.insert("SourceAssetId".to_string());
        Self {
            ignore_properties: ignore,
        }
    }
}

/// Compute a presentation diff through the shared semantic change planner.
///
/// The cache parameters remain for API compatibility; semantic planning owns
/// the caches used by matching so diff and merge cannot choose different
/// identities.
pub fn compute_diff(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    _old_hashes: &LazyHashCache,
    _new_hashes: &LazyHashCache,
    config: &DiffConfig,
) -> Vec<DiffEntry> {
    let changes =
        crate::edit_script::compute_semantic_changes_with_identity(old_dom, new_dom, config, None);
    semantic_changes_to_diff(old_dom, new_dom, &changes)
}

// ============================================================================
// Property comparison
// ============================================================================

/// A property difference carrying raw Variants — the applicable form used by
/// the edit-script layer. Name changes are NOT included (Instance.name lives
/// outside the property map; callers handle it separately).
///
/// Attributes and Tags are GRANULAR: container properties expand into one
/// change per key, named `Attributes.<key>` / `Tags.<tag>`, so independent
/// keys diff, merge, and conflict independently (two branches touching
/// different attributes on one instance compose instead of conflicting).
/// Roblox property names can't contain dots, so the namespace is unambiguous.
#[derive(Debug, Clone)]
pub(crate) struct RawPropertyChange {
    pub name: String,
    pub old: Option<Variant>,
    pub new: Option<Variant>,
}

/// Equality for attribute values (no Refs possible inside Attributes; float
/// tolerance matches variants_equal).
fn attr_string_bytes(value: &Variant) -> Option<&[u8]> {
    match value {
        Variant::String(value) => Some(value.as_bytes()),
        Variant::BinaryString(value) => Some(value.as_ref()),
        _ => None,
    }
}

pub(crate) fn attr_value_eq(a: &Variant, b: &Variant) -> bool {
    if let (Some(a), Some(b)) = (attr_string_bytes(a), attr_string_bytes(b)) {
        return a == b;
    }
    match (a, b) {
        (Variant::Float32(x), Variant::Float32(y)) => {
            x == y || (x.is_nan() && y.is_nan()) || (x - y).abs() < 0.01
        }
        (Variant::Float64(x), Variant::Float64(y)) => {
            x == y || (x.is_nan() && y.is_nan()) || (x - y).abs() < 0.01
        }
        _ => a == b,
    }
}

/// Expand an Attributes or Tags change into per-key granular changes.
/// An empty container on one side only produces nothing — semantically
/// identical to the property being absent (kills Studio's habit of adding
/// empty Attributes containers on save).
fn expand_container_changes(
    changes: &mut Vec<RawPropertyChange>,
    container_name: &str,
    old: Option<&Variant>,
    new: Option<&Variant>,
) {
    match container_name {
        "Attributes" => {
            let empty = rbx_types::Attributes::new();
            let old_attrs = match old {
                Some(Variant::Attributes(a)) => a,
                _ => &empty,
            };
            let new_attrs = match new {
                Some(Variant::Attributes(a)) => a,
                _ => &empty,
            };

            let mut keys: Vec<&str> = old_attrs
                .iter()
                .map(|(k, _)| k.as_str())
                .chain(new_attrs.iter().map(|(k, _)| k.as_str()))
                .collect();
            keys.sort_unstable();
            keys.dedup();

            for key in keys {
                let name = format!("Attributes.{key}");
                let old_value = old_attrs.get(key);
                let new_value = new_attrs.get(key);
                let changed = match (old_value, new_value) {
                    (Some(a), Some(b)) => !attr_value_eq(a, b),
                    (None, None) => false,
                    _ => true,
                };
                if changed {
                    changes.push(RawPropertyChange {
                        name,
                        old: old_value.cloned(),
                        new: new_value.cloned(),
                    });
                }
            }
        }
        "Tags" => {
            let empty = rbx_types::Tags::new();
            let old_tags = match old {
                Some(Variant::Tags(t)) => t,
                _ => &empty,
            };
            let new_tags = match new {
                Some(Variant::Tags(t)) => t,
                _ => &empty,
            };

            let mut tags: Vec<&str> = old_tags.iter().chain(new_tags.iter()).collect();
            tags.sort_unstable();
            tags.dedup();

            for tag in tags {
                let name = format!("Tags.{tag}");
                let in_old = old_tags.iter().any(|t| t == tag);
                let in_new = new_tags.iter().any(|t| t == tag);
                if in_old != in_new {
                    changes.push(RawPropertyChange {
                        name,
                        old: if in_old {
                            Some(Variant::String(tag.to_string()))
                        } else {
                            None
                        },
                        new: if in_new {
                            Some(Variant::String(tag.to_string()))
                        } else {
                            None
                        },
                    });
                }
            }
        }
        _ => unreachable!("expand_container_changes only handles Attributes/Tags"),
    }
}

/// Compare properties between two matched instances at the Variant level.
/// Uses the ref mapping for Ref property comparison. Filters out
/// non-reflected, non-serializable, and default-valued properties to avoid
/// false positives from serialization differences.
pub(crate) fn raw_property_changes(
    old_dom: &dyn DomView,
    new_dom: &dyn DomView,
    old_ref: Ref,
    new_ref: Ref,
    config: &DiffConfig,
    ref_mapping: &HashMap<Ref, Ref>,
) -> Vec<RawPropertyChange> {
    let old_inst = old_dom.get_by_ref(old_ref).unwrap();
    let new_inst = new_dom.get_by_ref(new_ref).unwrap();

    let database = rbx_reflection_database::get().unwrap();
    let class_name = new_inst.class();
    let defaults = database
        .classes
        .get(class_name)
        .map(|cd| &cd.default_properties);

    let mut changes = Vec::new();
    let mut visited = HashSet::new();

    // Check properties in new instance
    for (name, new_value) in new_inst.properties() {
        if config.ignore_properties.contains(name) {
            continue;
        }
        if !should_compare_property(class_name, name) {
            continue;
        }
        visited.insert(name.to_string());

        let old_value = old_inst.property(name);

        // Container properties expand into per-key granular changes
        if name == "Attributes" || name == "Tags" {
            expand_container_changes(&mut changes, name, old_value, Some(new_value));
            continue;
        }

        match old_value {
            Some(old_value) => {
                if !variants_equal(old_dom, new_dom, old_value, new_value, ref_mapping) {
                    changes.push(RawPropertyChange {
                        name: name.to_string(),
                        old: Some(old_value.clone()),
                        new: Some(new_value.clone()),
                    });
                }
            }
            None => {
                // Property only in new — skip if it's just a default value
                if is_default_value(defaults, name, new_value) {
                    continue;
                }
                // A nil Ref on one side only is semantically "unset" — not a change
                if matches!(new_value, Variant::Ref(r) if r.is_none()) {
                    continue;
                }
                changes.push(RawPropertyChange {
                    name: name.to_string(),
                    old: None,
                    new: Some(new_value.clone()),
                });
            }
        }
    }

    // Check for removed properties
    for (name, old_value) in old_inst.properties() {
        if config.ignore_properties.contains(name) {
            continue;
        }
        if !should_compare_property(old_inst.class(), name) {
            continue;
        }
        if !visited.contains(name) {
            // Container property removed entirely: granular removals per key
            if name == "Attributes" || name == "Tags" {
                expand_container_changes(&mut changes, name, Some(old_value), None);
                continue;
            }
            // Property only in old — skip if it's just a default value
            if is_default_value(defaults, name, old_value) {
                continue;
            }
            // A nil Ref on one side only is semantically "unset" — not a change
            if matches!(old_value, Variant::Ref(r) if r.is_none()) {
                continue;
            }
            changes.push(RawPropertyChange {
                name: name.to_string(),
                old: Some(old_value.clone()),
                new: None,
            });
        }
    }

    changes
}

/// Project storage-independent semantic changes into the presentation diff.
///
/// The change set is authoritative: this function does not rematch instances
/// or compare properties again. Changes below a moved root are deferred until
/// after that root's `Moved` row, preserving the tree-oriented output order.
fn presentation_path(
    dom: &dyn DomView,
    referent: Ref,
    canonical_refs: Option<&HashMap<Ref, Ref>>,
) -> (String, Vec<(Ref, String)>) {
    let mut segments = get_instance_path_segments(dom, referent);
    if let Some(canonical_refs) = canonical_refs {
        for (referent, _) in &mut segments {
            if let Some(canonical) = canonical_refs.get(referent) {
                *referent = *canonical;
            }
        }
    }
    let path = join_instance_path(&segments);
    (path, segments)
}

pub(crate) fn semantic_changes_to_diff(
    old_dom: &dyn DomView,
    new_dom: &dyn DomView,
    changes: &SemanticChangeSet,
) -> Vec<DiffEntry> {
    let mut result = Vec::with_capacity(changes.pivots.len() + changes.ops.len());
    for pivot in &changes.pivots {
        let Some(instance) = new_dom.get_by_ref(pivot.side_ref) else {
            continue;
        };
        let (path, path_segments) = presentation_path(
            new_dom,
            pivot.side_ref,
            Some(&changes.identity.reverse_matched),
        );
        result.push(DiffEntry::Pivoted {
            old_ref: pivot.target_ref.to_string(),
            new_ref: pivot.side_ref.to_string(),
            path,
            path_segments,
            class: instance.class().to_string(),
            order: pivot.order,
            parent_order: pivot.parent_order,
            delta: pivot.delta.into(),
        });
    }

    let mut modifications: HashMap<Ref, Vec<PropertyChange>> = HashMap::new();

    for op in &changes.ops {
        match op {
            EditOp::SetName { old_ref, name } => {
                let Some(old_instance) = old_dom.get_by_ref(*old_ref) else {
                    continue;
                };
                modifications
                    .entry(*old_ref)
                    .or_default()
                    .push(PropertyChange {
                        name: "Name".to_string(),
                        old_value: Some(PropertyValue::String {
                            value: old_instance.name().to_string(),
                        }),
                        new_value: Some(PropertyValue::String {
                            value: name.clone(),
                        }),
                    });
            }
            EditOp::SetProperty {
                old_ref,
                name,
                old_value,
                value,
            } => {
                let attribute = name.starts_with("Attributes.");
                let display = |value: &Variant| {
                    if attribute {
                        attribute_variant_to_property_value(value)
                    } else {
                        variant_to_property_value(value)
                    }
                };
                modifications
                    .entry(*old_ref)
                    .or_default()
                    .push(PropertyChange {
                        name: name.clone(),
                        old_value: old_value.as_ref().map(display),
                        new_value: value.as_ref().map(display),
                    });
            }
            _ => {}
        }
    }

    let moved_ancestor = |mut referent: Ref| {
        while let Some(instance) = old_dom.get_by_ref(referent) {
            referent = instance.parent();
            if changes.identity.moved_old.contains(&referent) {
                return Some(referent);
            }
        }
        None
    };
    let mut deferred: HashMap<Ref, Vec<DiffEntry>> = HashMap::new();
    let mut emitted_modifications = HashSet::new();
    let push = |entry: DiffEntry,
                owner: Option<Ref>,
                result: &mut Vec<DiffEntry>,
                deferred: &mut HashMap<Ref, Vec<DiffEntry>>| {
        if let Some(owner) = owner {
            deferred.entry(owner).or_default().push(entry);
        } else {
            result.push(entry);
        }
    };

    for op in &changes.ops {
        match op {
            EditOp::RemoveSubtree { old_ref } => {
                let Some(instance) = old_dom.get_by_ref(*old_ref) else {
                    continue;
                };
                let (path, path_segments) = presentation_path(old_dom, *old_ref, None);
                push(
                    DiffEntry::Removed {
                        old_ref: old_ref.to_string(),
                        path,
                        path_segments,
                        class: instance.class().to_string(),
                    },
                    moved_ancestor(*old_ref),
                    &mut result,
                    &mut deferred,
                );
            }
            EditOp::AddSubtree { parent, new_ref } => {
                let Some(instance) = new_dom.get_by_ref(*new_ref) else {
                    continue;
                };
                let (path, path_segments) = presentation_path(
                    new_dom,
                    *new_ref,
                    Some(&changes.identity.reverse_matched),
                );
                let owner = match parent {
                    Anchor::Old(parent) => {
                        if changes.identity.moved_old.contains(parent) {
                            Some(*parent)
                        } else {
                            moved_ancestor(*parent)
                        }
                    }
                    Anchor::Added(_) => None,
                };
                push(
                    DiffEntry::Added {
                        new_ref: new_ref.to_string(),
                        path,
                        path_segments,
                        class: instance.class().to_string(),
                    },
                    owner,
                    &mut result,
                    &mut deferred,
                );
            }
            EditOp::SetName { old_ref, .. } | EditOp::SetProperty { old_ref, .. } => {
                // A moved instance's property edits are emitted immediately
                // after its primitive Moved entry below.
                if changes.identity.moved_old.contains(old_ref) {
                    continue;
                }
                if !emitted_modifications.insert(*old_ref) {
                    continue;
                }
                let Some(&new_ref) = changes.identity.matched.get(old_ref) else {
                    continue;
                };
                let Some(instance) = new_dom.get_by_ref(new_ref) else {
                    continue;
                };
                let property_changes = modifications.remove(old_ref).unwrap_or_default();
                if property_changes.is_empty() {
                    continue;
                }
                let (path, path_segments) = presentation_path(
                    new_dom,
                    new_ref,
                    Some(&changes.identity.reverse_matched),
                );
                push(
                    DiffEntry::Modified {
                        old_ref: old_ref.to_string(),
                        new_ref: new_ref.to_string(),
                        path,
                        path_segments,
                        class: instance.class().to_string(),
                        property_changes,
                    },
                    moved_ancestor(*old_ref),
                    &mut result,
                    &mut deferred,
                );
            }
            EditOp::Move { .. } => {}
        }
    }

    for (old_ref, new_ref) in changes.identity.moves.iter() {
        let Some(instance) = new_dom.get_by_ref(*new_ref) else {
            continue;
        };
        let (path, path_segments) = presentation_path(
            new_dom,
            *new_ref,
            Some(&changes.identity.reverse_matched),
        );
        let property_changes = modifications.remove(old_ref).unwrap_or_default();
        result.push(DiffEntry::Moved {
            old_ref: old_ref.to_string(),
            new_ref: new_ref.to_string(),
            old_path: get_instance_path(old_dom, *old_ref),
            path: path.clone(),
            path_segments: path_segments.clone(),
            class: instance.class().to_string(),
        });
        if !property_changes.is_empty() {
            result.push(DiffEntry::Modified {
                old_ref: old_ref.to_string(),
                new_ref: new_ref.to_string(),
                path,
                path_segments,
                class: instance.class().to_string(),
                property_changes,
            });
        }
        if let Some(mut descendants) = deferred.remove(old_ref) {
            result.append(&mut descendants);
        }
    }

    // Conservatively retain any deferred entries if malformed identity data
    // omitted their owning move.
    for mut entries in deferred.into_values() {
        result.append(&mut entries);
    }
    result
}

/// Check if a property should be compared (is meaningful for diffing).
/// Uses the shared authored-property policy from property_semantics.rs.
fn should_compare_property(class_name: &str, prop_name: &str) -> bool {
    get_authored_properties(class_name).contains(prop_name)
}

/// Studio serializes every service under the DataModel root on save, plus
/// internals like FilteredSelection (class "Instance"). Additions/removals of
/// those at the root are serialization noise, not changes. The check is
/// class-based, NOT position-based: rbx_binary gives model files a
/// DataModel-class root too, and top-level model content (Parts, Models, ...)
/// must still diff normally.
pub(crate) fn is_studio_artifact(
    dom: &dyn DomView,
    parent_ref: Ref,
    inst: InstanceView<'_>,
) -> bool {
    let parent = match dom.get_by_ref(parent_ref) {
        Some(p) => p,
        None => return false,
    };
    // The rbx-diff conflict container is tool metadata, never content
    if inst.name() == crate::conflict_file::CONTAINER_NAME {
        return true;
    }
    if parent_ref != dom.root_ref() || parent.class() != "DataModel" {
        return false;
    }
    let class_name = inst.class();
    if class_name == "Instance" {
        return true; // Studio's FilteredSelection objects
    }
    let database = rbx_reflection_database::get().unwrap();
    database
        .classes
        .get(class_name)
        .map(|cd| cd.tags.contains(&rbx_reflection::ClassTag::Service))
        .unwrap_or(false)
}

/// Check if a value matches the reflection database default for this property.
pub(crate) fn is_default_value(
    defaults: Option<&std::collections::HashMap<&str, Variant>>,
    name: &str,
    value: &Variant,
) -> bool {
    if let Some(defaults) = defaults {
        if let Some(default_value) = defaults.get(name) {
            return value == default_value;
        }
    }
    false
}

/// Compare two variants for equality. Ref values use the global identity map;
/// other values use the shared strict floating-point policy above.
fn variants_equal(
    old_dom: &dyn DomView,
    new_dom: &dyn DomView,
    a: &Variant,
    b: &Variant,
    ref_mapping: &HashMap<Ref, Ref>,
) -> bool {
    match (a, b) {
        (Variant::Ref(old_target), Variant::Ref(new_target)) => {
            refs_equal(old_dom, new_dom, *old_target, *new_target, ref_mapping)
        }
        _ => non_ref_variants_equal(a, b),
    }
}

/// Compare two Ref values by checking if they point to the same matched instance.
/// Complete identity covers unchanged and changed subtrees alike.
fn refs_equal(
    old_dom: &dyn DomView,
    new_dom: &dyn DomView,
    old_target: Ref,
    new_target: Ref,
    ref_mapping: &HashMap<Ref, Ref>,
) -> bool {
    let old_exists = !old_target.is_none() && old_dom.get_by_ref(old_target).is_some();
    let new_exists = !new_target.is_none() && new_dom.get_by_ref(new_target).is_some();

    match (old_exists, new_exists) {
        (false, false) => true,
        (true, false) | (false, true) => false,
        (true, true) => ref_mapping.get(&old_target) == Some(&new_target),
    }
}

// ============================================================================
// Variant → PropertyValue conversion
// ============================================================================

pub(crate) fn variant_to_property_value(v: &Variant) -> PropertyValue {
    match v {
        Variant::Bool(b) => PropertyValue::Bool { value: *b },
        Variant::Int32(n) => PropertyValue::Int32 { value: *n },
        Variant::Int64(n) => PropertyValue::Int64 { value: *n },
        Variant::Float32(n) => PropertyValue::Float32 { value: *n },
        Variant::Float64(n) => PropertyValue::Float64 { value: *n },
        Variant::String(s) => PropertyValue::String { value: s.clone() },
        Variant::BinaryString(bs) => PropertyValue::BinaryString {
            len: bs.clone().into_vec().len(),
        },
        Variant::Vector2(v) => PropertyValue::Vector2 { x: v.x, y: v.y },
        Variant::Vector3(v) => PropertyValue::Vector3 {
            x: v.x,
            y: v.y,
            z: v.z,
        },
        Variant::CFrame(cf) => PropertyValue::CFrame((*cf).into()),
        Variant::Color3(c) => PropertyValue::Color3 {
            r: c.r,
            g: c.g,
            b: c.b,
        },
        Variant::Color3uint8(c) => PropertyValue::Color3 {
            r: c.r as f32 / 255.0,
            g: c.g as f32 / 255.0,
            b: c.b as f32 / 255.0,
        },
        Variant::OptionalCFrame(Some(cf)) => variant_to_property_value(&Variant::CFrame(*cf)),
        Variant::OptionalCFrame(None) => PropertyValue::Nil,
        Variant::ContentId(content) => PropertyValue::String {
            value: content.as_str().to_string(),
        },
        Variant::Content(content) => match content.value() {
            ContentType::None => PropertyValue::Nil,
            ContentType::Uri(uri) => PropertyValue::String {
                value: uri.to_string(),
            },
            ContentType::Object(_) => PropertyValue::Other {
                type_name: "ContentObject".to_string(),
            },
            _ => PropertyValue::Other {
                type_name: "Content".to_string(),
            },
        },
        Variant::BrickColor(bc) => PropertyValue::BrickColor { value: *bc as u16 },
        Variant::Enum(e) => PropertyValue::Enum { value: e.to_u32() },
        Variant::UDim(u) => PropertyValue::UDim {
            scale: u.scale,
            offset: u.offset,
        },
        Variant::UDim2(u) => PropertyValue::UDim2 {
            x_scale: u.x.scale,
            x_offset: u.x.offset,
            y_scale: u.y.scale,
            y_offset: u.y.offset,
        },
        Variant::Ref(r) => {
            if r.is_none() {
                PropertyValue::Nil
            } else {
                PropertyValue::Ref {
                    value: format!("{}", r),
                }
            }
        }
        Variant::NumberRange(nr) => PropertyValue::NumberRange {
            min: nr.min,
            max: nr.max,
        },
        Variant::NumberSequence(ns) => PropertyValue::NumberSequence {
            keypoints: ns
                .keypoints
                .iter()
                .map(|kp| NumberKeypoint {
                    time: kp.time,
                    value: kp.value,
                    envelope: kp.envelope,
                })
                .collect(),
        },
        Variant::ColorSequence(cs) => PropertyValue::ColorSequence {
            keypoints: cs
                .keypoints
                .iter()
                .map(|kp| ColorKeypoint {
                    time: kp.time,
                    r: kp.color.r,
                    g: kp.color.g,
                    b: kp.color.b,
                })
                .collect(),
        },
        Variant::Rect(r) => PropertyValue::Rect {
            min_x: r.min.x,
            min_y: r.min.y,
            max_x: r.max.x,
            max_y: r.max.y,
        },
        _ => PropertyValue::Other {
            type_name: format!("{:?}", v.ty()),
        },
    }
}

/// Attribute strings decode from binary Roblox files as `BinaryString`
/// because the container's payload has no reflected property type. Inside an
/// Attributes container, valid UTF-8 bytes are nevertheless the Roblox
/// `string` attribute type. Keep generic BinaryString properties opaque.
pub(crate) fn attribute_variant_to_property_value(v: &Variant) -> PropertyValue {
    match v {
        Variant::BinaryString(value) => match std::str::from_utf8(value.as_ref()) {
            Ok(value) => PropertyValue::String {
                value: value.to_string(),
            },
            Err(_) => PropertyValue::BinaryString {
                len: AsRef::<[u8]>::as_ref(value).len(),
            },
        },
        _ => variant_to_property_value(v),
    }
}

#[cfg(test)]
mod tests {
    use super::refs_equal;
    use rbx_dom_weak::{InstanceBuilder, WeakDom};
    use std::collections::HashMap;

    #[test]
    fn identical_but_unmatched_ref_targets_are_not_equal() {
        let old_target = InstanceBuilder::new("Part").with_name("Target");
        let old_ref = old_target.referent();
        let old = WeakDom::new(InstanceBuilder::new("Folder").with_child(old_target));

        let new_target = InstanceBuilder::new("Part").with_name("Target");
        let new_ref = new_target.referent();
        let new = WeakDom::new(InstanceBuilder::new("Folder").with_child(new_target));

        assert!(!refs_equal(
            &old,
            &new,
            old_ref,
            new_ref,
            &HashMap::new()
        ));
        assert!(refs_equal(
            &old,
            &new,
            old_ref,
            new_ref,
            &HashMap::from([(old_ref, new_ref)])
        ));
    }
}
