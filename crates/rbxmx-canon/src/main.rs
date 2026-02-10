use anyhow::{bail, Result};
use blake3::{Hash, Hasher};
use clap::Parser;
use rayon::prelude::*;
use rbx_dom_weak::{types::Ref, InstanceBuilder, WeakDom};
use rbx_types::{PhysicalProperties, Variant, Vector3};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::sync::RwLock;

#[derive(Parser)]
#[command(name = "rbxmx-canon")]
#[command(about = "Canonicalize rbxm/rbxmx files for deterministic diffs")]
struct Args {
    /// Input rbxm or rbxmx file
    input: String,

    /// Output file (defaults to input path with .rbxmx extension)
    #[arg(short, long)]
    output: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let input_path = Path::new(&args.input);
    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    // Load the file based on extension
    let file = BufReader::new(File::open(&args.input)?);
    let source_dom = match ext.to_lowercase().as_str() {
        "rbxm" => rbx_binary::from_reader(file)?,
        "rbxmx" => rbx_xml::from_reader_default(file)?,
        _ => bail!("Unknown file extension: {}. Expected .rbxm or .rbxmx", ext),
    };

    // Pass 1: Compute hashes for all instances (bottom-up)
    let hashes = compute_all_hashes(&source_dom);

    // Pass 2: Rebuild DOM with sorted children using hashes as tiebreaker
    let new_dom = rebuild_with_sorted_children(&source_dom, &hashes);

    // Pass 3: Build ref mapping by walking both DOMs in parallel
    let ref_mapping = build_ref_mapping(&source_dom, &new_dom, &hashes);

    // Pass 4: Update all Ref properties using the mapping
    let mut canonical_dom = new_dom;
    update_ref_properties(&mut canonical_dom, &ref_mapping);

    // Determine output path (default: same name with .rbxmx extension)
    let output_path = args.output.unwrap_or_else(|| {
        input_path.with_extension("rbxmx").to_string_lossy().to_string()
    });
    let file = BufWriter::new(File::create(&output_path)?);
    rbx_xml::to_writer_default(file, &canonical_dom, canonical_dom.root().children())?;

    eprintln!("Wrote canonicalized output to {}", output_path);

    Ok(())
}

// ============================================================================
// Hash computation (adapted from rojo's syncback/hash) - PARALLELIZED
// ============================================================================

/// Compute hashes for all instances in the DOM, bottom-up, using parallel processing.
/// Children are hashed before parents so parent hashes include child hashes.
/// Uses level-based parallelism: instances at the same depth are hashed in parallel.
fn compute_all_hashes(dom: &WeakDom) -> HashMap<Ref, Hash> {
    // Group instances by depth level
    let levels = get_instances_by_level(dom, dom.root_ref());
    let map: RwLock<HashMap<Ref, Hash>> = RwLock::new(HashMap::with_capacity(
        levels.iter().map(|l| l.len()).sum(),
    ));

    // Process levels from deepest to shallowest (children first)
    for level in levels.into_iter().rev() {
        level.into_par_iter().for_each(|referent| {
            let inst = dom.get_by_ref(referent).unwrap();
            let mut hasher = hash_instance(inst);

            // Read child hashes (already computed in previous iteration)
            let map_read = map.read().unwrap();
            let mut child_hashes: Vec<[u8; 32]> = inst
                .children()
                .iter()
                .filter_map(|r| map_read.get(r).map(|h| *h.as_bytes()))
                .collect();
            drop(map_read);

            child_hashes.sort_unstable();
            for hash in child_hashes {
                hasher.update(&hash);
            }

            map.write().unwrap().insert(referent, hasher.finalize());
        });
    }

    map.into_inner().unwrap()
}

/// Get all instances grouped by their depth level (BFS order).
/// Level 0 is the root, level 1 is root's children, etc.
fn get_instances_by_level(dom: &WeakDom, root: Ref) -> Vec<Vec<Ref>> {
    let mut levels: Vec<Vec<Ref>> = Vec::new();
    let mut current_level = vec![root];

    while !current_level.is_empty() {
        let mut next_level = Vec::new();
        for referent in &current_level {
            if let Some(inst) = dom.get_by_ref(*referent) {
                next_level.extend(inst.children().iter().copied());
            }
        }
        levels.push(current_level);
        current_level = next_level;
    }

    levels
}

/// Hash a single instance (name, class, properties)
fn hash_instance(inst: &rbx_dom_weak::Instance) -> Hasher {
    let mut hasher = Hasher::new();
    hasher.update(inst.name.as_bytes());
    hasher.update(inst.class.as_bytes());

    // Sort properties by name for deterministic hashing
    let mut props: Vec<_> = inst.properties.iter().collect();
    props.sort_unstable_by_key(|(name, _)| name.as_str());

    for (name, value) in props {
        hasher.update(name.as_bytes());
        hash_variant(&mut hasher, value);
    }

    hasher
}

// ============================================================================
// Variant hashing (adapted from rojo's syncback/hash/variant.rs)
// ============================================================================

macro_rules! round {
    ($value:expr) => {
        (($value * 10.0).round() / 10.0)
    };
}

macro_rules! n_hash {
    ($hash:ident, $($num:expr),*) => {
        {$(
            $hash.update(&($num).to_le_bytes());
        )*}
    };
}

fn hash_variant(hasher: &mut Hasher, value: &Variant) {
    match value {
        Variant::Attributes(attrs) => {
            let mut sorted: Vec<_> = attrs.iter().collect();
            sorted.sort_unstable_by_key(|(name, _)| name.as_str());
            for (name, attribute) in sorted {
                hasher.update(name.as_bytes());
                hash_variant(hasher, attribute);
            }
        }
        Variant::Axes(a) => { hasher.update(&[a.bits()]); }
        Variant::BinaryString(bytes) => { hasher.update(bytes.as_ref()); }
        Variant::Bool(b) => { hasher.update(&[*b as u8]); }
        Variant::BrickColor(color) => { n_hash!(hasher, *color as u16); }
        Variant::CFrame(cf) => {
            vector_hash(hasher, cf.position);
            vector_hash(hasher, cf.orientation.x);
            vector_hash(hasher, cf.orientation.y);
            vector_hash(hasher, cf.orientation.z);
        }
        Variant::Color3(color) => {
            n_hash!(hasher, round!(color.r), round!(color.g), round!(color.b));
        }
        Variant::Color3uint8(color) => { hasher.update(&[color.r, color.g, color.b]); }
        Variant::ColorSequence(seq) => {
            let mut keypoints: Vec<_> = seq.keypoints.iter().collect();
            keypoints.sort_unstable_by(|a, b| {
                round!(a.time).partial_cmp(&round!(b.time)).unwrap()
            });
            for kp in keypoints {
                n_hash!(hasher, round!(kp.time), round!(kp.color.r), round!(kp.color.g), round!(kp.color.b));
            }
        }
        Variant::Content(_content) => {
            // Content type doesn't have a simple string representation in this version
            // Hash a marker byte
            hasher.update(&[0x01]);
        }
        Variant::Enum(e) => { n_hash!(hasher, e.to_u32()); }
        Variant::Faces(f) => { hasher.update(&[f.bits()]); }
        Variant::Float32(n) => { n_hash!(hasher, round!(*n)); }
        Variant::Float64(n) => { n_hash!(hasher, round!(n)); }
        Variant::Font(f) => {
            n_hash!(hasher, f.weight as u16, f.style as u8);
            hasher.update(f.family.as_bytes());
        }
        Variant::Int32(n) => { n_hash!(hasher, n); }
        Variant::Int64(n) => { n_hash!(hasher, n); }
        Variant::NumberRange(nr) => { n_hash!(hasher, round!(nr.max), round!(nr.min)); }
        Variant::NumberSequence(seq) => {
            let mut keypoints: Vec<_> = seq.keypoints.iter().collect();
            keypoints.sort_unstable_by(|a, b| {
                round!(a.time).partial_cmp(&round!(b.time)).unwrap()
            });
            for kp in keypoints {
                n_hash!(hasher, round!(kp.time), round!(kp.value), round!(kp.envelope));
            }
        }
        Variant::OptionalCFrame(maybe_cf) => {
            if let Some(cf) = maybe_cf {
                hasher.update(&[0x01]);
                vector_hash(hasher, cf.position);
                vector_hash(hasher, cf.orientation.x);
                vector_hash(hasher, cf.orientation.y);
                vector_hash(hasher, cf.orientation.z);
            } else {
                hasher.update(&[0x00]);
            }
        }
        Variant::PhysicalProperties(properties) => {
            match properties {
                PhysicalProperties::Default => { hasher.update(&[0x00]); }
                PhysicalProperties::Custom(custom) => {
                    hasher.update(&[0x01]);
                    n_hash!(
                        hasher,
                        round!(custom.density()),
                        round!(custom.friction()),
                        round!(custom.elasticity()),
                        round!(custom.friction_weight()),
                        round!(custom.elasticity_weight())
                    );
                }
            }
        }
        Variant::Ray(ray) => {
            vector_hash(hasher, ray.origin);
            vector_hash(hasher, ray.direction);
        }
        Variant::Rect(rect) => {
            n_hash!(
                hasher,
                round!(rect.max.x),
                round!(rect.max.y),
                round!(rect.min.x),
                round!(rect.min.y)
            );
        }
        Variant::Ref(_) => {} // Skip Ref properties - they contain non-deterministic source IDs
        Variant::Region3(region) => {
            vector_hash(hasher, region.max);
            vector_hash(hasher, region.min);
        }
        Variant::Region3int16(region) => {
            n_hash!(
                hasher,
                region.max.x, region.max.y, region.max.z,
                region.min.x, region.min.y, region.min.z
            );
        }
        Variant::SecurityCapabilities(caps) => { n_hash!(hasher, caps.bits()); }
        Variant::SharedString(sstr) => { hasher.update(sstr.hash().as_bytes()); }
        Variant::String(s) => { hasher.update(s.as_bytes()); }
        Variant::Tags(tags) => {
            let mut sorted: Vec<&str> = tags.iter().collect();
            sorted.sort_unstable();
            for tag in sorted {
                hasher.update(tag.as_bytes());
            }
        }
        Variant::UDim(udim) => { n_hash!(hasher, round!(udim.scale), udim.offset); }
        Variant::UDim2(udim) => {
            n_hash!(
                hasher,
                round!(udim.x.scale), udim.x.offset,
                round!(udim.y.scale), udim.y.offset
            );
        }
        Variant::Vector2(v2) => { n_hash!(hasher, round!(v2.x), round!(v2.y)); }
        Variant::Vector2int16(v2) => { n_hash!(hasher, v2.x, v2.y); }
        Variant::Vector3(v3) => { vector_hash(hasher, *v3); }
        Variant::Vector3int16(v3) => { n_hash!(hasher, v3.x, v3.y, v3.z); }
        Variant::UniqueId(_) => {} // Skip UniqueId as it's not deterministic
        _ => {} // Skip unknown variants
    }
}

fn vector_hash(hasher: &mut Hasher, vector: Vector3) {
    n_hash!(hasher, round!(vector.x), round!(vector.y), round!(vector.z))
}

// ============================================================================
// DOM rebuilding with sorted children
// ============================================================================

/// Rebuild the DOM with children sorted by (name, class, hash) at each level
fn rebuild_with_sorted_children(source: &WeakDom, hashes: &HashMap<Ref, Hash>) -> WeakDom {
    let root = source.root();
    let root_builder = build_instance_recursive(source, root.referent(), hashes);
    WeakDom::new(root_builder)
}

/// Recursively build an InstanceBuilder with sorted children
fn build_instance_recursive(
    source: &WeakDom,
    referent: Ref,
    hashes: &HashMap<Ref, Hash>,
) -> InstanceBuilder {
    let instance = source.get_by_ref(referent).unwrap();

    let mut builder = InstanceBuilder::new(instance.class.as_str())
        .with_name(&instance.name);

    // Copy all properties
    for (name, value) in &instance.properties {
        builder = builder.with_property(name.as_str(), value.clone());
    }

    // Get children and sort by (name, class, hash)
    let mut children_info: Vec<_> = instance
        .children()
        .iter()
        .filter_map(|&child_ref| {
            source.get_by_ref(child_ref).map(|child| {
                let hash = hashes.get(&child_ref).map(|h| *h.as_bytes()).unwrap_or([0; 32]);
                (child_ref, child.name.as_str(), child.class.as_str(), hash)
            })
        })
        .collect();

    // Sort by (name, class, hash) for full determinism
    children_info.sort_by(|a, b| {
        (a.1, a.2, &a.3).cmp(&(b.1, b.2, &b.3))
    });

    let child_builders: Vec<InstanceBuilder> = children_info
        .into_iter()
        .map(|(child_ref, _, _, _)| build_instance_recursive(source, child_ref, hashes))
        .collect();

    builder.with_children(child_builders)
}

// ============================================================================
// Ref mapping
// ============================================================================

/// Build a mapping from source refs to destination refs
fn build_ref_mapping(
    source: &WeakDom,
    dest: &WeakDom,
    hashes: &HashMap<Ref, Hash>,
) -> HashMap<Ref, Ref> {
    let mut mapping = HashMap::new();
    build_ref_mapping_recursive(source, dest, source.root_ref(), dest.root_ref(), hashes, &mut mapping);
    mapping
}

fn build_ref_mapping_recursive(
    source: &WeakDom,
    dest: &WeakDom,
    source_ref: Ref,
    dest_ref: Ref,
    hashes: &HashMap<Ref, Hash>,
    mapping: &mut HashMap<Ref, Ref>,
) {
    mapping.insert(source_ref, dest_ref);

    let source_inst = source.get_by_ref(source_ref).unwrap();
    let dest_inst = dest.get_by_ref(dest_ref).unwrap();

    // Get sorted children from source (same order as we built them)
    let mut source_children: Vec<_> = source_inst.children()
        .iter()
        .filter_map(|&r| {
            source.get_by_ref(r).map(|i| {
                let hash = hashes.get(&r).map(|h| *h.as_bytes()).unwrap_or([0; 32]);
                (r, i.name.as_str(), i.class.as_str(), hash)
            })
        })
        .collect();
    source_children.sort_by(|a, b| (a.1, a.2, &a.3).cmp(&(b.1, b.2, &b.3)));

    let dest_children = dest_inst.children();

    for (i, (source_child_ref, _, _, _)) in source_children.iter().enumerate() {
        if i < dest_children.len() {
            build_ref_mapping_recursive(source, dest, *source_child_ref, dest_children[i], hashes, mapping);
        }
    }
}

/// Update all Ref properties in the DOM using the old->new mapping
fn update_ref_properties(dom: &mut WeakDom, ref_mapping: &HashMap<Ref, Ref>) {
    let all_refs: Vec<Ref> = dom.descendants().map(|i| i.referent()).collect();

    for inst_ref in all_refs {
        if let Some(inst) = dom.get_by_ref_mut(inst_ref) {
            for (_name, value) in inst.properties.iter_mut() {
                if let Variant::Ref(old_ref) = value {
                    if let Some(&new_ref) = ref_mapping.get(old_ref) {
                        *value = Variant::Ref(new_ref);
                    }
                }
            }
        }
    }
}
