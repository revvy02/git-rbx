//! Shared semantic policy for Roblox properties and instance pairing.
//!
//! Matching, hashing, diffing, and merging must agree about which serialized
//! values are authored content. A content key is deliberately only *evidence*:
//! a uniquely anchored MeshPart may legitimately change MeshContent, while an
//! ambiguous or moved MeshPart must retain its key before we infer identity.

use rbx_dom_weak::Instance;
use rbx_reflection::{PropertyKind, PropertySerialization, Scriptability};
use rbx_types::{ContentType, Variant};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

/// Studio rewrites equivalent asset URL spellings on save. Canonicalize them
/// before using content IDs for equality, hashing, or identity.
pub(crate) fn normalize_asset_uri(uri: &str) -> String {
    let value = uri.trim();
    let lower = value.to_ascii_lowercase();

    let digits_at = |start: usize| -> String {
        value[start..]
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect()
    };

    if let Some(position) = lower.find("roblox.com/asset") {
        if let Some(id_position) = lower[position..].find("id=") {
            let digits = digits_at(position + id_position + 3);
            if !digits.is_empty() {
                return format!("rbxassetid://{digits}");
            }
        }
    }
    if lower.starts_with("rbxassetid://") {
        let digits = digits_at("rbxassetid://".len());
        if !digits.is_empty() {
            return format!("rbxassetid://{digits}");
        }
    }
    value.to_string()
}

/// Serialized-but-not-scriptable properties that carry authored content.
///
/// rbx-diff edits the serialized DOM directly, so Studio scriptability is not
/// a reason to discard these values. Derived/volatile properties remain
/// excluded; extend this list only for values that must survive a file merge.
struct ClassSemantics {
    class: &'static str,
    authored_exceptions: &'static [&'static str],
    content_key_properties: &'static [&'static str],
    bundles: &'static [SemanticPropertyBundle],
}

/// Properties that represent one indivisible authored value. If both branches
/// replace a mesh, choosing its asset from one branch and its source extent
/// from the other produces a visually corrupt hybrid, so they resolve once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SemanticPropertyBundle {
    pub(crate) name: &'static str,
    pub(crate) properties: &'static [&'static str],
}

const MESH_GEOMETRY: SemanticPropertyBundle = SemanticPropertyBundle {
    name: "Mesh geometry",
    properties: &["MeshContent", "MeshId", "MeshID", "InitialSize"],
};

const CLASS_SEMANTICS: &[ClassSemantics] = &[
    ClassSemantics {
        class: "PartOperation",
        authored_exceptions: &[
            "MeshData",
            "MeshData2",
            "ChildData",
            "ChildData2",
            "AssetId",
        ],
        content_key_properties: &[],
        bundles: &[],
    },
    ClassSemantics {
        class: "MeshPart",
        authored_exceptions: &[
            "MeshContent",
            "TextureContent",
            "MeshId",
            "TextureID",
            // Studio uses this persisted source-mesh extent to scale the
            // rendered mesh from `Size`. Keeping MeshContent but borrowing a
            // different part's InitialSize produces visually gigantic or tiny
            // geometry even though MeshId, Size, and CFrame all look correct.
            "InitialSize",
        ],
        content_key_properties: &["MeshContent", "MeshId", "MeshID"],
        bundles: &[MESH_GEOMETRY],
    },
    ClassSemantics {
        class: "SpecialMesh",
        authored_exceptions: &["MeshId", "TextureId"],
        content_key_properties: &["MeshId", "MeshID"],
        bundles: &[],
    },
    ClassSemantics {
        class: "Terrain",
        authored_exceptions: &["SmoothGrid", "Decoration"],
        content_key_properties: &[],
        bundles: &[],
    },
    ClassSemantics {
        class: "Workspace",
        authored_exceptions: &["CollisionGroupData"],
        content_key_properties: &[],
        bundles: &[],
    },
];

fn class_semantics(class_name: &str) -> Option<&'static ClassSemantics> {
    CLASS_SEMANTICS
        .iter()
        .find(|semantics| semantics.class == class_name)
}

/// Get the authored property names for a class. The result is cached for the
/// process lifetime and shared by matching, hashing, move detection, and diff.
pub(crate) fn get_authored_properties(class_name: &str) -> &'static HashSet<String> {
    static CLASS_PROPERTIES: OnceLock<Mutex<HashMap<String, &'static HashSet<String>>>> =
        OnceLock::new();

    let map_mutex = CLASS_PROPERTIES.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map_mutex.lock().unwrap();

    if !map.contains_key(class_name) {
        // One allocation per reflection class for the process lifetime. The
        // stable allocation lets callers release the mutex before reading the
        // set without a later HashMap growth invalidating their reference.
        let properties = Box::leak(Box::new(build_authored_properties(class_name)));
        map.insert(class_name.to_string(), properties);
    }

    let properties = *map.get(class_name).unwrap();
    drop(map);
    properties
}

fn build_authored_properties(class_name: &str) -> HashSet<String> {
    let database = rbx_reflection_database::get().unwrap();
    let mut result = HashSet::new();
    let mut current_class = class_name;

    while let Some(class_data) = database.classes.get(current_class) {
        if let Some(semantics) = class_semantics(current_class) {
            result.extend(
                semantics
                    .authored_exceptions
                    .iter()
                    .map(|property| (*property).to_string()),
            );
        }

        for (property_name, property_data) in &class_data.properties {
            if matches!(
                property_data.scriptability,
                Scriptability::None | Scriptability::Read
            ) {
                continue;
            }
            let does_not_serialize = match &property_data.kind {
                PropertyKind::Canonical { serialization } => {
                    matches!(serialization, PropertySerialization::DoesNotSerialize)
                }
                PropertyKind::Alias { .. } => continue,
                _ => false,
            };
            if !does_not_serialize {
                result.insert(property_name.to_string());
            }
        }

        let Some(superclass) = class_data.superclass.as_ref() else {
            break;
        };
        current_class = superclass;
    }

    result
}

fn content_uri(value: &Variant) -> Option<&str> {
    match value {
        Variant::ContentId(content) => Some(content.as_str()),
        Variant::Content(content) => match content.value() {
            ContentType::Uri(uri) => Some(uri),
            _ => None,
        },
        Variant::String(value) => Some(value),
        _ => None,
    }
}

/// A strong, placement-independent clue for distinguishing ambiguous peers.
/// It is not permanent identity: an anchored instance may change this value.
pub(crate) fn strong_content_key(instance: &Instance) -> Option<String> {
    let semantics = class_semantics(instance.class.as_str())?;
    let identity_property = semantics
        .content_key_properties
        .iter()
        .find_map(|name| instance.properties.get(&(*name).into()))?;
    let uri = content_uri(identity_property)?;
    if uri.is_empty() {
        return None;
    }
    Some(normalize_asset_uri(uri))
}

/// Why a caller believes two instances correspond. All matching paths pass
/// through this policy so a new heuristic cannot accidentally weaken the
/// content-key constraint used by other matchers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PairingBasis {
    /// Unique same-name/class siblings under an already matched parent.
    AnchoredName,
    /// Exact authored hashes already prove the content; class remains a guard.
    ExactContent,
    /// Similarity, position, or another heuristic is inferring correspondence.
    Inferred,
    /// A deep hash excluding only the root name proves a pure rename.
    ContentPreservingRename,
}

pub(crate) fn pairing_compatible(old: &Instance, new: &Instance, basis: PairingBasis) -> bool {
    if old.class != new.class {
        return false;
    }
    match basis {
        PairingBasis::AnchoredName => old.name == new.name,
        PairingBasis::ExactContent | PairingBasis::ContentPreservingRename => true,
        PairingBasis::Inferred => strong_content_key(old) == strong_content_key(new),
    }
}

pub(crate) fn semantic_property_bundle(
    class_name: &str,
    property_name: &str,
) -> Option<SemanticPropertyBundle> {
    class_semantics(class_name)?
        .bundles
        .iter()
        .copied()
        .find(|bundle| bundle.properties.contains(&property_name))
}

/// Compare the complete logical value of a bundle, not just the low-level ops
/// a branch happened to emit. Asset aliases collapse through the normalized
/// content key; the remaining support properties use normal semantic equality.
pub(crate) fn semantic_bundle_values_equal(
    old: &Instance,
    new: &Instance,
    bundle: SemanticPropertyBundle,
) -> bool {
    if old.class != new.class {
        return false;
    }
    let content_key_properties = class_semantics(old.class.as_str())
        .map(|semantics| semantics.content_key_properties)
        .unwrap_or_default();
    if bundle
        .properties
        .iter()
        .any(|property| content_key_properties.contains(property))
        && strong_content_key(old) != strong_content_key(new)
    {
        return false;
    }

    bundle
        .properties
        .iter()
        .filter(|property| !content_key_properties.contains(property))
        .all(|property| {
            let old_value = old.properties.get(&(*property).into());
            let new_value = new.properties.get(&(*property).into());
            match (old_value, new_value) {
                (Some(old_value), Some(new_value)) => {
                    crate::value_compare::non_ref_variants_equal(old_value, new_value)
                }
                (None, None) => true,
                _ => false,
            }
        })
}

#[cfg(test)]
mod tests {
    use super::{
        get_authored_properties, pairing_compatible, semantic_property_bundle, PairingBasis,
    };
    use rbx_dom_weak::{InstanceBuilder, WeakDom};
    use rbx_types::{Content, Variant};

    #[test]
    fn authored_property_references_survive_cache_growth_and_concurrency() {
        let part_properties = get_authored_properties("Part");
        assert!(part_properties.contains("CFrame"));

        let classes: Vec<String> = rbx_reflection_database::get()
            .unwrap()
            .classes
            .keys()
            .map(|name| name.to_string())
            .collect();

        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    for class in &classes {
                        let _ = get_authored_properties(class);
                    }
                });
            }
        });

        assert!(part_properties.contains("CFrame"));
    }

    #[test]
    fn mesh_geometry_is_authored_content() {
        let properties = get_authored_properties("MeshPart");
        assert!(properties.contains("MeshContent"));
        assert!(properties.contains("TextureContent"));
        assert!(properties.contains("InitialSize"));
    }

    #[test]
    fn content_keys_constrain_inference_but_not_anchored_edits() {
        let old = WeakDom::new(
            InstanceBuilder::new("MeshPart")
                .with_name("Part")
                .with_property(
                    "MeshContent",
                    Variant::Content(Content::from_uri("rbxassetid://1")),
                ),
        );
        let new = WeakDom::new(
            InstanceBuilder::new("MeshPart")
                .with_name("Part")
                .with_property(
                    "MeshContent",
                    Variant::Content(Content::from_uri("rbxassetid://2")),
                ),
        );

        assert!(pairing_compatible(
            old.root(),
            new.root(),
            PairingBasis::AnchoredName
        ));
        assert!(!pairing_compatible(
            old.root(),
            new.root(),
            PairingBasis::Inferred
        ));
    }

    #[test]
    fn mesh_content_and_initial_size_share_one_bundle() {
        let content = semantic_property_bundle("MeshPart", "MeshContent").unwrap();
        let initial_size = semantic_property_bundle("MeshPart", "InitialSize").unwrap();
        assert_eq!(content, initial_size);
        assert_eq!(content.name, "Mesh geometry");
        assert!(semantic_property_bundle("MeshPart", "Size").is_none());
    }
}
