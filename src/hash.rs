//! Instance hashing for content-based comparison.
//!
//! Two cache types:
//! - `LazyHashCache`: Shallow hash (name + class + properties). Used for matching disambiguation.
//! - `DeepHashCache`: Subtree hash (shallow hash + children's deep hashes). Used for Ref
//!   comparison and subtree pruning in the diff pass.

use blake3::{Hash, Hasher};
use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::{PhysicalProperties, Variant, Vector3};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use tracing::info;

use crate::diff_dom::{DomView, InstanceView};
use crate::property_semantics::{get_authored_properties, normalize_asset_uri};

macro_rules! n_hash {
    ($hash:ident, $($num:expr),*) => {
        {$(
            $hash.update(&($num).to_le_bytes());
        )*}
    };
}

/// Hash floats exactly as Roblox stores them. Zero has two IEEE encodings and
/// NaN has many; canonicalizing those keeps semantically identical values on
/// one hash without discarding any finite precision.
fn f32_hash(hasher: &mut Hasher, value: f32) {
    let bits = if value == 0.0 {
        0
    } else if value.is_nan() {
        f32::NAN.to_bits()
    } else {
        value.to_bits()
    };
    hasher.update(&bits.to_le_bytes());
}

fn f64_hash(hasher: &mut Hasher, value: f64) {
    let bits = if value == 0.0 {
        0
    } else if value.is_nan() {
        f64::NAN.to_bits()
    } else {
        value.to_bits()
    };
    hasher.update(&bits.to_le_bytes());
}

fn vector_hash(hasher: &mut Hasher, vector: Vector3) {
    f32_hash(hasher, vector.x);
    f32_hash(hasher, vector.y);
    f32_hash(hasher, vector.z);
}

/// The target's position in the tree as child indices from the root,
/// e.g. [2, 0, 3]. Distinguishes same-named siblings for Ref hashing.
fn ref_index_path(dom: &dyn DomView, target: Ref) -> Vec<u32> {
    let mut path = Vec::new();
    let mut current = target;
    while let Some(inst) = dom.get_by_ref(current) {
        let parent = inst.parent();
        let Some(parent_inst) = dom.get_by_ref(parent) else {
            break;
        };
        if let Some(index) = parent_inst.children().position(|child| child == current) {
            path.push(index as u32);
        }
        current = parent;
    }
    path.reverse();
    path
}

#[derive(Clone, Copy)]
struct HashPolicy {
    include_name: bool,
    include_refs: bool,
}

const FULL_HASH: HashPolicy = HashPolicy {
    include_name: true,
    include_refs: true,
};
const NO_REFS_HASH: HashPolicy = HashPolicy {
    include_name: true,
    include_refs: false,
};

enum HashStorage {
    Sparse(HashMap<Ref, Hash>),
    Dense {
        values: Vec<Option<Hash>>,
        populated: usize,
    },
}

impl HashStorage {
    fn new(dom: &dyn DomView) -> Self {
        match dom.dense_len() {
            Some(len) => Self::Dense {
                values: vec![None; len],
                populated: 0,
            },
            None => Self::Sparse(HashMap::new()),
        }
    }

    fn get(&self, instance: InstanceView<'_>) -> Option<Hash> {
        match self {
            Self::Sparse(values) => values.get(&instance.referent()).copied(),
            Self::Dense { values, .. } => {
                values[instance
                    .dense_index()
                    .expect("dense cache received a sparse instance")]
            }
        }
    }

    fn insert(&mut self, instance: InstanceView<'_>, hash: Hash) {
        match self {
            Self::Sparse(values) => {
                values.insert(instance.referent(), hash);
            }
            Self::Dense { values, populated } => {
                let slot = &mut values[instance
                    .dense_index()
                    .expect("dense cache received a sparse instance")];
                if slot.is_none() {
                    *populated += 1;
                }
                *slot = Some(hash);
            }
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Sparse(values) => values.len(),
            Self::Dense { populated, .. } => *populated,
        }
    }
}

/// Hash one instance's class and authored properties under a policy. Deep
/// hashing builds on the same prefix before adding child hashes.
fn hash_instance(
    dom: &dyn DomView,
    inst: InstanceView<'_>,
    ignore_properties: Option<&HashSet<String>>,
    policy: HashPolicy,
) -> Hasher {
    let authored = get_authored_properties(inst.class());
    let mut hasher = Hasher::new();

    if policy.include_name {
        hasher.update(inst.name().as_bytes());
    }
    hasher.update(inst.class().as_bytes());

    let mut hash_property = |name: &str, value: &Variant| {
        if ignore_properties.is_some_and(|ignored| ignored.contains(name))
            || (!policy.include_refs && matches!(value, Variant::Ref(_)))
            || !authored.contains(name)
        {
            return;
        }
        hasher.update(name.as_bytes());
        hash_variant(dom, &mut hasher, value);
    };

    if matches!(inst, InstanceView::Compact(_)) {
        // DiffDom property ranges are sorted during construction, so hashing
        // them directly avoids one temporary Vec and sort per hashed node.
        for (name, value) in inst.properties() {
            hash_property(name, value);
        }
    } else {
        let mut properties: Vec<_> = inst.properties().collect();
        properties.sort_unstable_by_key(|(name, _)| *name);
        for (name, value) in properties {
            hash_property(name, value);
        }
    }

    hasher
}

/// Lazy shallow hash cache - computes hashes on demand.
/// Provides two hash variants for multi-pass matching:
/// - `get()`: Full hash (all properties including Refs)
/// - `get_no_refs()`: Hash excluding Ref properties (stable when only Refs change)
pub struct LazyHashCache<'a> {
    dom: &'a dyn DomView,
    cache: RefCell<HashStorage>,
    cache_no_refs: RefCell<HashStorage>,
}

impl<'a> LazyHashCache<'a> {
    /// Create a new lazy hash cache for the given DOM.
    pub fn new(dom: &'a WeakDom) -> Self {
        Self::new_view(dom)
    }

    pub(crate) fn new_view(dom: &'a dyn DomView) -> Self {
        Self {
            dom,
            cache: RefCell::new(HashStorage::new(dom)),
            cache_no_refs: RefCell::new(HashStorage::new(dom)),
        }
    }

    /// Get hash for an instance, computing it if needed.
    pub fn get(&self, referent: Ref) -> Hash {
        let instance = self.dom.get_by_ref(referent).unwrap();
        self.get_instance_with_policy(instance, FULL_HASH)
    }

    /// Get hash excluding Ref properties, computing it if needed.
    /// Stable when only Ref properties (like PrimaryPart) change.
    pub fn get_no_refs(&self, referent: Ref) -> Hash {
        let instance = self.dom.get_by_ref(referent).unwrap();
        self.get_instance_with_policy(instance, NO_REFS_HASH)
    }

    pub(crate) fn get_instance(&self, instance: InstanceView<'_>) -> Hash {
        self.get_instance_with_policy(instance, FULL_HASH)
    }

    pub(crate) fn get_instance_no_refs(&self, instance: InstanceView<'_>) -> Hash {
        self.get_instance_with_policy(instance, NO_REFS_HASH)
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

    fn get_instance_with_policy(&self, instance: InstanceView<'_>, policy: HashPolicy) -> Hash {
        let cached = if policy.include_refs {
            self.cache.borrow().get(instance)
        } else {
            self.cache_no_refs.borrow().get(instance)
        };
        if let Some(hash) = cached {
            return hash;
        }

        let hash = hash_instance(self.dom, instance, None, policy).finalize();
        if policy.include_refs {
            self.cache.borrow_mut().insert(instance, hash);
        } else {
            self.cache_no_refs.borrow_mut().insert(instance, hash);
        }
        hash
    }
}

/// Deep subtree hash cache — includes children's hashes.
/// Used for Ref comparison (compare target content instead of ref_mapping)
/// and for subtree pruning (if deep hashes match, entire subtree is unchanged).
///
/// Computed lazily, bottom-up: requesting a parent's hash first computes all children.
/// Skips ignored properties (e.g. UniqueId, HistoryId) so pruning works correctly.
pub struct DeepHashCache<'a> {
    dom: &'a dyn DomView,
    ignore_properties: &'a HashSet<String>,
    cache: RefCell<HashStorage>,
    cache_no_refs: RefCell<HashStorage>,
}

impl<'a> DeepHashCache<'a> {
    pub fn new(dom: &'a dyn DomView, ignore_properties: &'a HashSet<String>) -> Self {
        Self {
            dom,
            ignore_properties,
            cache: RefCell::new(HashStorage::new(dom)),
            cache_no_refs: RefCell::new(HashStorage::new(dom)),
        }
    }

    /// Get deep hash for an instance, computing bottom-up if needed.
    pub fn get(&self, referent: Ref) -> Hash {
        let instance = self.dom.get_by_ref(referent).unwrap();
        self.get_instance_with_policy(instance, FULL_HASH)
    }

    pub(crate) fn get_instance_without_name(&self, instance: InstanceView<'_>) -> Hash {
        self.compute(
            instance,
            HashPolicy {
                include_name: false,
                include_refs: true,
            },
        )
    }

    pub(crate) fn get_instance_without_name_no_refs(&self, instance: InstanceView<'_>) -> Hash {
        self.compute(
            instance,
            HashPolicy {
                include_name: false,
                include_refs: false,
            },
        )
    }

    fn get_instance_with_policy(&self, instance: InstanceView<'_>, policy: HashPolicy) -> Hash {
        let cached = if policy.include_refs {
            self.cache.borrow().get(instance)
        } else {
            self.cache_no_refs.borrow().get(instance)
        };
        if let Some(hash) = cached {
            return hash;
        }

        let hash = self.compute(instance, policy);
        if policy.include_refs {
            self.cache.borrow_mut().insert(instance, hash);
        } else {
            self.cache_no_refs.borrow_mut().insert(instance, hash);
        }
        hash
    }

    fn compute(&self, inst: InstanceView<'_>, policy: HashPolicy) -> Hash {
        let mut hasher = hash_instance(self.dom, inst, Some(self.ignore_properties), policy);

        for child_ref in inst.children() {
            let child = self.dom.get_by_ref(child_ref).unwrap();
            let child_hash = self.get_instance_with_policy(
                child,
                HashPolicy {
                    include_name: true,
                    include_refs: policy.include_refs,
                },
            );
            hasher.update(child_hash.as_bytes());
        }

        hasher.finalize()
    }
}

/// Hash a variant value.
/// For Ref properties, uses name+class of the target as a stable identifier.
pub(crate) fn hash_variant(dom: &dyn DomView, hasher: &mut Hasher, value: &Variant) {
    match value {
        Variant::Attributes(attrs) => {
            let mut sorted: Vec<_> = attrs.iter().collect();
            sorted.sort_unstable_by_key(|(name, _)| name.as_str());
            for (name, attribute) in sorted {
                hasher.update(name.as_bytes());
                hash_variant(dom, hasher, attribute);
            }
        }
        Variant::Axes(a) => {
            hasher.update(&[a.bits()]);
        }
        Variant::BinaryString(bytes) => {
            let b: &[u8] = bytes.as_ref();
            hasher.update(b);
        }
        Variant::Bool(b) => {
            hasher.update(&[*b as u8]);
        }
        Variant::BrickColor(color) => {
            n_hash!(hasher, *color as u16);
        }
        Variant::CFrame(cf) => {
            vector_hash(hasher, cf.position);
            vector_hash(hasher, cf.orientation.x);
            vector_hash(hasher, cf.orientation.y);
            vector_hash(hasher, cf.orientation.z);
        }
        Variant::Color3(color) => {
            f32_hash(hasher, color.r);
            f32_hash(hasher, color.g);
            f32_hash(hasher, color.b);
        }
        Variant::Color3uint8(color) => {
            hasher.update(&[color.r, color.g, color.b]);
        }
        Variant::ColorSequence(seq) => {
            let mut keypoints: Vec<_> = seq.keypoints.iter().collect();
            keypoints.sort_unstable_by(|a, b| a.time.total_cmp(&b.time));
            for kp in keypoints {
                f32_hash(hasher, kp.time);
                f32_hash(hasher, kp.color.r);
                f32_hash(hasher, kp.color.g);
                f32_hash(hasher, kp.color.b);
            }
        }
        Variant::Content(content) => {
            use rbx_types::ContentType;
            match content.value() {
                ContentType::None => {
                    hasher.update(&[0x00]);
                }
                ContentType::Uri(uri) => {
                    hasher.update(&[0x01]);
                    hasher.update(normalize_asset_uri(uri).as_bytes());
                }
                // Object refs point at DOM instances; skip like Variant::Ref
                ContentType::Object(_) => {
                    hasher.update(&[0x02]);
                }
                _ => {
                    hasher.update(&[0x03]);
                }
            }
        }
        Variant::ContentId(id) => {
            hasher.update(normalize_asset_uri(id.as_str()).as_bytes());
        }
        Variant::Enum(e) => {
            n_hash!(hasher, e.to_u32());
        }
        Variant::Faces(f) => {
            hasher.update(&[f.bits()]);
        }
        Variant::Float32(n) => {
            f32_hash(hasher, *n);
        }
        Variant::Float64(n) => {
            f64_hash(hasher, *n);
        }
        Variant::Font(f) => {
            n_hash!(hasher, f.weight as u16, f.style as u8);
            hasher.update(f.family.as_bytes());
        }
        Variant::Int32(n) => {
            n_hash!(hasher, n);
        }
        Variant::Int64(n) => {
            n_hash!(hasher, n);
        }
        Variant::NumberRange(nr) => {
            f32_hash(hasher, nr.max);
            f32_hash(hasher, nr.min);
        }
        Variant::NumberSequence(seq) => {
            let mut keypoints: Vec<_> = seq.keypoints.iter().collect();
            keypoints.sort_unstable_by(|a, b| a.time.total_cmp(&b.time));
            for kp in keypoints {
                f32_hash(hasher, kp.time);
                f32_hash(hasher, kp.value);
                f32_hash(hasher, kp.envelope);
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
        Variant::PhysicalProperties(properties) => match properties {
            PhysicalProperties::Default => {
                hasher.update(&[0x00]);
            }
            PhysicalProperties::Custom(custom) => {
                hasher.update(&[0x01]);
                f32_hash(hasher, custom.density());
                f32_hash(hasher, custom.friction());
                f32_hash(hasher, custom.elasticity());
                f32_hash(hasher, custom.friction_weight());
                f32_hash(hasher, custom.elasticity_weight());
            }
        },
        Variant::Ray(ray) => {
            vector_hash(hasher, ray.origin);
            vector_hash(hasher, ray.direction);
        }
        Variant::Rect(rect) => {
            f32_hash(hasher, rect.max.x);
            f32_hash(hasher, rect.max.y);
            f32_hash(hasher, rect.min.x);
            f32_hash(hasher, rect.min.y);
        }
        Variant::Ref(target_ref) => {
            if target_ref.is_none() {
                hasher.update(&[0x00]); // null ref
            } else if let Some(target) = dom.get_by_ref(*target_ref) {
                // Identify the target by name+class AND its index path from the
                // root: name+class alone makes a retarget to a same-named
                // sibling invisible (deep hashes stay equal, pruning hides the
                // change). Index paths shift when siblings are inserted, but a
                // spuriously changed hash only costs pruning/matching work —
                // the property pass still decides equality via the ref mapping.
                hasher.update(&[0x01]);
                hasher.update(target.name().as_bytes());
                hasher.update(target.class().as_bytes());
                for index in ref_index_path(dom, *target_ref) {
                    n_hash!(hasher, index);
                }
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
                region.max.x,
                region.max.y,
                region.max.z,
                region.min.x,
                region.min.y,
                region.min.z
            );
        }
        Variant::SecurityCapabilities(caps) => {
            n_hash!(hasher, caps.bits());
        }
        Variant::SharedString(sstr) => {
            hasher.update(sstr.hash().as_bytes());
        }
        Variant::String(s) => {
            hasher.update(s.as_bytes());
        }
        Variant::Tags(tags) => {
            let mut sorted: Vec<&str> = tags.iter().collect();
            sorted.sort_unstable();
            for tag in sorted {
                hasher.update(tag.as_bytes());
            }
        }
        Variant::UDim(udim) => {
            f32_hash(hasher, udim.scale);
            n_hash!(hasher, udim.offset);
        }
        Variant::UDim2(udim) => {
            f32_hash(hasher, udim.x.scale);
            n_hash!(hasher, udim.x.offset);
            f32_hash(hasher, udim.y.scale);
            n_hash!(hasher, udim.y.offset);
        }
        Variant::Vector2(v2) => {
            f32_hash(hasher, v2.x);
            f32_hash(hasher, v2.y);
        }
        Variant::Vector2int16(v2) => {
            n_hash!(hasher, v2.x, v2.y);
        }
        Variant::Vector3(v3) => {
            vector_hash(hasher, *v3);
        }
        Variant::Vector3int16(v3) => {
            n_hash!(hasher, v3.x, v3.y, v3.z);
        }
        Variant::UniqueId(_) => {} // Skip - non-deterministic
        _ => {}                    // Skip unknown variants
    }
}
