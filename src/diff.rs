//! Two-phase tree diffing.
//!
//! Phase 1: Recursively match instances across both DOMs, building a global ref mapping.
//! Phase 2: Compare properties using the ref mapping for Ref property comparison.
//! Ref properties pointing to matched instances are considered equal (same logical target),
//! with hash-based fallback for refs into pruned (identical) subtrees.

use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::{CFrame, ContentType, Variant};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tracing::{info, info_span};

use crate::diff_dom::{DomView, InstanceView};
use crate::edit_script::InstanceIdentity;
use crate::hash::{DeepHashCache, LazyHashCache};
use crate::match_instances::{get_instance_path, Matcher};
use crate::move_detect::detect_moves;
use crate::property_semantics::get_authored_properties;
use crate::value_compare::non_ref_variants_equal;

/// A single difference found between two DOMs.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DiffEntry {
    /// Instance was added (only in new DOM)
    Added {
        new_ref: String,
        path: String,
        class: String,
    },
    /// Instance was removed (only in old DOM)
    Removed {
        old_ref: String,
        path: String,
        class: String,
    },
    /// Instance was modified (properties changed)
    Modified {
        old_ref: String,
        new_ref: String,
        path: String,
        class: String,
        property_changes: Vec<PropertyChange>,
    },
    /// Instance was moved to a different parent (same logical instance).
    /// `path` is the new location; property_changes covers any edits made
    /// alongside the move (empty for a pure move).
    Moved {
        old_ref: String,
        new_ref: String,
        old_path: String,
        path: String,
        class: String,
        property_changes: Vec<PropertyChange>,
    },
    /// A Model boundary and its world-space descendants moved together.
    /// This is an inferred rigid transform, not a Roblox property change;
    /// `WorldPivotData` edits that differ from the content still appear as
    /// ordinary property changes.
    ModelFrame {
        old_ref: String,
        new_ref: String,
        path: String,
        class: String,
        /// Stable top-down order among model-frame entries.
        order: usize,
        /// `order` of the nearest ancestor frame, when nested.
        parent_order: Option<usize>,
        /// Local transform relative to `parent_order`, or world-space when
        /// this entry has no participating parent.
        delta: CFrameValue,
    },
}

/// Serializable representation of a rigid CFrame used by model-frame diffs.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CFrameValue {
    pub position: [f32; 3],
    pub orientation: [[f32; 3]; 3],
}

impl From<CFrame> for CFrameValue {
    fn from(value: CFrame) -> Self {
        Self {
            position: [value.position.x, value.position.y, value.position.z],
            orientation: [
                [
                    value.orientation.x.x,
                    value.orientation.x.y,
                    value.orientation.x.z,
                ],
                [
                    value.orientation.y.x,
                    value.orientation.y.y,
                    value.orientation.y.z,
                ],
                [
                    value.orientation.z.x,
                    value.orientation.z.y,
                    value.orientation.z.z,
                ],
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
    CFrame {
        position: [f32; 3],
        orientation: [[f32; 3]; 3],
    },
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

/// Compute the diff between two DOMs.
/// Phase 1: Build global ref mapping. Phase 2: Diff using the mapping.
pub fn compute_diff(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    old_hashes: &LazyHashCache,
    new_hashes: &LazyHashCache,
    config: &DiffConfig,
) -> Vec<DiffEntry> {
    compute_diff_with_identity(old_dom, new_dom, old_hashes, new_hashes, config, None)
}

/// Compute a diff while preserving identity captured before a
/// representation-only canonicalization.
pub(crate) fn compute_diff_with_identity(
    old_dom: &dyn DomView,
    new_dom: &dyn DomView,
    old_hashes: &LazyHashCache,
    new_hashes: &LazyHashCache,
    config: &DiffConfig,
    identity: Option<&InstanceIdentity>,
) -> Vec<DiffEntry> {
    let _span = info_span!("compute_diff").entered();

    let old_deep = DeepHashCache::new(old_dom, &config.ignore_properties);
    let new_deep = DeepHashCache::new(new_dom, &config.ignore_properties);
    let mut matcher = Matcher::new(
        old_dom, new_dom, old_hashes, new_hashes, &old_deep, &new_deep,
    );
    if let Some(identity) = identity {
        matcher = matcher.with_complete_identity(&identity.matched);
    }

    // Phase 1: Build global ref mapping (old_ref → new_ref) for matched instances,
    // collecting removed/added subtree roots for global move detection
    let mut ref_mapping = identity
        .map(|identity| identity.matched.clone())
        .unwrap_or_default();
    let mut removed_roots = Vec::new();
    let mut added_roots = Vec::new();
    if identity.is_none() {
        build_ref_mapping(
            &matcher,
            old_dom.root_ref(),
            new_dom.root_ref(),
            &mut ref_mapping,
            &mut removed_roots,
            &mut added_roots,
        );
    }
    info!(matched_pairs = ref_mapping.len(), "ref mapping built");

    // Phase 1.5: Pair removed/added roots globally into moves.
    // Must happen before the diff pass so Ref properties pointing at moved
    // instances compare through the mapping instead of reporting false changes.
    let moves = identity.map_or_else(
        || {
            detect_moves(
                old_dom,
                new_dom,
                removed_roots,
                added_roots,
                &old_deep,
                &new_deep,
            )
        },
        |identity| identity.moves.clone(),
    );
    let moved_old: HashSet<Ref> = moves.iter().map(|(o, _)| *o).collect();
    let moved_new: HashSet<Ref> = moves.iter().map(|(_, n)| *n).collect();
    for (old_root, new_root) in &moves {
        ref_mapping.insert(*old_root, *new_root);
        // Map matched descendants of edited moves too (pure moves prune via deep hash)
        if old_deep.get(*old_root) != new_deep.get(*new_root) {
            build_ref_mapping(
                &matcher,
                *old_root,
                *new_root,
                &mut ref_mapping,
                &mut Vec::new(),
                &mut Vec::new(),
            );
        }
    }

    // Phase 2: Diff using the mapping
    let mut diffs = Vec::new();
    diff_pass(
        &matcher,
        old_dom.root_ref(),
        new_dom.root_ref(),
        config,
        &ref_mapping,
        &moved_old,
        &moved_new,
        &mut diffs,
    );

    // Phase 3: Emit moves, then recurse into moved subtrees that also changed
    for (old_root, new_root) in &moves {
        let property_changes = diff_properties(
            old_dom,
            new_dom,
            *old_root,
            *new_root,
            config,
            &ref_mapping,
            &old_deep,
            &new_deep,
        );
        if let Some(inst) = new_dom.get_by_ref(*new_root) {
            diffs.push(DiffEntry::Moved {
                old_ref: format!("{}", *old_root),
                new_ref: format!("{}", *new_root),
                old_path: get_instance_path(old_dom, *old_root),
                path: get_instance_path(new_dom, *new_root),
                class: inst.class().to_string(),
                property_changes,
            });
        }
        if old_deep.get(*old_root) != new_deep.get(*new_root) {
            diff_pass(
                &matcher,
                *old_root,
                *new_root,
                config,
                &ref_mapping,
                &moved_old,
                &moved_new,
                &mut diffs,
            );
        }
    }

    old_hashes.log_stats("old");
    new_hashes.log_stats("new");
    info!(diffs_found = diffs.len(), "diff complete");

    diffs
}

/// Phase 1: Recursively match all instances, building the global ref mapping.
/// Only recurses into non-pruned subtrees (where deep hashes differ).
/// Collects removed/added subtree roots along the way for global move detection.
pub(crate) fn build_ref_mapping(
    matcher: &Matcher<'_>,
    old_ref: Ref,
    new_ref: Ref,
    mapping: &mut HashMap<Ref, Ref>,
    removed_roots: &mut Vec<Ref>,
    added_roots: &mut Vec<Ref>,
) {
    let match_result = matcher.match_children(old_ref, new_ref);
    removed_roots.extend_from_slice(&match_result.removed);
    added_roots.extend_from_slice(&match_result.added);
    for (old_child, new_child) in &match_result.matched {
        mapping.insert(*old_child, *new_child);
        // Only recurse into subtrees where deep hashes differ (same pruning as diff_pass)
        if matcher.old_deep().get(*old_child) != matcher.new_deep().get(*new_child) {
            build_ref_mapping(
                matcher,
                *old_child,
                *new_child,
                mapping,
                removed_roots,
                added_roots,
            );
        }
    }
}

/// Phase 2: match children, compare properties, recurse into changed subtrees.
fn diff_pass(
    matcher: &Matcher<'_>,
    old_ref: Ref,
    new_ref: Ref,
    config: &DiffConfig,
    ref_mapping: &HashMap<Ref, Ref>,
    moved_old: &HashSet<Ref>,
    moved_new: &HashSet<Ref>,
    diffs: &mut Vec<DiffEntry>,
) {
    let old_dom = matcher.old_dom();
    let new_dom = matcher.new_dom();
    let old_deep = matcher.old_deep();
    let new_deep = matcher.new_deep();
    let match_result = matcher.match_children(old_ref, new_ref);

    // Report removed instances (skipping those reclassified as moves)
    for removed_ref in &match_result.removed {
        if moved_old.contains(removed_ref) {
            continue;
        }
        if let Some(inst) = old_dom.get_by_ref(*removed_ref) {
            if is_studio_artifact(old_dom, old_ref, inst) {
                continue;
            }
            diffs.push(DiffEntry::Removed {
                old_ref: format!("{}", *removed_ref),
                path: get_instance_path(old_dom, *removed_ref),
                class: inst.class().to_string(),
            });
        }
    }

    // Report added instances (skipping those reclassified as moves)
    for added_ref in &match_result.added {
        if moved_new.contains(added_ref) {
            continue;
        }
        if let Some(inst) = new_dom.get_by_ref(*added_ref) {
            if is_studio_artifact(new_dom, new_ref, inst) {
                continue;
            }
            diffs.push(DiffEntry::Added {
                new_ref: format!("{}", *added_ref),
                path: get_instance_path(new_dom, *added_ref),
                class: inst.class().to_string(),
            });
        }
    }

    // Process matched pairs — prune unchanged subtrees via deep hash
    for (old_child_ref, new_child_ref) in &match_result.matched {
        // Pruning: if deep hashes match, entire subtree is identical — skip
        if old_deep.get(*old_child_ref) == new_deep.get(*new_child_ref) {
            continue;
        }

        let property_changes = diff_properties(
            old_dom,
            new_dom,
            *old_child_ref,
            *new_child_ref,
            config,
            ref_mapping,
            old_deep,
            new_deep,
        );

        if !property_changes.is_empty() {
            if let Some(inst) = new_dom.get_by_ref(*new_child_ref) {
                diffs.push(DiffEntry::Modified {
                    old_ref: format!("{}", *old_child_ref),
                    new_ref: format!("{}", *new_child_ref),
                    path: get_instance_path(new_dom, *new_child_ref),
                    class: inst.class().to_string(),
                    property_changes,
                });
            }
        }

        // Recurse into children
        diff_pass(
            matcher,
            *old_child_ref,
            *new_child_ref,
            config,
            ref_mapping,
            moved_old,
            moved_new,
            diffs,
        );
    }
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

fn attr_value_eq(a: &Variant, b: &Variant) -> bool {
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
    old_deep: &DeepHashCache,
    new_deep: &DeepHashCache,
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
                if !variants_equal(
                    old_dom,
                    new_dom,
                    old_value,
                    new_value,
                    ref_mapping,
                    old_deep,
                    new_deep,
                ) {
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

/// Compare properties between two matched instances, for display.
/// Detects name changes (inst.name is separate from inst.properties) and
/// converts raw variants into display-oriented PropertyValues.
fn diff_properties(
    old_dom: &dyn DomView,
    new_dom: &dyn DomView,
    old_ref: Ref,
    new_ref: Ref,
    config: &DiffConfig,
    ref_mapping: &HashMap<Ref, Ref>,
    old_deep: &DeepHashCache,
    new_deep: &DeepHashCache,
) -> Vec<PropertyChange> {
    let old_inst = old_dom.get_by_ref(old_ref).unwrap();
    let new_inst = new_dom.get_by_ref(new_ref).unwrap();

    let mut changes = Vec::new();

    if old_inst.name() != new_inst.name() {
        changes.push(PropertyChange {
            name: "Name".to_string(),
            old_value: Some(PropertyValue::String {
                value: old_inst.name().to_string(),
            }),
            new_value: Some(PropertyValue::String {
                value: new_inst.name().to_string(),
            }),
        });
    }

    for raw in raw_property_changes(
        old_dom,
        new_dom,
        old_ref,
        new_ref,
        config,
        ref_mapping,
        old_deep,
        new_deep,
    ) {
        let attribute = raw.name.starts_with("Attributes.");
        let display_value = |value: &Variant| {
            if attribute {
                attribute_variant_to_property_value(value)
            } else {
                variant_to_property_value(value)
            }
        };
        changes.push(PropertyChange {
            name: raw.name,
            old_value: raw.old.as_ref().map(display_value),
            new_value: raw.new.as_ref().map(display_value),
        });
    }

    changes
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
fn is_default_value(
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
    old_deep: &DeepHashCache,
    new_deep: &DeepHashCache,
) -> bool {
    match (a, b) {
        (Variant::Ref(old_target), Variant::Ref(new_target)) => refs_equal(
            old_dom,
            new_dom,
            *old_target,
            *new_target,
            ref_mapping,
            old_deep,
            new_deep,
        ),
        _ => non_ref_variants_equal(a, b),
    }
}

/// Compare two Ref values by checking if they point to the same matched instance.
/// Uses the ref mapping first (covers matched instances that may have changed content),
/// falls back to deep hash comparison for refs into pruned (identical) subtrees.
fn refs_equal(
    old_dom: &dyn DomView,
    new_dom: &dyn DomView,
    old_target: Ref,
    new_target: Ref,
    ref_mapping: &HashMap<Ref, Ref>,
    old_deep: &DeepHashCache,
    new_deep: &DeepHashCache,
) -> bool {
    let old_exists = !old_target.is_none() && old_dom.get_by_ref(old_target).is_some();
    let new_exists = !new_target.is_none() && new_dom.get_by_ref(new_target).is_some();

    match (old_exists, new_exists) {
        (false, false) => true,
        (true, false) | (false, true) => false,
        (true, true) => {
            // Check mapping: are these the same logical instance?
            if let Some(&mapped_new) = ref_mapping.get(&old_target) {
                return mapped_new == new_target;
            }
            // Fallback for pruned subtrees (identical content, not in mapping)
            old_deep.get(old_target) == new_deep.get(new_target)
        }
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
        Variant::CFrame(cf) => PropertyValue::CFrame {
            position: [cf.position.x, cf.position.y, cf.position.z],
            orientation: [
                [cf.orientation.x.x, cf.orientation.x.y, cf.orientation.x.z],
                [cf.orientation.y.x, cf.orientation.y.y, cf.orientation.y.z],
                [cf.orientation.z.x, cf.orientation.z.y, cf.orientation.z.z],
            ],
        },
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
