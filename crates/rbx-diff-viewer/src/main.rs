//! rbx-diff-viewer: A visual diff viewer for Roblox files.
//! Generates a self-contained HTML file with embedded diff data.

use anyhow::{bail, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use clap::Parser;
use flate2::write::GzEncoder;
use flate2::Compression;
use rbx_diff::{diff_doms, DiffEntry};
use rbx_dom_weak::{types::Ref, WeakDom};
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

#[derive(Serialize)]
struct RefInfoEntry {
    name: String,
    path: String,
    class: String,
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
    #[serde(rename = "oldRefInfo")]
    old_ref_info: HashMap<String, RefInfoEntry>,
    #[serde(rename = "newRefInfo")]
    new_ref_info: HashMap<String, RefInfoEntry>,
}

// ============================================================================
// Main
// ============================================================================

fn main() -> Result<()> {
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

    // Build ref info maps
    let old_ref_info: HashMap<String, RefInfoEntry> =
        collect_ref_info(&old_dom, old_dom.root_ref(), "").into_iter().collect();
    let new_ref_info: HashMap<String, RefInfoEntry> =
        collect_ref_info(&new_dom, new_dom.root_ref(), "").into_iter().collect();

    // Build properties maps for all instances
    let old_properties = collect_all_properties(&old_dom);
    let new_properties = collect_all_properties(&new_dom);

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
        old_ref_info,
        new_ref_info,
    };

    eprintln!("Generating HTML...");

    // Serialize core data to JSON (small, fast to parse)
    let core_json = serde_json::to_string(&core_data)?;

    // Serialize and compress properties separately (large, lazy loaded)
    eprintln!("Compressing properties...");
    let old_props_json = serde_json::to_string(&old_properties)?;
    let new_props_json = serde_json::to_string(&new_properties)?;

    let old_props_compressed = gzip_compress(old_props_json.as_bytes());
    let new_props_compressed = gzip_compress(new_props_json.as_bytes());

    let old_props_b64 = BASE64.encode(&old_props_compressed);
    let new_props_b64 = BASE64.encode(&new_props_compressed);

    eprintln!(
        "Properties: old {}MB -> {}MB, new {}MB -> {}MB (compressed)",
        old_props_json.len() / 1024 / 1024,
        old_props_compressed.len() / 1024 / 1024,
        new_props_json.len() / 1024 / 1024,
        new_props_compressed.len() / 1024 / 1024,
    );

    // Load HTML template and inject data
    let html_template = include_str!("../dist/index.html");

    // Inject core data (parsed immediately)
    let output_html = html_template.replace(
        "/*__DIFF_DATA_PLACEHOLDER__*/",
        &format!("window.__DIFF_DATA__ = {}", core_json),
    );

    // Inject compressed properties (lazy loaded)
    let output_html = output_html.replace(
        "/*__OLD_PROPERTIES_PLACEHOLDER__*/",
        &format!("window.__OLD_PROPERTIES_B64__ = \"{}\"", old_props_b64),
    );
    let output_html = output_html.replace(
        "/*__NEW_PROPERTIES_PLACEHOLDER__*/",
        &format!("window.__NEW_PROPERTIES_B64__ = \"{}\"", new_props_b64),
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

/// Collect all ref->name/path mappings from a DOM tree
fn collect_ref_info(dom: &WeakDom, referent: Ref, parent_path: &str) -> Vec<(String, RefInfoEntry)> {
    let mut results = Vec::new();

    if let Some(inst) = dom.get_by_ref(referent) {
        let path = if parent_path.is_empty() {
            inst.name.clone()
        } else {
            format!("{}/{}", parent_path, inst.name)
        };

        results.push((
            format!("{}", referent),
            RefInfoEntry {
                name: inst.name.clone(),
                path: path.clone(),
                class: inst.class.to_string(),
            },
        ));

        for &child_ref in inst.children() {
            results.extend(collect_ref_info(dom, child_ref, &path));
        }
    }

    results
}

/// Collect properties for all instances in a DOM
fn collect_all_properties(dom: &WeakDom) -> HashMap<String, Vec<Property>> {
    let mut result = HashMap::new();
    collect_properties_recursive(dom, dom.root_ref(), &mut result);
    result
}

fn collect_properties_recursive(dom: &WeakDom, referent: Ref, result: &mut HashMap<String, Vec<Property>>) {
    if let Some(inst) = dom.get_by_ref(referent) {
        let mut props: Vec<Property> = inst
            .properties
            .iter()
            .map(|(name, value)| Property {
                name: name.to_string(),
                value: to_property_value(value),
                prop_type: format!("{:?}", value.ty()),
                category: get_property_category(name).to_string(),
                read_only: is_property_read_only(name),
            })
            .collect();

        // Sort by category, then name
        props.sort_by(|a, b| {
            let cat_cmp = category_order(&a.category).cmp(&category_order(&b.category));
            if cat_cmp == std::cmp::Ordering::Equal {
                a.name.cmp(&b.name)
            } else {
                cat_cmp
            }
        });

        result.insert(format!("{}", referent), props);

        for &child_ref in inst.children() {
            collect_properties_recursive(dom, child_ref, result);
        }
    }
}

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
