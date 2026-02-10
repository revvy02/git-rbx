//! Tree and property diffing logic.

use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::Variant;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tracing::{info, info_span};

use crate::hash::LazyHashCache;
use crate::match_instances::{get_instance_path, match_children, MatchResult};

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
pub fn compute_diff(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    old_hashes: &LazyHashCache,
    new_hashes: &LazyHashCache,
    config: &DiffConfig,
) -> Vec<DiffEntry> {
    let _span = info_span!("compute_diff").entered();

    let mut diffs = Vec::new();
    // Global mapping of matched instances: old_ref → new_ref
    let mut ref_mapping: HashMap<Ref, Ref> = HashMap::new();
    // Cache match results from first pass to avoid recomputing in second pass
    let mut match_cache: HashMap<(Ref, Ref), MatchResult> = HashMap::new();

    // First pass: build the complete ref mapping by traversing the tree
    {
        let _span = info_span!("build_ref_mapping").entered();
        build_ref_mapping(
            old_dom,
            new_dom,
            old_dom.root_ref(),
            new_dom.root_ref(),
            old_hashes,
            new_hashes,
            &mut ref_mapping,
            &mut match_cache,
        );
        info!(matched_pairs = ref_mapping.len(), cached_matches = match_cache.len(), "ref mapping complete");
    }

    // Second pass: compute diffs using the mapping for Ref comparison
    {
        let _span = info_span!("diff_recursive_pass").entered();
        diff_recursive(
            old_dom,
            new_dom,
            old_dom.root_ref(),
            new_dom.root_ref(),
            config,
            &ref_mapping,
            &match_cache,
            &mut diffs,
        );
        info!(diffs_found = diffs.len(), "diff pass complete");
    }

    old_hashes.log_stats("old");
    new_hashes.log_stats("new");

    diffs
}

/// Build a mapping of all matched instances (old_ref → new_ref)
fn build_ref_mapping(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    old_ref: Ref,
    new_ref: Ref,
    old_hashes: &LazyHashCache,
    new_hashes: &LazyHashCache,
    ref_mapping: &mut HashMap<Ref, Ref>,
    match_cache: &mut HashMap<(Ref, Ref), MatchResult>,
) {
    // Add this pair to the mapping
    ref_mapping.insert(old_ref, new_ref);

    // Match children and cache the result
    let match_result = match_children(old_dom, new_dom, old_ref, new_ref, old_hashes, new_hashes);

    for (old_child_ref, new_child_ref) in &match_result.matched {
        build_ref_mapping(
            old_dom,
            new_dom,
            *old_child_ref,
            *new_child_ref,
            old_hashes,
            new_hashes,
            ref_mapping,
            match_cache,
        );
    }

    match_cache.insert((old_ref, new_ref), match_result);
}

fn diff_recursive(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    old_ref: Ref,
    new_ref: Ref,
    config: &DiffConfig,
    ref_mapping: &HashMap<Ref, Ref>,
    match_cache: &HashMap<(Ref, Ref), MatchResult>,
    diffs: &mut Vec<DiffEntry>,
) {
    // Use cached match result from build_ref_mapping pass (avoids recomputing)
    let match_result = match_cache.get(&(old_ref, new_ref))
        .expect("match_cache missing entry — build_ref_mapping should have populated it");

    // Report removed instances
    for removed_ref in &match_result.removed {
        if let Some(inst) = old_dom.get_by_ref(*removed_ref) {
            diffs.push(DiffEntry::Removed {
                old_ref: format!("{}", *removed_ref),
                path: get_instance_path(old_dom, *removed_ref),
                class: inst.class.to_string(),
            });
        }
    }

    // Report added instances
    for added_ref in &match_result.added {
        if let Some(inst) = new_dom.get_by_ref(*added_ref) {
            diffs.push(DiffEntry::Added {
                new_ref: format!("{}", *added_ref),
                path: get_instance_path(new_dom, *added_ref),
                class: inst.class.to_string(),
            });
        }
    }

    // Compare matched pairs — always recurse (no hash-based pruning needed
    // since shallow hashes don't represent subtrees)
    for (old_child_ref, new_child_ref) in &match_result.matched {
        let property_changes = diff_properties(
            old_dom,
            new_dom,
            *old_child_ref,
            *new_child_ref,
            config,
            ref_mapping,
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

        diff_recursive(
            old_dom,
            new_dom,
            *old_child_ref,
            *new_child_ref,
            config,
            ref_mapping,
            match_cache,
            diffs,
        );
    }
}

/// Compare properties between two matched instances.
fn diff_properties(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    old_ref: Ref,
    new_ref: Ref,
    config: &DiffConfig,
    ref_mapping: &HashMap<Ref, Ref>,
) -> Vec<PropertyChange> {
    let old_inst = old_dom.get_by_ref(old_ref).unwrap();
    let new_inst = new_dom.get_by_ref(new_ref).unwrap();

    let mut changes = Vec::new();
    let mut visited = HashSet::new();

    // Check properties in new instance
    for (name, new_value) in &new_inst.properties {
        if config.ignore_properties.contains(name.as_str()) {
            continue;
        }
        visited.insert(name.clone());

        match old_inst.properties.get(name) {
            Some(old_value) => {
                if !variants_equal(old_dom, new_dom, old_value, new_value, ref_mapping) {
                    changes.push(PropertyChange {
                        name: name.to_string(),
                        old_value: Some(variant_to_property_value(old_value)),
                        new_value: Some(variant_to_property_value(new_value)),
                    });
                }
            }
            None => {
                // Property added
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
        if !visited.contains(name) {
            changes.push(PropertyChange {
                name: name.to_string(),
                old_value: Some(variant_to_property_value(old_value)),
                new_value: None,
            });
        }
    }

    changes
}

/// Compare two variants for equality (with tolerance for floats).
/// Uses ref_mapping to compare Ref properties by checking if targets are matched pairs.
fn variants_equal(
    _old_dom: &WeakDom,
    _new_dom: &WeakDom,
    a: &Variant,
    b: &Variant,
    ref_mapping: &HashMap<Ref, Ref>,
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
            let pos_eq = (x.position.x - y.position.x).abs() < 0.01
                && (x.position.y - y.position.y).abs() < 0.01
                && (x.position.z - y.position.z).abs() < 0.01;
            pos_eq // Simplified - full rotation comparison is complex
        }
        (Variant::Ref(old_target), Variant::Ref(new_target)) => {
            // Compare Refs by checking if they point to matched instances
            refs_equal(*old_target, *new_target, ref_mapping)
        }
        (Variant::UniqueId(_), Variant::UniqueId(_)) => true, // Skip uniqueid
        _ => a == b,
    }
}

/// Compare two Ref values by checking if they point to matched instances.
/// Returns true if:
/// - Both are null refs
/// - old_target maps to new_target in the ref_mapping (they're a matched pair)
fn refs_equal(old_target: Ref, new_target: Ref, ref_mapping: &HashMap<Ref, Ref>) -> bool {
    let old_is_none = old_target.is_none();
    let new_is_none = new_target.is_none();

    match (old_is_none, new_is_none) {
        (true, true) => true,   // Both null
        (true, false) | (false, true) => false,  // One null, one not
        (false, false) => {
            // Check if old_target maps to new_target (they're matched instances)
            ref_mapping.get(&old_target) == Some(&new_target)
        }
    }
}

/// Convert a Variant to a typed PropertyValue.
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

