//! Shared policy for authored Roblox properties and stable content identity.
//!
//! Matching, hashing, and diffing must agree about which serialized values are
//! authored content. Stable identity is deliberately narrower: it is evidence
//! that two duplicate siblings represent the same object even when spatial
//! properties changed.

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
const CONTENT_PROPERTY_EXCEPTIONS: &[(&str, &[&str])] = &[
    (
        "PartOperation",
        &[
            "MeshData",
            "MeshData2",
            "ChildData",
            "ChildData2",
            "AssetId",
        ],
    ),
    (
        "MeshPart",
        &[
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
    ),
    ("SpecialMesh", &["MeshId", "TextureId"]),
    ("Terrain", &["SmoothGrid", "Decoration"]),
    ("Workspace", &["CollisionGroupData"]),
];

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
        for (class, properties) in CONTENT_PROPERTY_EXCEPTIONS {
            if *class == current_class {
                result.extend(properties.iter().map(|property| (*property).to_string()));
            }
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

/// A strong, placement-independent identity clue for duplicate siblings.
/// Return `None` when a class has no such clue; callers must then use weaker
/// matching or preserve the instances as replacements.
pub(crate) fn stable_content_identity(instance: &Instance) -> Option<String> {
    let identity_property = match instance.class.as_str() {
        "MeshPart" => ["MeshContent", "MeshId", "MeshID"]
            .into_iter()
            .find_map(|name| instance.properties.get(&name.into())),
        "SpecialMesh" => ["MeshId", "MeshID"]
            .into_iter()
            .find_map(|name| instance.properties.get(&name.into())),
        _ => None,
    }?;
    let uri = content_uri(identity_property)?;
    if uri.is_empty() {
        return None;
    }
    Some(normalize_asset_uri(uri))
}

#[cfg(test)]
mod tests {
    use super::get_authored_properties;

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
}
