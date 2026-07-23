//! Exhaustive reachability check for the corrected tow-truck merge oracle.
//!
//! The merge runs once. Every binary conflict combination is then finalized
//! from the same in-memory conflicted bytes. A cheap, sibling-order-independent
//! tree-shape fingerprint rejects impossible candidates before the expensive
//! full DOM diff.

use rbx_diff::{
    diff_doms, finalize, find_container, list_entries, mark_entry, normalize_model_dom_to_base,
    verify_mesh_geometry,
};
use rbx_dom_weak::{types::Ref, WeakDom};
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
            .expect("candidate should have a dominant pivot");
        if diff_doms(&expected, &candidate).is_empty()
            && verify_mesh_geometry(&candidate, &expected).is_empty()
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
