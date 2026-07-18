//! Two-phase tree diffing.
//!
//! Phase 1: Recursively match instances across both DOMs, building a global ref mapping.
//! Phase 2: Compare properties using the ref mapping for Ref property comparison.
//! Ref properties pointing to matched instances are considered equal (same logical target),
//! with hash-based fallback for refs into pruned (identical) subtrees.

use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::Variant;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tracing::{info, info_span};

use crate::hash::{get_comparable_properties, DeepHashCache, LazyHashCache};
use crate::match_instances::{get_instance_path, match_children};
use crate::move_detect::detect_moves;

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
    Bool { value: bool },
    Int32 { value: i32 },
    Int64 { value: i64 },
    Float32 { value: f32 },
    Float64 { value: f64 },
    String { value: String },
    BinaryString { len: usize },
    Ref { value: String },
    Vector2 { x: f32, y: f32 },
    Vector3 { x: f32, y: f32, z: f32 },
    CFrame {
        position: [f32; 3],
        orientation: [[f32; 3]; 3],
    },
    Color3 { r: f32, g: f32, b: f32 },
    BrickColor { value: u16 },
    Enum { value: u32 },
    UDim { scale: f32, offset: i32 },
    UDim2 {
        x_scale: f32,
        x_offset: i32,
        y_scale: f32,
        y_offset: i32,
    },
    NumberRange { min: f32, max: f32 },
    NumberSequence { keypoints: Vec<NumberKeypoint> },
    ColorSequence { keypoints: Vec<ColorKeypoint> },
    Rect { min_x: f32, min_y: f32, max_x: f32, max_y: f32 },
    Other { type_name: String },
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
        Self { ignore_properties: ignore }
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
    let _span = info_span!("compute_diff").entered();

    let old_deep = DeepHashCache::new(old_dom, &config.ignore_properties);
    let new_deep = DeepHashCache::new(new_dom, &config.ignore_properties);

    // Phase 1: Build global ref mapping (old_ref → new_ref) for matched instances,
    // collecting removed/added subtree roots for global move detection
    let mut ref_mapping = HashMap::new();
    let mut removed_roots = Vec::new();
    let mut added_roots = Vec::new();
    build_ref_mapping(
        old_dom, new_dom,
        old_dom.root_ref(), new_dom.root_ref(),
        old_hashes, new_hashes,
        &old_deep, &new_deep,
        &mut ref_mapping,
        &mut removed_roots,
        &mut added_roots,
    );
    info!(matched_pairs = ref_mapping.len(), "ref mapping built");

    // Phase 1.5: Pair removed/added roots globally into moves.
    // Must happen before the diff pass so Ref properties pointing at moved
    // instances compare through the mapping instead of reporting false changes.
    let moves = detect_moves(
        old_dom, new_dom,
        removed_roots, added_roots,
        &old_deep, &new_deep,
    );
    let moved_old: HashSet<Ref> = moves.iter().map(|(o, _)| *o).collect();
    let moved_new: HashSet<Ref> = moves.iter().map(|(_, n)| *n).collect();
    for (old_root, new_root) in &moves {
        ref_mapping.insert(*old_root, *new_root);
        // Map matched descendants of edited moves too (pure moves prune via deep hash)
        if old_deep.get(*old_root) != new_deep.get(*new_root) {
            build_ref_mapping(
                old_dom, new_dom, *old_root, *new_root,
                old_hashes, new_hashes, &old_deep, &new_deep,
                &mut ref_mapping,
                &mut Vec::new(), &mut Vec::new(),
            );
        }
    }

    // Phase 2: Diff using the mapping
    let mut diffs = Vec::new();
    diff_pass(
        old_dom,
        new_dom,
        old_dom.root_ref(),
        new_dom.root_ref(),
        old_hashes,
        new_hashes,
        &old_deep,
        &new_deep,
        config,
        &ref_mapping,
        &moved_old,
        &moved_new,
        &mut diffs,
    );

    // Phase 3: Emit moves, then recurse into moved subtrees that also changed
    for (old_root, new_root) in &moves {
        let property_changes = diff_properties(
            old_dom, new_dom, *old_root, *new_root,
            config, &ref_mapping, &old_deep, &new_deep,
        );
        if let Some(inst) = new_dom.get_by_ref(*new_root) {
            diffs.push(DiffEntry::Moved {
                old_ref: format!("{}", *old_root),
                new_ref: format!("{}", *new_root),
                old_path: get_instance_path(old_dom, *old_root),
                path: get_instance_path(new_dom, *new_root),
                class: inst.class.to_string(),
                property_changes,
            });
        }
        if old_deep.get(*old_root) != new_deep.get(*new_root) {
            diff_pass(
                old_dom, new_dom, *old_root, *new_root,
                old_hashes, new_hashes, &old_deep, &new_deep,
                config, &ref_mapping, &moved_old, &moved_new,
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
fn build_ref_mapping(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    old_ref: Ref,
    new_ref: Ref,
    old_hashes: &LazyHashCache,
    new_hashes: &LazyHashCache,
    old_deep: &DeepHashCache,
    new_deep: &DeepHashCache,
    mapping: &mut HashMap<Ref, Ref>,
    removed_roots: &mut Vec<Ref>,
    added_roots: &mut Vec<Ref>,
) {
    let match_result = match_children(old_dom, new_dom, old_ref, new_ref, old_hashes, new_hashes);
    removed_roots.extend_from_slice(&match_result.removed);
    added_roots.extend_from_slice(&match_result.added);
    for (old_child, new_child) in &match_result.matched {
        mapping.insert(*old_child, *new_child);
        // Only recurse into subtrees where deep hashes differ (same pruning as diff_pass)
        if old_deep.get(*old_child) != new_deep.get(*new_child) {
            build_ref_mapping(
                old_dom, new_dom, *old_child, *new_child,
                old_hashes, new_hashes, old_deep, new_deep, mapping,
                removed_roots, added_roots,
            );
        }
    }
}

/// Phase 2: match children, compare properties, recurse into changed subtrees.
fn diff_pass(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    old_ref: Ref,
    new_ref: Ref,
    old_hashes: &LazyHashCache,
    new_hashes: &LazyHashCache,
    old_deep: &DeepHashCache,
    new_deep: &DeepHashCache,
    config: &DiffConfig,
    ref_mapping: &HashMap<Ref, Ref>,
    moved_old: &HashSet<Ref>,
    moved_new: &HashSet<Ref>,
    diffs: &mut Vec<DiffEntry>,
) {
    let match_result = match_children(old_dom, new_dom, old_ref, new_ref, old_hashes, new_hashes);

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
                class: inst.class.to_string(),
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
                class: inst.class.to_string(),
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
                    class: inst.class.to_string(),
                    property_changes,
                });
            }
        }

        // Recurse into children
        diff_pass(
            old_dom,
            new_dom,
            *old_child_ref,
            *new_child_ref,
            old_hashes,
            new_hashes,
            old_deep,
            new_deep,
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

/// Compare properties between two matched instances.
/// Detects name changes (for renamed instances matched via class fallback)
/// and uses the ref mapping for Ref property comparison.
/// Filters out non-reflected, non-serializable, and default-valued properties
/// to avoid false positives from serialization differences.
fn diff_properties(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    old_ref: Ref,
    new_ref: Ref,
    config: &DiffConfig,
    ref_mapping: &HashMap<Ref, Ref>,
    old_deep: &DeepHashCache,
    new_deep: &DeepHashCache,
) -> Vec<PropertyChange> {
    let old_inst = old_dom.get_by_ref(old_ref).unwrap();
    let new_inst = new_dom.get_by_ref(new_ref).unwrap();

    let database = rbx_reflection_database::get().unwrap();
    let class_name = new_inst.class.as_str();
    let defaults = database
        .classes
        .get(class_name)
        .map(|cd| &cd.default_properties);

    let mut changes = Vec::new();
    let mut visited = HashSet::new();

    // Detect name changes (inst.name is separate from inst.properties in rbx_dom_weak)
    if old_inst.name != new_inst.name {
        changes.push(PropertyChange {
            name: "Name".to_string(),
            old_value: Some(PropertyValue::String { value: old_inst.name.clone() }),
            new_value: Some(PropertyValue::String { value: new_inst.name.clone() }),
        });
    }

    // Check properties in new instance
    for (name, new_value) in &new_inst.properties {
        if config.ignore_properties.contains(name.as_str()) {
            continue;
        }
        if !should_compare_property(class_name, name) {
            continue;
        }
        visited.insert(name.clone());

        match old_inst.properties.get(name) {
            Some(old_value) => {
                if !variants_equal(old_dom, new_dom, old_value, new_value, ref_mapping, old_deep, new_deep) {
                    changes.push(PropertyChange {
                        name: name.to_string(),
                        old_value: Some(variant_to_property_value(old_value)),
                        new_value: Some(variant_to_property_value(new_value)),
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
                changes.push(PropertyChange {
                    name: name.to_string(),
                    old_value: None,
                    new_value: Some(variant_to_property_value(new_value)),
                });
            }
        }
    }

    // Check for removed properties
    for (name, old_value) in &old_inst.properties {
        if config.ignore_properties.contains(name.as_str()) {
            continue;
        }
        if !should_compare_property(old_inst.class.as_str(), name) {
            continue;
        }
        if !visited.contains(name) {
            // Property only in old — skip if it's just a default value
            if is_default_value(defaults, name, old_value) {
                continue;
            }
            // A nil Ref on one side only is semantically "unset" — not a change
            if matches!(old_value, Variant::Ref(r) if r.is_none()) {
                continue;
            }
            changes.push(PropertyChange {
                name: name.to_string(),
                old_value: Some(variant_to_property_value(old_value)),
                new_value: None,
            });
        }
    }

    changes
}

/// Check if a property should be compared (is meaningful for diffing).
/// Uses the shared comparable properties set from hash.rs.
fn should_compare_property(class_name: &str, prop_name: &str) -> bool {
    get_comparable_properties(class_name).contains(prop_name)
}

/// Studio serializes every service under the DataModel root on save, plus
/// internals like FilteredSelection. When comparing place files (root is a
/// DataModel), additions/removals directly at the root are serialization
/// noise, not changes. Never applies to model diffs (non-DataModel root).
fn is_studio_artifact(dom: &WeakDom, parent_ref: Ref, _inst: &rbx_dom_weak::Instance) -> bool {
    let parent = match dom.get_by_ref(parent_ref) {
        Some(p) => p,
        None => return false,
    };
    parent_ref == dom.root_ref() && parent.class.as_str() == "DataModel"
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

/// Compare two variants for equality (with tolerance for floats).
/// Uses ref mapping for Ref comparison (checks logical identity of targets).
fn variants_equal(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    a: &Variant,
    b: &Variant,
    ref_mapping: &HashMap<Ref, Ref>,
    old_deep: &DeepHashCache,
    new_deep: &DeepHashCache,
) -> bool {
    use std::mem::discriminant;

    if discriminant(a) != discriminant(b) {
        return false;
    }

    match (a, b) {
        (Variant::Float32(x), Variant::Float32(y)) => {
            x == y || (x.is_nan() && y.is_nan()) || (x - y).abs() < 0.01
        }
        (Variant::Float64(x), Variant::Float64(y)) => {
            x == y || (x.is_nan() && y.is_nan()) || (x - y).abs() < 0.01
        }
        (Variant::Vector3(x), Variant::Vector3(y)) => {
            (x.x - y.x).abs() < 0.01 && (x.y - y.y).abs() < 0.01 && (x.z - y.z).abs() < 0.01
        }
        (Variant::CFrame(x), Variant::CFrame(y)) => {
            // Compare all components (position + rotation matrix), like rojo's
            // trueEquals — position-only comparison silently drops pure rotations
            let vec_eq = |a: rbx_types::Vector3, b: rbx_types::Vector3| {
                (a.x - b.x).abs() < 0.01 && (a.y - b.y).abs() < 0.01 && (a.z - b.z).abs() < 0.01
            };
            vec_eq(x.position, y.position)
                && vec_eq(x.orientation.x, y.orientation.x)
                && vec_eq(x.orientation.y, y.orientation.y)
                && vec_eq(x.orientation.z, y.orientation.z)
        }
        (Variant::Ref(old_target), Variant::Ref(new_target)) => {
            refs_equal(old_dom, new_dom, *old_target, *new_target, ref_mapping, old_deep, new_deep)
        }
        // Asset URIs: Studio rewrites URL spellings on save (roblox.com/asset/?id=N
        // vs rbxassetid://N) — compare normalized so the same asset is equal
        (Variant::Content(a), Variant::Content(b)) => {
            use crate::hash::normalize_asset_uri;
            use rbx_types::ContentType;
            match (a.value(), b.value()) {
                (ContentType::None, ContentType::None) => true,
                (ContentType::Uri(ua), ContentType::Uri(ub)) => {
                    normalize_asset_uri(ua) == normalize_asset_uri(ub)
                }
                // Object refs into the DOM: treat as equal (rare; ref identity
                // is covered by the instances themselves)
                (ContentType::Object(_), ContentType::Object(_)) => true,
                _ => false,
            }
        }
        (Variant::ContentId(a), Variant::ContentId(b)) => {
            crate::hash::normalize_asset_uri(a.as_str()) == crate::hash::normalize_asset_uri(b.as_str())
        }
        (Variant::UniqueId(_), Variant::UniqueId(_)) => true, // Skip uniqueid
        _ => a == b,
    }
}

/// Compare two Ref values by checking if they point to the same matched instance.
/// Uses the ref mapping first (covers matched instances that may have changed content),
/// falls back to deep hash comparison for refs into pruned (identical) subtrees.
fn refs_equal(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
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

fn variant_to_property_value(v: &Variant) -> PropertyValue {
    match v {
        Variant::Bool(b) => PropertyValue::Bool { value: *b },
        Variant::Int32(n) => PropertyValue::Int32 { value: *n },
        Variant::Int64(n) => PropertyValue::Int64 { value: *n },
        Variant::Float32(n) => PropertyValue::Float32 { value: *n },
        Variant::Float64(n) => PropertyValue::Float64 { value: *n },
        Variant::String(s) => PropertyValue::String { value: s.clone() },
        Variant::BinaryString(bs) => PropertyValue::BinaryString { len: bs.clone().into_vec().len() },
        Variant::Vector2(v) => PropertyValue::Vector2 { x: v.x, y: v.y },
        Variant::Vector3(v) => PropertyValue::Vector3 { x: v.x, y: v.y, z: v.z },
        Variant::CFrame(cf) => PropertyValue::CFrame {
            position: [cf.position.x, cf.position.y, cf.position.z],
            orientation: [
                [cf.orientation.x.x, cf.orientation.x.y, cf.orientation.x.z],
                [cf.orientation.y.x, cf.orientation.y.y, cf.orientation.y.z],
                [cf.orientation.z.x, cf.orientation.z.y, cf.orientation.z.z],
            ],
        },
        Variant::Color3(c) => PropertyValue::Color3 { r: c.r, g: c.g, b: c.b },
        Variant::BrickColor(bc) => PropertyValue::BrickColor { value: *bc as u16 },
        Variant::Enum(e) => PropertyValue::Enum { value: e.to_u32() },
        Variant::UDim(u) => PropertyValue::UDim { scale: u.scale, offset: u.offset },
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
                PropertyValue::Ref { value: format!("{}", r) }
            }
        }
        Variant::NumberRange(nr) => PropertyValue::NumberRange { min: nr.min, max: nr.max },
        Variant::NumberSequence(ns) => PropertyValue::NumberSequence {
            keypoints: ns.keypoints.iter().map(|kp| NumberKeypoint {
                time: kp.time,
                value: kp.value,
                envelope: kp.envelope,
            }).collect(),
        },
        Variant::ColorSequence(cs) => PropertyValue::ColorSequence {
            keypoints: cs.keypoints.iter().map(|kp| ColorKeypoint {
                time: kp.time,
                r: kp.color.r,
                g: kp.color.g,
                b: kp.color.b,
            }).collect(),
        },
        Variant::Rect(r) => PropertyValue::Rect {
            min_x: r.min.x,
            min_y: r.min.y,
            max_x: r.max.x,
            max_y: r.max.y,
        },
        _ => PropertyValue::Other { type_name: format!("{:?}", v.ty()) },
    }
}
