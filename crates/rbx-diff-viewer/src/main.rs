//! rbx-diff-viewer: A visual diff viewer for Roblox files.
//! Generates a self-contained HTML file with embedded diff data.

use anyhow::{bail, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use clap::Parser;
use flate2::write::GzEncoder;
use flate2::Compression;
use rbx_diff::{diff_doms, DiffEntry};
use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_reflection::{PropertyKind, PropertySerialization, Scriptability};
use rbx_types::Variant;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "rbx-diff-viewer")]
#[command(about = "Visual diff viewer for Roblox rbxm/rbxmx files - generates HTML report")]
#[command(version)]
struct Args {
    /// Old (base) rbxm or rbxmx file
    old_file: String,

    /// New (changed) rbxm or rbxmx file
    new_file: String,

    /// Output HTML file
    #[arg(long, short, default_value = "diff.html")]
    output: String,
}

// ============================================================================
// Data structures for embedded JSON
// ============================================================================

/// Tree node for JSON serialization (full tree, no depth limit)
#[derive(Serialize)]
struct TreeNode {
    name: String,
    class: String,
    #[serde(rename = "ref")]
    referent: String,
    children: Vec<TreeNode>,
    has_children: bool,
}

#[derive(Serialize)]
struct Meta {
    old_name: String,
    new_name: String,
    summary: Summary,
}

#[derive(Serialize)]
struct Summary {
    added: usize,
    removed: usize,
    modified: usize,
}

/// Attribute entry for serialization
#[derive(Serialize)]
struct AttributeEntry {
    name: String,
    value: PropertyValue,
}

/// Property value variants for type-specific rendering
#[derive(Serialize)]
#[serde(tag = "kind")]
enum PropertyValue {
    Bool { value: bool },
    Int { value: i64 },
    Float { value: f64 },
    String { value: String },
    Vector2 { x: f64, y: f64 },
    Vector3 { x: f64, y: f64, z: f64 },
    CFrame { position: [f64; 3], orientation: [f64; 3] },
    Color3 { r: f64, g: f64, b: f64 },
    BrickColor { name: String, r: u8, g: u8, b: u8 },
    Enum { value: u32, name: Option<String> },
    Ref { value: Option<String> },
    Binary { size: usize },
    Attributes { entries: Vec<AttributeEntry> },
    Tags { values: Vec<String> },
    Unknown { display: String },
}

/// Property for JSON serialization
#[derive(Serialize)]
struct Property {
    name: String,
    value: PropertyValue,
    #[serde(rename = "type")]
    prop_type: String,
    category: String,
    #[serde(rename = "readOnly")]
    read_only: bool,
}

/// Core embedded data (parsed immediately on load)
#[derive(Serialize)]
struct CoreData {
    meta: Meta,
    #[serde(rename = "oldTree")]
    old_tree: TreeNode,
    #[serde(rename = "newTree")]
    new_tree: TreeNode,
    diffs: Vec<DiffEntry>,
    #[serde(rename = "classIcons")]
    class_icons: HashMap<String, String>,
}

// ============================================================================
// Main
// ============================================================================

fn main() -> Result<()> {
    // Init tracing subscriber (controlled via RUST_LOG env var)
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();

    eprintln!("Loading {}...", args.old_file);
    let old_dom = load_file(&args.old_file)?;

    eprintln!("Loading {}...", args.new_file);
    let new_dom = load_file(&args.new_file)?;

    eprintln!("Computing differences...");
    let diffs = diff_doms(&old_dom, &new_dom);

    let (added, removed, modified) = count_diffs(&diffs);
    eprintln!("Found {} added, {} removed, {} modified", added, removed, modified);

    eprintln!("Building data structures...");

    // Build full trees (no depth limit)
    let old_tree = serialize_tree_full(&old_dom, old_dom.root_ref());
    let new_tree = serialize_tree_full(&new_dom, new_dom.root_ref());

    // Build properties maps for diff-relevant instances only
    let (old_properties, new_properties) = collect_diff_properties(&old_dom, &new_dom, &diffs);

    // Load class icons from Roblox Studio installation
    let class_icons = if let Some(content_path) = find_roblox_content_path() {
        eprintln!("Found Roblox Studio at: {}", content_path.display());
        load_class_icons(&content_path)
    } else {
        eprintln!("Warning: Roblox Studio not found, icons will not be embedded");
        HashMap::new()
    };

    let meta = Meta {
        old_name: Path::new(&args.old_file)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
        new_name: Path::new(&args.new_file)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
        summary: Summary { added, removed, modified },
    };

    // Core data (small, parsed immediately)
    let core_data = CoreData {
        meta,
        old_tree,
        new_tree,
        diffs,
        class_icons,
    };

    eprintln!("Generating HTML...");

    // Serialize core data to JSON
    let core_json = serde_json::to_string(&core_data)?;
    let core_compressed = gzip_compress(core_json.as_bytes());
    let core_b64 = BASE64.encode(&core_compressed);
    eprintln!("Core data: {}MB -> {}MB compressed -> {}MB base64",
        core_json.len() / 1024 / 1024,
        core_compressed.len() / 1024 / 1024,
        core_b64.len() / 1024 / 1024,
    );

    // Compress properties (diff-only, so small)
    let old_props_json = serde_json::to_string(&old_properties)?;
    let old_props_compressed = gzip_compress(old_props_json.as_bytes());
    let old_props_b64 = BASE64.encode(&old_props_compressed);

    let new_props_json = serde_json::to_string(&new_properties)?;
    let new_props_compressed = gzip_compress(new_props_json.as_bytes());
    let new_props_b64 = BASE64.encode(&new_props_compressed);

    eprintln!(
        "Properties (diff-only): old {} instances ({}KB), new {} instances ({}KB)",
        old_properties.len(),
        old_props_b64.len() / 1024,
        new_properties.len(),
        new_props_b64.len() / 1024,
    );

    // Load HTML template and inject data
    let html_template = include_str!("../dist/index.html");

    // Inject core data (compressed + base64, decompressed on load)
    let output_html = html_template.replace(
        "/*__DIFF_DATA_PLACEHOLDER__*/",
        &format!("window.__DIFF_DATA_B64__ = \"{}\"", core_b64),
    );

    // Inject compressed properties
    let output_html = output_html.replace(
        "/*__OLD_PROPS_PLACEHOLDER__*/",
        &format!("window.__OLD_PROPS_B64__ = \"{}\"", old_props_b64),
    );
    let output_html = output_html.replace(
        "/*__NEW_PROPS_PLACEHOLDER__*/",
        &format!("window.__NEW_PROPS_B64__ = \"{}\"", new_props_b64),
    );

    // Write output file
    std::fs::write(&args.output, output_html)?;

    eprintln!("Generated: {}", args.output);
    eprintln!("Open in any browser to view the diff.");

    Ok(())
}

// ============================================================================
// Helper functions
// ============================================================================

/// Build a ref-string -> Ref lookup map for an entire DOM.
fn build_ref_lookup(dom: &WeakDom) -> HashMap<String, Ref> {
    let mut map = HashMap::new();
    build_ref_lookup_recursive(dom, dom.root_ref(), &mut map);
    map
}

fn build_ref_lookup_recursive(dom: &WeakDom, referent: Ref, map: &mut HashMap<String, Ref>) {
    map.insert(format!("{}", referent), referent);
    if let Some(inst) = dom.get_by_ref(referent) {
        for &child_ref in inst.children() {
            build_ref_lookup_recursive(dom, child_ref, map);
        }
    }
}

/// Collect properties only for instances that appear in the diff list.
fn collect_diff_properties(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    diffs: &[DiffEntry],
) -> (HashMap<String, Vec<Property>>, HashMap<String, Vec<Property>>) {
    let old_lookup = build_ref_lookup(old_dom);
    let new_lookup = build_ref_lookup(new_dom);

    let mut old_props = HashMap::new();
    let mut new_props = HashMap::new();

    for diff in diffs {
        match diff {
            DiffEntry::Added { new_ref, .. } => {
                if let Some(&r) = new_lookup.get(new_ref) {
                    if let Some(inst) = new_dom.get_by_ref(r) {
                        new_props.insert(new_ref.clone(), collect_instance_properties(inst));
                    }
                }
            }
            DiffEntry::Removed { old_ref, .. } => {
                if let Some(&r) = old_lookup.get(old_ref) {
                    if let Some(inst) = old_dom.get_by_ref(r) {
                        old_props.insert(old_ref.clone(), collect_instance_properties(inst));
                    }
                }
            }
            DiffEntry::Modified { old_ref, new_ref, .. } => {
                if let Some(&r) = old_lookup.get(old_ref) {
                    if let Some(inst) = old_dom.get_by_ref(r) {
                        old_props.insert(old_ref.clone(), collect_instance_properties(inst));
                    }
                }
                if let Some(&r) = new_lookup.get(new_ref) {
                    if let Some(inst) = new_dom.get_by_ref(r) {
                        new_props.insert(new_ref.clone(), collect_instance_properties(inst));
                    }
                }
            }
        }
    }

    (old_props, new_props)
}

/// Compress data using gzip
fn gzip_compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).expect("Failed to write to gzip encoder");
    encoder.finish().expect("Failed to finish gzip compression")
}

/// Find the Roblox Studio content path based on the current OS.
fn find_roblox_content_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let path = PathBuf::from("/Applications/RobloxStudio.app/Contents/Resources/content");
        if path.exists() {
            return Some(path);
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let versions_dir = PathBuf::from(local_app_data).join("Roblox/Versions");
            if let Ok(entries) = std::fs::read_dir(&versions_dir) {
                let mut versions: Vec<_> = entries
                    .flatten()
                    .filter(|e| e.file_name().to_string_lossy().starts_with("version-"))
                    .collect();
                versions.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
                if let Some(latest) = versions.first() {
                    let path = latest.path().join("content");
                    if path.exists() {
                        return Some(path);
                    }
                }
            }
        }
    }

    None
}

/// Load class icons from Roblox Studio installation, encoding as base64 data URLs.
fn load_class_icons(content_path: &Path) -> HashMap<String, String> {
    let json: serde_json::Value = serde_json::from_str(include_str!("../class_icons.json"))
        .expect("Invalid class_icons.json");

    let mut icons = HashMap::new();

    if let Some(obj) = json.as_object() {
        for (class_name, icon_data) in obj {
            if let Some(rbxasset_url) = icon_data.get("Image").and_then(|v| v.as_str()) {
                if let Some(relative) = rbxasset_url.strip_prefix("rbxasset://") {
                    let full_path = content_path.join(relative);
                    if let Ok(bytes) = std::fs::read(&full_path) {
                        let b64 = BASE64.encode(&bytes);
                        let data_url = format!("data:image/png;base64,{}", b64);
                        icons.insert(class_name.clone(), data_url);
                    }
                }
            }
        }
    }

    eprintln!("Loaded {} class icons", icons.len());
    icons
}

/// Load a Roblox file (rbxm or rbxmx) based on extension.
fn load_file(path: &str) -> Result<WeakDom> {
    let path = Path::new(path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let file = BufReader::new(File::open(path)?);

    match ext.to_lowercase().as_str() {
        "rbxm" | "rbxl" => Ok(rbx_binary::from_reader(file)?),
        "rbxmx" | "rbxlx" => Ok(rbx_xml::from_reader_default(file)?),
        _ => bail!("Unknown file extension: {}. Expected .rbxm, .rbxmx, .rbxl, or .rbxlx", ext),
    }
}

/// Count diffs by type
fn count_diffs(diffs: &[DiffEntry]) -> (usize, usize, usize) {
    let mut added = 0;
    let mut removed = 0;
    let mut modified = 0;
    for d in diffs {
        match d {
            DiffEntry::Added { .. } => added += 1,
            DiffEntry::Removed { .. } => removed += 1,
            DiffEntry::Modified { .. } => modified += 1,
        }
    }
    (added, removed, modified)
}

/// Serialize entire DOM tree to JSON (no depth limit)
fn serialize_tree_full(dom: &WeakDom, referent: Ref) -> TreeNode {
    let inst = dom.get_by_ref(referent).unwrap();
    let children_refs = inst.children();
    TreeNode {
        name: inst.name.clone(),
        class: inst.class.to_string(),
        referent: format!("{}", referent),
        has_children: !children_refs.is_empty(),
        children: children_refs
            .iter()
            .map(|&r| serialize_tree_full(dom, r))
            .collect(),
    }
}


/// Collect filtered properties for a single instance.
fn collect_instance_properties(inst: &rbx_dom_weak::Instance) -> Vec<Property> {
    let database = rbx_reflection_database::get().unwrap();
    let class_data = database.classes.get(inst.class.as_str());
    let defaults = class_data.map(|cd| &cd.default_properties);

    let mut props: Vec<Property> = inst
        .properties
        .iter()
        .filter(|(name, value)| {
            if SKIP_PROPERTIES.contains(&name.as_str()) {
                return false;
            }
            if !should_property_serialize(database, &inst.class, name) {
                return false;
            }
            // Skip properties not in reflection database (internal engine state)
            // and properties with Scriptability::None (matches rojo's approach)
            match find_property_descriptor(database, &inst.class, name) {
                None => return false, // Not in reflection = internal
                Some(prop_data) => {
                    if matches!(prop_data.scriptability, Scriptability::None) {
                        return false;
                    }
                }
            }
            if let Some(defaults) = defaults {
                if let Some(default_value) = defaults.get(name.as_str()) {
                    if variant_eq(value, default_value) {
                        return false;
                    }
                }
            }
            true
        })
        .map(|(name, value)| Property {
            name: name.to_string(),
            value: to_property_value(value),
            prop_type: format!("{:?}", value.ty()),
            category: get_property_category(name).to_string(),
            read_only: is_property_read_only(name),
        })
        .collect();

    props.sort_by(|a, b| {
        let cat_cmp = category_order(&a.category).cmp(&category_order(&b.category));
        if cat_cmp == std::cmp::Ordering::Equal {
            a.name.cmp(&b.name)
        } else {
            cat_cmp
        }
    });

    props
}

/// Properties to always skip (non-deterministic, not useful in viewer)
static SKIP_PROPERTIES: &[&str] = &["UniqueId", "HistoryId", "SourceAssetId"];

/// Get category for a property name
fn get_property_category(name: &str) -> &'static str {
    match name {
        "BrickColor" | "CastShadow" | "Color" | "Material" | "MaterialVariant"
        | "Reflectance" | "Transparency" | "Color3" | "Color3uint8" => "Appearance",

        "Archivable" | "ClassName" | "Locked" | "Name" | "Parent"
        | "RobloxLocked" | "UniqueId" | "SourceAssetId"
        | "Attributes" | "Tags" => "Data",

        "Size" | "CFrame" | "Position" | "Orientation" | "Rotation"
        | "Origin" | "Pivot" | "PivotOffset" => "Transform",

        "Anchored" | "CanCollide" | "CanQuery" | "CanTouch" | "Massless"
        | "RootPriority" | "Shape" | "TopSurface" | "BottomSurface"
        | "FrontSurface" | "BackSurface" | "LeftSurface" | "RightSurface" => "Part",

        "PrimaryPart" | "ModelStreamingMode" | "LevelOfDetail" | "Scale"
        | "WorldPivot" | "ModelMeshCFrame" | "ModelMeshData" | "ModelMeshSize"
        | "NeedsPivotMigration" => "Model",

        "AssemblyAngularVelocity" | "AssemblyLinearVelocity" | "AssemblyMass"
        | "AssemblyCenterOfMass" | "AssemblyRootPart" | "CustomPhysicalProperties"
        | "Density" | "Elasticity" | "Friction" | "ElasticityWeight" | "FrictionWeight" => "Physics",

        "Enabled" | "Visible" | "Active" | "Disabled" => "Behavior",

        _ => "Other",
    }
}

/// Check if a property is read-only
fn is_property_read_only(name: &str) -> bool {
    matches!(name,
        "ClassName" | "UniqueId" | "Parent" | "AssemblyMass"
        | "AssemblyCenterOfMass" | "AssemblyRootPart" | "Mass"
        | "ResizeableFaces" | "ResizeIncrement"
    )
}

/// Get category sort order
fn category_order(category: &str) -> u8 {
    match category {
        "Appearance" => 0,
        "Data" => 1,
        "Transform" => 2,
        "Part" => 3,
        "Model" => 4,
        "Physics" => 5,
        "Behavior" => 6,
        _ => 99,
    }
}

/// Find a property descriptor by walking the class hierarchy.
/// Returns None if the property is not found in any class in the hierarchy.
fn find_property_descriptor<'a>(
    database: &'a rbx_reflection::ReflectionDatabase,
    class_name: &str,
    prop_name: &str,
) -> Option<&'a rbx_reflection::PropertyDescriptor<'a>> {
    let mut current = class_name;
    loop {
        let class_data = database.classes.get(current)?;
        if let Some(prop_data) = class_data.properties.get(prop_name) {
            return Some(prop_data);
        }
        current = class_data.superclass.as_ref()?;
    }
}

/// Check if a property should be serialized (per reflection database).
/// Walks up the class hierarchy to find the property descriptor.
fn should_property_serialize(
    database: &rbx_reflection::ReflectionDatabase,
    class_name: &str,
    prop_name: &str,
) -> bool {
    let mut current = class_name;
    loop {
        let class_data = match database.classes.get(current) {
            Some(data) => data,
            None => return true, // Unknown class — include property
        };
        if let Some(prop_data) = class_data.properties.get(prop_name) {
            return match &prop_data.kind {
                PropertyKind::Alias { alias_for } => {
                    should_property_serialize(database, current, alias_for)
                }
                PropertyKind::Canonical { serialization } => {
                    !matches!(serialization, PropertySerialization::DoesNotSerialize)
                }
                _ => true,
            };
        } else if let Some(super_class) = class_data.superclass.as_ref() {
            current = super_class;
        } else {
            break;
        }
    }
    true // Property not found in reflection — include it
}

/// Compare two Variants for equality with float tolerance.
/// Used to detect default-valued properties.
fn variant_eq(a: &Variant, b: &Variant) -> bool {
    if std::mem::discriminant(a) != std::mem::discriminant(b) {
        return false;
    }
    match (a, b) {
        (Variant::Float32(x), Variant::Float32(y)) => {
            x == y || (x.is_nan() && y.is_nan()) || (x - y).abs() < 0.001
        }
        (Variant::Float64(x), Variant::Float64(y)) => {
            x == y || (x.is_nan() && y.is_nan()) || (x - y).abs() < 0.001
        }
        (Variant::Vector2(a), Variant::Vector2(b)) => {
            float_eq(a.x, b.x) && float_eq(a.y, b.y)
        }
        (Variant::Vector3(a), Variant::Vector3(b)) => {
            float_eq(a.x, b.x) && float_eq(a.y, b.y) && float_eq(a.z, b.z)
        }
        (Variant::CFrame(a), Variant::CFrame(b)) => {
            float_eq(a.position.x, b.position.x)
                && float_eq(a.position.y, b.position.y)
                && float_eq(a.position.z, b.position.z)
                && float_eq(a.orientation.x.x, b.orientation.x.x)
                && float_eq(a.orientation.x.y, b.orientation.x.y)
                && float_eq(a.orientation.x.z, b.orientation.x.z)
                && float_eq(a.orientation.y.x, b.orientation.y.x)
                && float_eq(a.orientation.y.y, b.orientation.y.y)
                && float_eq(a.orientation.y.z, b.orientation.y.z)
                && float_eq(a.orientation.z.x, b.orientation.z.x)
                && float_eq(a.orientation.z.y, b.orientation.z.y)
                && float_eq(a.orientation.z.z, b.orientation.z.z)
        }
        (Variant::Color3(a), Variant::Color3(b)) => {
            float_eq(a.r, b.r) && float_eq(a.g, b.g) && float_eq(a.b, b.b)
        }
        (Variant::UDim(a), Variant::UDim(b)) => {
            float_eq(a.scale, b.scale) && a.offset == b.offset
        }
        (Variant::UDim2(a), Variant::UDim2(b)) => {
            float_eq(a.x.scale, b.x.scale)
                && a.x.offset == b.x.offset
                && float_eq(a.y.scale, b.y.scale)
                && a.y.offset == b.y.offset
        }
        (Variant::NumberRange(a), Variant::NumberRange(b)) => {
            float_eq(a.min, b.min) && float_eq(a.max, b.max)
        }
        (Variant::Rect(a), Variant::Rect(b)) => {
            float_eq(a.min.x, b.min.x)
                && float_eq(a.min.y, b.min.y)
                && float_eq(a.max.x, b.max.x)
                && float_eq(a.max.y, b.max.y)
        }
        (Variant::Attributes(a), Variant::Attributes(b)) => {
            if a.len() != b.len() {
                return false;
            }
            a.iter().zip(b.iter()).all(|((an, av), (bn, bv))| an == bn && variant_eq(av, bv))
        }
        (Variant::Tags(a), Variant::Tags(b)) => {
            a.iter().count() == b.iter().count()
                && a.iter().zip(b.iter()).all(|(x, y)| x == y)
        }
        // Everything else: exact equality
        _ => a == b,
    }
}

#[inline]
fn float_eq(a: f32, b: f32) -> bool {
    a == b || (a.is_nan() && b.is_nan()) || (a - b).abs() < 0.001
}

/// Convert a Variant to a structured PropertyValue
fn to_property_value(v: &rbx_types::Variant) -> PropertyValue {
    use rbx_types::Variant;
    match v {
        Variant::Bool(b) => PropertyValue::Bool { value: *b },
        Variant::Int32(n) => PropertyValue::Int { value: *n as i64 },
        Variant::Int64(n) => PropertyValue::Int { value: *n },
        Variant::Float32(n) => PropertyValue::Float { value: *n as f64 },
        Variant::Float64(n) => PropertyValue::Float { value: *n },
        Variant::String(s) => PropertyValue::String { value: s.clone() },
        Variant::BinaryString(bs) => PropertyValue::Binary { size: bs.clone().into_vec().len() },
        Variant::Vector2(vec) => PropertyValue::Vector2 {
            x: vec.x as f64,
            y: vec.y as f64,
        },
        Variant::Vector3(vec) => PropertyValue::Vector3 {
            x: vec.x as f64,
            y: vec.y as f64,
            z: vec.z as f64,
        },
        Variant::CFrame(cf) => {
            let matrix = cf.orientation;
            let pitch = (-matrix.y.z).asin();
            let yaw = matrix.y.x.atan2(matrix.y.y);
            let roll = matrix.x.z.atan2(matrix.z.z);
            PropertyValue::CFrame {
                position: [cf.position.x as f64, cf.position.y as f64, cf.position.z as f64],
                orientation: [
                    roll.to_degrees() as f64,
                    pitch.to_degrees() as f64,
                    yaw.to_degrees() as f64,
                ],
            }
        }
        Variant::Color3(c) => PropertyValue::Color3 {
            r: c.r as f64,
            g: c.g as f64,
            b: c.b as f64,
        },
        Variant::BrickColor(bc) => {
            let color = bc.to_color3uint8();
            PropertyValue::BrickColor {
                name: format!("{:?}", bc),
                r: color.r,
                g: color.g,
                b: color.b,
            }
        }
        Variant::Enum(e) => PropertyValue::Enum {
            value: e.to_u32(),
            name: None,
        },
        Variant::Ref(r) => PropertyValue::Ref {
            value: if r.is_none() {
                None
            } else {
                Some(format!("{}", r))
            },
        },
        Variant::Attributes(attrs) => {
            let entries: Vec<AttributeEntry> = attrs
                .iter()
                .map(|(name, value)| AttributeEntry {
                    name: name.to_string(),
                    value: to_property_value(value),
                })
                .collect();
            PropertyValue::Attributes { entries }
        }
        Variant::Tags(tags) => PropertyValue::Tags {
            values: tags.iter().map(|s| s.to_string()).collect(),
        },
        _ => PropertyValue::Unknown {
            display: format!("{:?}", v.ty()),
        },
    }
}

