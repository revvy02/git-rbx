//! Exhaustive reachability check for the corrected tow-truck merge oracle.
//!
//! The merge runs once. Every binary conflict combination is then finalized
//! from the same in-memory conflicted bytes. A cheap, sibling-order-independent
//! tree-shape fingerprint rejects impossible candidates before the expensive
//! full DOM diff.

use rbx_diff::{
    diff_doms, finalize, find_container, list_entries, mark_entry, normalize_model_dom_to_base,
};
use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::{CFrame, ContentType, Variant};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process::Command;

const FIXTURE: &str = "tests-new/tow-truck/origin-merge-into-tow-truck";

fn load(path: &Path) -> WeakDom {
    rbx_binary::from_reader(BufReader::new(File::open(path).unwrap())).unwrap()
}

fn instance_path(dom: &WeakDom, referent: Ref) -> String {
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

/// Multiset rather than ordered traversal: Roblox sibling order is not an
/// authored tree move, while class/name ancestry is structural content.
fn shape(dom: &WeakDom) -> BTreeMap<String, usize> {
    let mut result = BTreeMap::new();
    for instance in dom.descendants() {
        *result
            .entry(instance_path(dom, instance.referent()))
            .or_default() += 1;
    }
    result
}

fn shape_distance(a: &BTreeMap<String, usize>, b: &BTreeMap<String, usize>) -> usize {
    let mut distance = 0;
    for (path, a_count) in a {
        distance += a_count.abs_diff(b.get(path).copied().unwrap_or(0));
    }
    for (path, b_count) in b {
        if !a.contains_key(path) {
            distance += b_count;
        }
    }
    distance
}

fn child_named(dom: &WeakDom, parent: Ref, name: &str) -> Option<Ref> {
    dom.get_by_ref(parent)?
        .children()
        .iter()
        .copied()
        .find(|child| {
            dom.get_by_ref(*child)
                .is_some_and(|instance| instance.name == name)
        })
}

fn path_ref(dom: &WeakDom, names: &[&str]) -> Option<Ref> {
    names.iter().try_fold(dom.root_ref(), |parent, name| {
        child_named(dom, parent, name)
    })
}

fn cframe(dom: &WeakDom, referent: Ref) -> Option<CFrame> {
    match dom.get_by_ref(referent)?.properties.get(&"CFrame".into()) {
        Some(Variant::CFrame(value)) => Some(*value),
        _ => None,
    }
}

fn mesh_id(dom: &WeakDom, referent: Ref) -> Option<String> {
    let instance = dom.get_by_ref(referent)?;
    ["MeshContent", "MeshId", "MeshID"]
        .into_iter()
        .find_map(|name| instance.properties.get(&name.into()))
        .and_then(|value| match value {
            Variant::Content(content) => match content.value() {
                ContentType::Uri(uri) => Some(uri.clone()),
                _ => None,
            },
            Variant::ContentId(content) => Some(content.as_str().to_string()),
            _ => None,
        })
        .filter(|value| !value.is_empty())
}

/// Express a part CFrame in the reference part's coordinate system. This is
/// intentionally independent of rbx-diff's matcher and frame normalization.
fn relative_components(reference: CFrame, part: CFrame) -> [f64; 12] {
    let rows = |value: CFrame| {
        [
            [
                value.orientation.x.x as f64,
                value.orientation.x.y as f64,
                value.orientation.x.z as f64,
            ],
            [
                value.orientation.y.x as f64,
                value.orientation.y.y as f64,
                value.orientation.y.z as f64,
            ],
            [
                value.orientation.z.x as f64,
                value.orientation.z.y as f64,
                value.orientation.z.z as f64,
            ],
        ]
    };
    let reference_rotation = rows(reference);
    let part_rotation = rows(part);
    let inverse = [
        [
            reference_rotation[0][0],
            reference_rotation[1][0],
            reference_rotation[2][0],
        ],
        [
            reference_rotation[0][1],
            reference_rotation[1][1],
            reference_rotation[2][1],
        ],
        [
            reference_rotation[0][2],
            reference_rotation[1][2],
            reference_rotation[2][2],
        ],
    ];
    let position_delta = [
        (part.position.x - reference.position.x) as f64,
        (part.position.y - reference.position.y) as f64,
        (part.position.z - reference.position.z) as f64,
    ];
    let multiply_vector = |matrix: &[[f64; 3]; 3], vector: [f64; 3]| {
        [0, 1, 2].map(|row| {
            matrix[row][0] * vector[0] + matrix[row][1] * vector[1] + matrix[row][2] * vector[2]
        })
    };
    let position = multiply_vector(&inverse, position_delta);
    let mut rotation = [[0.0; 3]; 3];
    for (row, cells) in rotation.iter_mut().enumerate() {
        for (column, cell) in cells.iter_mut().enumerate() {
            *cell = (0..3)
                .map(|inner| inverse[row][inner] * part_rotation[inner][column])
                .sum();
        }
    }
    [
        position[0],
        position[1],
        position[2],
        rotation[0][0],
        rotation[0][1],
        rotation[0][2],
        rotation[1][0],
        rotation[1][1],
        rotation[1][2],
        rotation[2][0],
        rotation[2][1],
        rotation[2][2],
    ]
}

fn cab_mesh_layout(dom: &WeakDom, cab_name: &str) -> Option<BTreeMap<String, Vec<[f64; 12]>>> {
    let cab = path_ref(
        dom,
        &["1234567", "2026_Sierra_HT", "assembly", "cab", cab_name],
    )?;
    let drive_seat = child_named(dom, cab, "DriveSeat")?;
    let reference = cframe(dom, drive_seat)?;
    let mut result: BTreeMap<String, Vec<[f64; 12]>> = BTreeMap::new();
    let mut pending = vec![cab];
    while let Some(referent) = pending.pop() {
        let instance = dom.get_by_ref(referent)?;
        pending.extend(instance.children().iter().copied());
        if instance.class.as_str() != "MeshPart" {
            continue;
        }
        let (Some(id), Some(frame)) = (mesh_id(dom, referent), cframe(dom, referent)) else {
            continue;
        };
        result
            .entry(id)
            .or_default()
            .push(relative_components(reference, frame));
    }
    for transforms in result.values_mut() {
        transforms.sort_by(|left, right| {
            left.iter()
                .zip(right)
                .map(|(left, right)| left.total_cmp(right))
                .find(|ordering| !ordering.is_eq())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    Some(result)
}

fn cab_mesh_layouts_match(actual: &WeakDom, expected: &WeakDom) -> bool {
    for cab_name in ["crew", "extended", "standard"] {
        let (Some(actual), Some(expected)) = (
            cab_mesh_layout(actual, cab_name),
            cab_mesh_layout(expected, cab_name),
        ) else {
            return false;
        };
        if actual.keys().ne(expected.keys()) {
            return false;
        }
        for (mesh_id, actual_transforms) in actual {
            let expected_transforms = &expected[&mesh_id];
            if actual_transforms.len() != expected_transforms.len() {
                return false;
            }
            for (actual, expected) in actual_transforms.iter().zip(expected_transforms) {
                for component in 0..12 {
                    let tolerance = if component < 3 { 0.01 } else { 1e-4 };
                    if (actual[component] - expected[component]).abs() > tolerance {
                        return false;
                    }
                }
            }
        }
    }
    true
}

fn decision_string(names: &[String], mask: usize) -> String {
    names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            format!(
                "{name}={}",
                if mask & (1 << index) == 0 {
                    "ours"
                } else {
                    "theirs"
                }
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn resolve_mask(conflicted_bytes: &[u8], mask: usize) -> WeakDom {
    let mut candidate = rbx_binary::from_reader(conflicted_bytes).unwrap();
    let container = find_container(&candidate).unwrap();
    let choices: Vec<_> = list_entries(&candidate, container)
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            (
                entry.entry_ref,
                if mask & (1 << index) == 0 {
                    "ours"
                } else {
                    "theirs"
                },
            )
        })
        .collect();
    for (entry_ref, side) in choices {
        mark_entry(&mut candidate, entry_ref, side).unwrap();
    }
    finalize(&mut candidate).unwrap();
    candidate
}

#[test]
fn human_merge_is_reachable_from_binary_conflict_decisions() {
    let fixture = Path::new(FIXTURE);
    if !fixture.join("merged-expected.rbxm").exists() {
        eprintln!("SKIP: origin tow-truck fixture not present");
        return;
    }

    let scratch =
        std::env::temp_dir().join(format!("rbx-diff-tow-reachability-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    let conflicted_path = scratch.join("conflicted.rbxm");
    let output = Command::new(env!("CARGO_BIN_EXE_rbx-diff"))
        .args([
            "merge",
            fixture.join("base.rbxm").to_str().unwrap(),
            fixture.join("ours-tow-truck-rig.rbxm").to_str().unwrap(),
            fixture
                .join("theirs-origin-assembly.rbxm")
                .to_str()
                .unwrap(),
            "--output",
            conflicted_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(1),
        "merge should produce conflicts\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let conflicted_bytes = std::fs::read(&conflicted_path).unwrap();
    let conflicted = rbx_binary::from_reader(conflicted_bytes.as_slice()).unwrap();
    let entries = list_entries(&conflicted, find_container(&conflicted).unwrap());
    let entry_names: Vec<String> = entries.iter().map(|entry| entry.name.clone()).collect();
    assert!(
        entry_names.len() < usize::BITS as usize,
        "too many binary conflicts for a bitmask search"
    );

    let expected = load(&fixture.join("merged-expected.rbxm"));
    let expected_shape = shape(&expected);
    let mut exact_matches = Vec::new();
    let mut closest: Option<(usize, usize)> = None;

    for mask in 0..(1usize << entry_names.len()) {
        let mut candidate = resolve_mask(&conflicted_bytes, mask);
        let distance = shape_distance(&shape(&candidate), &expected_shape);
        if closest.is_none_or(|(_, previous)| distance < previous) {
            closest = Some((mask, distance));
        }
        if distance != 0 {
            continue;
        }

        // Absolute `.rbxm` save placement is not semantic. Align the candidate
        // to the expected asset frame; residual part and pivot edits remain.
        normalize_model_dom_to_base(&expected, &mut candidate)
            .expect("candidate should have a dominant model frame");
        if diff_doms(&expected, &candidate).is_empty()
            && cab_mesh_layouts_match(&candidate, &expected)
        {
            exact_matches.push(mask);
        }
    }

    std::fs::remove_dir_all(&scratch).unwrap();
    let closest = closest.unwrap();
    assert_eq!(
        exact_matches.len(),
        1,
        "expected one reachable human merge; matches={exact_matches:?}; \
         closest has {} structural mismatch(es): {}",
        closest.1,
        decision_string(&entry_names, closest.0),
    );
    eprintln!(
        "reachable as: {}",
        decision_string(&entry_names, exact_matches[0])
    );
}
