//! Matcher-independent verification of serialized geometry invariants.
//!
//! A normal diff must infer instance correspondence, so a shared matcher bug
//! can make both an edit script and its round-trip check agree on the same
//! wrong pairing. This verifier deliberately uses only structural paths,
//! normalized content keys, and raw persisted values. It is a second line of
//! defense for tests and callers that need to prove a produced model retains
//! the expected mesh-to-transform relationship.

use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::{CFrame, Variant, Vector3};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::property_semantics::strong_content_key;

const COMPONENT_NAMES: [&str; 18] = [
    "CFrame.position.x",
    "CFrame.position.y",
    "CFrame.position.z",
    "CFrame.orientation.xx",
    "CFrame.orientation.xy",
    "CFrame.orientation.xz",
    "CFrame.orientation.yx",
    "CFrame.orientation.yy",
    "CFrame.orientation.yz",
    "CFrame.orientation.zx",
    "CFrame.orientation.zy",
    "CFrame.orientation.zz",
    "Size.x",
    "Size.y",
    "Size.z",
    "InitialSize.x",
    "InitialSize.y",
    "InitialSize.z",
];

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticMismatch {
    /// Class/name ancestry plus normalized mesh content key.
    pub key: String,
    /// Human-readable count, missing-property, or component mismatch.
    pub detail: String,
}

#[derive(Debug, Clone, Copy)]
struct MeshState {
    components: [Option<f32>; 18],
}

fn structural_path(dom: &WeakDom, referent: Ref) -> String {
    let mut segments = Vec::new();
    let mut current = referent;
    while current != dom.root_ref() {
        let Some(instance) = dom.get_by_ref(current) else {
            break;
        };
        segments.push(format!("{}:{}", instance.class, instance.name));
        let parent = instance.parent();
        if parent.is_none() {
            break;
        }
        current = parent;
    }
    segments.reverse();
    segments.join("/")
}

fn cframe(instance: &rbx_dom_weak::Instance) -> Option<CFrame> {
    match instance.properties.get(&"CFrame".into()) {
        Some(Variant::CFrame(value)) => Some(*value),
        _ => None,
    }
}

fn vector3(instance: &rbx_dom_weak::Instance, property: &str) -> Option<Vector3> {
    match instance.properties.get(&property.into()) {
        Some(Variant::Vector3(value)) => Some(*value),
        _ => None,
    }
}

fn mesh_state(instance: &rbx_dom_weak::Instance) -> MeshState {
    let mut components = [None; 18];
    if let Some(frame) = cframe(instance) {
        let values = [
            frame.position.x,
            frame.position.y,
            frame.position.z,
            frame.orientation.x.x,
            frame.orientation.x.y,
            frame.orientation.x.z,
            frame.orientation.y.x,
            frame.orientation.y.y,
            frame.orientation.y.z,
            frame.orientation.z.x,
            frame.orientation.z.y,
            frame.orientation.z.z,
        ];
        for (index, value) in values.into_iter().enumerate() {
            components[index] = Some(value);
        }
    }
    for (offset, value) in [
        (12, vector3(instance, "Size")),
        (15, vector3(instance, "InitialSize")),
    ] {
        if let Some(value) = value {
            components[offset] = Some(value.x);
            components[offset + 1] = Some(value.y);
            components[offset + 2] = Some(value.z);
        }
    }
    MeshState { components }
}

fn states(dom: &WeakDom) -> BTreeMap<String, Vec<MeshState>> {
    let mut result: BTreeMap<String, Vec<MeshState>> = BTreeMap::new();
    for instance in dom
        .descendants()
        .filter(|instance| instance.class.as_str() == "MeshPart")
    {
        let key = format!(
            "{}|{}",
            structural_path(dom, instance.referent()),
            strong_content_key(instance).unwrap_or_else(|| "<missing-content-key>".to_string())
        );
        result.entry(key).or_default().push(mesh_state(instance));
    }
    for group in result.values_mut() {
        group.sort_by(compare_states);
    }
    result
}

fn compare_states(left: &MeshState, right: &MeshState) -> Ordering {
    left.components
        .iter()
        .zip(right.components)
        .map(|(left, right)| match (left, right) {
            (Some(left), Some(right)) => left.total_cmp(&right),
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
        })
        .find(|ordering| !ordering.is_eq())
        .unwrap_or(Ordering::Equal)
}

fn tolerance(component: usize) -> f32 {
    if component < 3 {
        // Normalizing an asset frame can accumulate a few millistuds.
        0.01
    } else {
        1.0e-4
    }
}

/// Compare every MeshPart's raw CFrame, Size, and persisted InitialSize after
/// grouping by structural path and normalized MeshContent. Sibling order and
/// referents are intentionally irrelevant; no rbx-diff matcher is consulted.
pub fn verify_mesh_geometry(actual: &WeakDom, expected: &WeakDom) -> Vec<SemanticMismatch> {
    let actual = states(actual);
    let expected = states(expected);
    let keys: BTreeSet<_> = actual.keys().chain(expected.keys()).cloned().collect();
    let mut mismatches = Vec::new();

    for key in keys {
        let actual_states = actual.get(&key).map(Vec::as_slice).unwrap_or_default();
        let expected_states = expected.get(&key).map(Vec::as_slice).unwrap_or_default();
        if actual_states.len() != expected_states.len() {
            mismatches.push(SemanticMismatch {
                key,
                detail: format!(
                    "instance count differs: actual {}, expected {}",
                    actual_states.len(),
                    expected_states.len()
                ),
            });
            continue;
        }

        'instances: for (instance_index, (actual, expected)) in
            actual_states.iter().zip(expected_states).enumerate()
        {
            for (component, component_name) in COMPONENT_NAMES.iter().enumerate() {
                let equal = match (actual.components[component], expected.components[component]) {
                    (Some(actual), Some(expected)) => {
                        (actual - expected).abs() <= tolerance(component)
                    }
                    (None, None) => true,
                    _ => false,
                };
                if !equal {
                    mismatches.push(SemanticMismatch {
                        key: key.clone(),
                        detail: format!(
                            "instance {instance_index} {} differs: actual {:?}, expected {:?}",
                            component_name,
                            actual.components[component],
                            expected.components[component]
                        ),
                    });
                    continue 'instances;
                }
            }
        }
    }

    mismatches
}

#[cfg(test)]
mod tests {
    use super::verify_mesh_geometry;
    use rbx_dom_weak::{InstanceBuilder, WeakDom};
    use rbx_types::{CFrame, Content, Matrix3, Variant, Vector3};

    fn mesh(initial_size: f32) -> WeakDom {
        WeakDom::new(
            InstanceBuilder::new("Folder").with_name("root").with_child(
                InstanceBuilder::new("MeshPart")
                    .with_name("Paint")
                    .with_property(
                        "MeshContent",
                        Variant::Content(Content::from_uri("rbxassetid://42")),
                    )
                    .with_property(
                        "CFrame",
                        Variant::CFrame(CFrame::new(
                            Vector3::new(1.0, 2.0, 3.0),
                            Matrix3::identity(),
                        )),
                    )
                    .with_property("Size", Variant::Vector3(Vector3::new(4.0, 5.0, 6.0)))
                    .with_property(
                        "InitialSize",
                        Variant::Vector3(Vector3::new(initial_size, initial_size, initial_size)),
                    ),
            ),
        )
    }

    #[test]
    fn raw_verifier_catches_support_property_mismatch() {
        assert!(verify_mesh_geometry(&mesh(1.0), &mesh(1.0)).is_empty());
        let mismatches = verify_mesh_geometry(&mesh(8.0), &mesh(1.0));
        assert_eq!(mismatches.len(), 1, "{mismatches:#?}");
        assert!(mismatches[0].detail.contains("InitialSize"));
    }
}
