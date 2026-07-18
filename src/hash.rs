//! Instance hashing for content-based comparison.
//!
//! Two cache types:
//! - `LazyHashCache`: Shallow hash (name + class + properties). Used for matching disambiguation.
//! - `DeepHashCache`: Subtree hash (shallow hash + children's deep hashes). Used for Ref
//!   comparison and subtree pruning in the diff pass.

use blake3::{Hash, Hasher};
use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_reflection::{PropertyKind, PropertySerialization, Scriptability};
use rbx_types::{PhysicalProperties, Variant, Vector3};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use tracing::info;

// ============================================================================
// Helper macros (must be defined before use)
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

fn vector_hash(hasher: &mut Hasher, vector: Vector3) {
    n_hash!(hasher, round!(vector.x), round!(vector.y), round!(vector.z))
}

/// Normalize a Roblox asset URI to a canonical form so different spellings of
/// the same asset compare equal — Studio rewrites e.g.
/// `http://www.roblox.com/asset/?id=123` to `rbxassetid://123` on save.
pub(crate) fn normalize_asset_uri(uri: &str) -> String {
    let s = uri.trim();
    let lower = s.to_ascii_lowercase();

    let digits_at = |start: usize| -> String {
        s[start..].chars().take_while(|c| c.is_ascii_digit()).collect()
    };

    if let Some(pos) = lower.find("roblox.com/asset") {
        if let Some(id_pos) = lower[pos..].find("id=") {
            let digits = digits_at(pos + id_pos + 3);
            if !digits.is_empty() {
                return format!("rbxassetid://{digits}");
            }
        }
    }
    if let Some(rest_pos) = lower.strip_prefix("rbxassetid://").map(|_| "rbxassetid://".len()) {
        let digits = digits_at(rest_pos);
        if !digits.is_empty() {
            return format!("rbxassetid://{digits}");
        }
    }
    s.to_string()
}

/// Lazy shallow hash cache - computes hashes on demand.
/// Provides two hash variants for multi-pass matching:
/// - `get()`: Full hash (all properties including Refs)
/// - `get_no_refs()`: Hash excluding Ref properties (stable when only Refs change)
pub struct LazyHashCache<'a> {
    dom: &'a WeakDom,
    cache: RefCell<HashMap<Ref, Hash>>,
    cache_no_refs: RefCell<HashMap<Ref, Hash>>,
}

impl<'a> LazyHashCache<'a> {
    /// Create a new lazy hash cache for the given DOM.
    pub fn new(dom: &'a WeakDom) -> Self {
        Self {
            dom,
            cache: RefCell::new(HashMap::new()),
            cache_no_refs: RefCell::new(HashMap::new()),
        }
    }

    /// Get hash for an instance, computing it if needed.
    pub fn get(&self, referent: Ref) -> Hash {
        if let Some(hash) = self.cache.borrow().get(&referent) {
            return *hash;
        }

        let hash = self.compute_hash(referent);
        self.cache.borrow_mut().insert(referent, hash);
        hash
    }

    /// Get hash excluding Ref properties, computing it if needed.
    /// Stable when only Ref properties (like PrimaryPart) change.
    pub fn get_no_refs(&self, referent: Ref) -> Hash {
        if let Some(hash) = self.cache_no_refs.borrow().get(&referent) {
            return *hash;
        }

        let hash = self.compute_hash_no_refs(referent);
        self.cache_no_refs.borrow_mut().insert(referent, hash);
        hash
    }

    /// Log cache stats.
    pub fn log_stats(&self, label: &str) {
        info!(
            label = label,
            cached = self.cache.borrow().len(),
            cached_no_refs = self.cache_no_refs.borrow().len(),
            "hash cache stats"
        );
    }

    fn compute_hash(&self, referent: Ref) -> Hash {
        let inst = self.dom.get_by_ref(referent).unwrap();
        let comparable = get_comparable_properties(&inst.class);
        let mut hasher = Hasher::new();

        hasher.update(inst.name.as_bytes());
        hasher.update(inst.class.as_bytes());

        // Sort properties by name for deterministic hashing
        let mut props: Vec<_> = inst.properties.iter().collect();
        props.sort_unstable_by_key(|(name, _)| name.as_str());

        for (name, value) in props {
            if !comparable.contains(name.as_str()) {
                continue;
            }
            hasher.update(name.as_bytes());
            hash_variant(&self.dom, &mut hasher, value);
        }

        hasher.finalize()
    }

    fn compute_hash_no_refs(&self, referent: Ref) -> Hash {
        let inst = self.dom.get_by_ref(referent).unwrap();
        let comparable = get_comparable_properties(&inst.class);
        let mut hasher = Hasher::new();

        hasher.update(inst.name.as_bytes());
        hasher.update(inst.class.as_bytes());

        // Sort properties by name for deterministic hashing
        let mut props: Vec<_> = inst.properties.iter().collect();
        props.sort_unstable_by_key(|(name, _)| name.as_str());

        for (name, value) in props {
            // Skip Ref properties — they change when targets are reassigned
            if matches!(value, Variant::Ref(_)) {
                continue;
            }
            if !comparable.contains(name.as_str()) {
                continue;
            }
            hasher.update(name.as_bytes());
            hash_variant(&self.dom, &mut hasher, value);
        }

        hasher.finalize()
    }
}

/// Deep subtree hash cache — includes children's hashes.
/// Used for Ref comparison (compare target content instead of ref_mapping)
/// and for subtree pruning (if deep hashes match, entire subtree is unchanged).
///
/// Computed lazily, bottom-up: requesting a parent's hash first computes all children.
/// Skips ignored properties (e.g. UniqueId, HistoryId) so pruning works correctly.
pub struct DeepHashCache<'a> {
    dom: &'a WeakDom,
    ignore_properties: &'a HashSet<String>,
    cache: RefCell<HashMap<Ref, Hash>>,
}

impl<'a> DeepHashCache<'a> {
    pub fn new(dom: &'a WeakDom, ignore_properties: &'a HashSet<String>) -> Self {
        Self {
            dom,
            ignore_properties,
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// Get deep hash for an instance, computing bottom-up if needed.
    pub fn get(&self, referent: Ref) -> Hash {
        if let Some(hash) = self.cache.borrow().get(&referent) {
            return *hash;
        }
        self.compute(referent)
    }

    fn compute(&self, referent: Ref) -> Hash {
        let inst = self.dom.get_by_ref(referent).unwrap();
        let comparable = get_comparable_properties(&inst.class);
        let mut hasher = Hasher::new();

        // Hash own identity + properties
        hasher.update(inst.name.as_bytes());
        hasher.update(inst.class.as_bytes());

        let mut props: Vec<_> = inst.properties.iter().collect();
        props.sort_unstable_by_key(|(name, _)| name.as_str());
        for (name, value) in props {
            if self.ignore_properties.contains(name.as_str()) {
                continue;
            }
            if !comparable.contains(name.as_str()) {
                continue;
            }
            hasher.update(name.as_bytes());
            hash_variant(self.dom, &mut hasher, value);
        }

        // Incorporate children's deep hashes (in order — order matters)
        let children = inst.children().to_vec();
        for child_ref in children {
            let child_hash = self.get(child_ref);
            hasher.update(child_hash.as_bytes());
        }

        let hash = hasher.finalize();
        self.cache.borrow_mut().insert(referent, hash);
        hash
    }
}

/// Hash a variant value.
/// For Ref properties, uses name+class of the target as a stable identifier.
pub(crate) fn hash_variant(dom: &WeakDom, hasher: &mut Hasher, value: &Variant) {
    match value {
        Variant::Attributes(attrs) => {
            let mut sorted: Vec<_> = attrs.iter().collect();
            sorted.sort_unstable_by_key(|(name, _)| name.as_str());
            for (name, attribute) in sorted {
                hasher.update(name.as_bytes());
                hash_variant(dom, hasher, attribute);
            }
        }
        Variant::Axes(a) => { hasher.update(&[a.bits()]); }
        Variant::BinaryString(bytes) => {
            let b: &[u8] = bytes.as_ref();
            hasher.update(b);
        }
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
        Variant::Content(content) => {
            use rbx_types::ContentType;
            match content.value() {
                ContentType::None => { hasher.update(&[0x00]); }
                ContentType::Uri(uri) => {
                    hasher.update(&[0x01]);
                    hasher.update(normalize_asset_uri(uri).as_bytes());
                }
                // Object refs point at DOM instances; skip like Variant::Ref
                ContentType::Object(_) => { hasher.update(&[0x02]); }
                _ => { hasher.update(&[0x03]); }
            }
        }
        Variant::ContentId(id) => {
            hasher.update(normalize_asset_uri(id.as_str()).as_bytes());
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
        Variant::Ref(target_ref) => {
            if target_ref.is_none() {
                hasher.update(&[0x00]); // null ref
            } else if let Some(target) = dom.get_by_ref(*target_ref) {
                // Use name+class as stable identifier (no subtree hashing)
                hasher.update(&[0x01]);
                hasher.update(target.name.as_bytes());
                hasher.update(target.class.as_bytes());
            } else {
                hasher.update(&[0x00]); // invalid ref = null
            }
        }
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
        Variant::UniqueId(_) => {} // Skip - non-deterministic
        _ => {} // Skip unknown variants
    }
}

/// Get the set of comparable property names for a class.
/// Caches per class — builds the set once by walking the class hierarchy.
/// Properties NOT in this set should be skipped (non-reflected, non-scriptable, non-serializable).
pub fn get_comparable_properties(class_name: &str) -> &'static HashSet<String> {
    use std::sync::OnceLock;
    use std::sync::Mutex;

    static CLASS_PROPS: OnceLock<Mutex<HashMap<String, HashSet<String>>>> = OnceLock::new();

    let map_mutex = CLASS_PROPS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map_mutex.lock().unwrap();

    if !map.contains_key(class_name) {
        let props = build_comparable_properties(class_name);
        map.insert(class_name.to_string(), props);
    }

    // Safety: we never remove entries, only add — pointer remains valid
    let ptr = map.get(class_name).unwrap() as *const HashSet<String>;
    // Release the lock before returning
    drop(map);
    unsafe { &*ptr }
}

/// Serialized-but-not-scriptable properties that carry real user content.
/// The scriptability filter below would drop these, making the diff blind to
/// CSG edits, terrain sculpting, and collision group changes. Derived/volatile
/// None-scriptability props (PhysicsData, UnscaledVolume, UniqueId, ...) stay
/// excluded — extend this list for new cases rather than widening the filter.
const CONTENT_PROPERTY_EXCEPTIONS: &[(&str, &[&str])] = &[
    ("PartOperation", &["MeshData", "MeshData2", "ChildData", "ChildData2", "AssetId"]),
    ("Terrain", &["SmoothGrid", "Decoration"]),
    ("Workspace", &["CollisionGroupData"]),
];

fn build_comparable_properties(class_name: &str) -> HashSet<String> {
    let database = rbx_reflection_database::get().unwrap();
    let mut result = HashSet::new();

    // Walk up the class hierarchy, collecting all comparable properties
    let mut current_class = class_name;
    loop {
        let class_data = match database.classes.get(current_class) {
            Some(data) => data,
            None => break,
        };

        for (class, props) in CONTENT_PROPERTY_EXCEPTIONS {
            if *class == current_class {
                for prop in *props {
                    result.insert((*prop).to_string());
                }
            }
        }

        for (prop_name, prop_data) in &class_data.properties {
            // Skip non-scriptable and read-only properties: users can't set them
            // and a merge can't apply them (e.g. UnionOperation.TriangleCount)
            if matches!(prop_data.scriptability, Scriptability::None | Scriptability::Read) {
                continue;
            }
            let dominated = match &prop_data.kind {
                PropertyKind::Canonical { serialization } => {
                    matches!(serialization, PropertySerialization::DoesNotSerialize)
                }
                PropertyKind::Alias { .. } => continue, // Skip aliases, canonical will be found
                _ => false,
            };
            if !dominated {
                result.insert(prop_name.to_string());
            }
        }

        match class_data.superclass.as_ref() {
            Some(super_class) => current_class = super_class,
            None => break,
        }
    }

    result
}
