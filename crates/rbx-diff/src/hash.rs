//! Instance hashing for content-based comparison.
//! Adapted from rbxmx-canon and rojo syncback.

use blake3::{Hash, Hasher};
use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::{PhysicalProperties, Variant, Vector3};
use std::cell::RefCell;
use std::collections::HashMap;

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

/// Lazy hash cache - computes hashes on demand.
/// Only computes hashes when requested, caching results for reuse.
pub struct LazyHashCache<'a> {
    dom: &'a WeakDom,
    cache: RefCell<HashMap<Ref, Hash>>,
}

impl<'a> LazyHashCache<'a> {
    /// Create a new lazy hash cache for the given DOM.
    pub fn new(dom: &'a WeakDom) -> Self {
        Self {
            dom,
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// Get hash for an instance, computing it if needed.
    /// Hashes are computed bottom-up (children first).
    pub fn get(&self, referent: Ref) -> Hash {
        // Check cache first
        if let Some(hash) = self.cache.borrow().get(&referent) {
            return *hash;
        }

        // Compute and cache
        let hash = self.compute_hash(referent);
        self.cache.borrow_mut().insert(referent, hash);
        hash
    }

    /// Check if `target` is a descendant of `ancestor`.
    /// Walks UP the parent chain - no recursion, no cycles possible.
    fn is_descendant(&self, target: Ref, ancestor: Ref) -> bool {
        let mut current = target;
        while let Some(inst) = self.dom.get_by_ref(current) {
            let parent_ref = inst.parent();
            if parent_ref == ancestor {
                return true;
            }
            if parent_ref.is_none() {
                break;
            }
            current = parent_ref;
        }
        false
    }

    fn compute_hash(&self, referent: Ref) -> Hash {
        let inst = self.dom.get_by_ref(referent).unwrap();
        let mut hasher = Hasher::new();

        hasher.update(inst.name.as_bytes());
        hasher.update(inst.class.as_bytes());

        // Sort properties by name for deterministic hashing
        let mut props: Vec<_> = inst.properties.iter().collect();
        props.sort_unstable_by_key(|(name, _)| name.as_str());

        for (name, value) in props {
            hasher.update(name.as_bytes());
            self.hash_variant(&mut hasher, value, referent);
        }

        // Recursively get child hashes (triggers lazy compute)
        let mut child_hashes: Vec<[u8; 32]> = inst
            .children()
            .iter()
            .map(|&child| *self.get(child).as_bytes())
            .collect();
        child_hashes.sort_unstable();

        for hash in child_hashes {
            hasher.update(&hash);
        }

        hasher.finalize()
    }

    /// Hash a variant value. For Ref properties, uses descendants-only hashing
    /// to avoid cycles while still capturing Ref target identity.
    fn hash_variant(&self, hasher: &mut Hasher, value: &Variant, current_ref: Ref) {
        match value {
            Variant::Attributes(attrs) => {
                let mut sorted: Vec<_> = attrs.iter().collect();
                sorted.sort_unstable_by_key(|(name, _)| name.as_str());
                for (name, attribute) in sorted {
                    hasher.update(name.as_bytes());
                    self.hash_variant(hasher, attribute, current_ref);
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
            Variant::Content(_) => { hasher.update(&[0x01]); }
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
                // Descendants-only Ref hashing for cycle safety
                if target_ref.is_none() {
                    hasher.update(&[0x00]); // null ref
                } else if self.is_descendant(*target_ref, current_ref) {
                    // Descendant: use full content hash (guaranteed in cache)
                    hasher.update(&[0x01]);
                    hasher.update(self.get(*target_ref).as_bytes());
                } else {
                    // Non-descendant: use name+class fallback
                    if let Some(target) = self.dom.get_by_ref(*target_ref) {
                        hasher.update(&[0x02]);
                        hasher.update(target.name.as_bytes());
                        hasher.update(target.class.as_bytes());
                    } else {
                        hasher.update(&[0x00]); // invalid ref = null
                    }
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
}
