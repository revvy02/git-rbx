//! Regression tests over the real-world fixture files (the private
//! `fixtures/` submodule; see tests/common for how they are located).
//!
//! Small fixtures run on every `cargo test`. The multi-megabyte place files
//! are `#[ignore]`d because debug-mode decoding is slow — run them with
//! `cargo test --release -- --ignored`.

mod common;
use common::fixture_str;

use git_rbx::{
    diff_doms, diff_model_compact_doms_with_config, DiffConfig, DiffDom, DiffEntry,
};
use rbx_dom_weak::WeakDom;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

fn load(path: &str) -> Option<WeakDom> {
    if !Path::new(path).exists() {
        eprintln!("SKIP: fixture {path} not present");
        return None;
    }
    let file = BufReader::new(File::open(path).unwrap());
    Some(if path.ends_with(".rbxm") || path.ends_with(".rbxl") {
        rbx_binary::from_reader(file).unwrap()
    } else {
        rbx_xml::from_reader_default(file).unwrap()
    })
}

fn load_compact(path: &str) -> Option<DiffDom> {
    if !Path::new(path).exists() {
        eprintln!("SKIP: fixture {path} not present");
        return None;
    }
    let file = BufReader::new(File::open(path).unwrap());
    Some(if path.ends_with(".rbxm") || path.ends_with(".rbxl") {
        DiffDom::from_binary_reader(file).unwrap()
    } else {
        DiffDom::from_weak_dom_owned(rbx_xml::from_reader_default(file).unwrap())
    })
}

fn diff_files(old: &str, new: &str) -> Option<Vec<DiffEntry>> {
    Some(diff_doms(&load(old)?, &load(new)?))
}

fn counts(diffs: &[DiffEntry]) -> (usize, usize, usize, usize) {
    let mut c = (0, 0, 0, 0);
    for d in diffs {
        match d {
            DiffEntry::Added { .. } => c.0 += 1,
            DiffEntry::Removed { .. } => c.1 += 1,
            DiffEntry::Modified { .. } => c.2 += 1,
            DiffEntry::Moved { .. } => c.3 += 1,
            DiffEntry::Pivoted { .. } => {}
        }
    }
    c
}

// ============================================================================
// Small fixtures — always run
// ============================================================================

#[test]
fn union_operation_replaces_parts() {
    let Some(diffs) = diff_files(
        &fixture_str("union-operation/separated-parts.rbxm"),
        &fixture_str("union-operation/unioned-parts.rbxm"),
    ) else {
        return;
    };
    let (added, removed, modified, moved) = counts(&diffs);
    assert_eq!(
        (added, removed, modified, moved),
        (1, 2, 0, 0),
        "two Parts should be replaced by one Union: {diffs:?}"
    );
}

#[test]
fn union_geometry_change_is_detected() {
    // Same size and position, only the CSG geometry differs — exercises the
    // CONTENT_PROPERTY_EXCEPTIONS allowlist (MeshData/ChildData).
    let Some(diffs) = diff_files(
        &fixture_str("union-operation/unioned-parts.rbxm"),
        &fixture_str("union-operation/unioned-parts-in-same-spot-but-diff-geometry.rbxm"),
    ) else {
        return;
    };
    let (added, removed, modified, moved) = counts(&diffs);
    assert_eq!((added, removed, moved), (0, 0, 0), "{diffs:?}");
    assert_eq!(modified, 1, "geometry-only change must be visible: {diffs:?}");
    match &diffs[0] {
        DiffEntry::Modified { class, property_changes, .. } => {
            assert_eq!(class, "UnionOperation");
            assert!(
                !property_changes.is_empty(),
                "expected CSG blob property changes"
            );
        }
        other => panic!("expected Modified, got {other:?}"),
    }
}

#[test]
fn primary_part_retarget_among_same_named_siblings() {
    // A Model's PrimaryPart moves to a different, identically-named sibling.
    // Regression for the Ref hashing bug where name+class-only target identity
    // made this invisible to subtree pruning.
    let Some(diffs) = diff_files(
        &fixture_str("referential-properties/primary-part/model-with-grey-primary-part-and-has-dupe-children-names.rbxm"),
        &fixture_str("referential-properties/primary-part/model-with-yellow-primary-part-and-has-dupe-children-names.rbxm"),
    ) else {
        return;
    };
    let (added, removed, modified, moved) = counts(&diffs);
    assert_eq!((added, removed, modified, moved), (0, 0, 1, 0), "{diffs:?}");
    match &diffs[0] {
        DiffEntry::Modified { class, property_changes, .. } => {
            assert_eq!(class, "Model");
            assert_eq!(property_changes.len(), 1, "{property_changes:?}");
            assert_eq!(property_changes[0].name, "PrimaryPart");
        }
        other => panic!("expected Modified, got {other:?}"),
    }
}

// ============================================================================
// Large place/model fixtures — run with `cargo test --release -- --ignored`
// ============================================================================

#[test]
#[ignore = "23MB fixtures; run with cargo test --release -- --ignored"]
fn case_1_baseline() {
    let Some(diffs) = diff_files(
        &fixture_str("rcdev-maps/case_1/map_2.rbxm"),
        &fixture_str("rcdev-maps/case_1/rcdev_map_current.rbxm"),
    ) else {
        return;
    };
    let (added, removed, modified, moved) = counts(&diffs);
    assert_eq!((added, removed, modified, moved), (0, 0, 5, 0), "{diffs:?}");

    // The rename described in the fixture README must be present
    let has_rename = diffs.iter().any(|d| matches!(
        d,
        DiffEntry::Modified { property_changes, .. }
            if property_changes.iter().any(|c| c.name == "Name")
    ));
    assert!(has_rename, "expected the Sign→Signs rename: {diffs:?}");
}

#[test]
#[ignore = "46MB fixtures; run with cargo test --release -- --ignored"]
fn save_vs_save_tree_move_is_cframe_changes_only() {
    let Some(diffs) = diff_files(
        &fixture_str("rc-builds/rc_manually_saved_build.rbxl"),
        &fixture_str("models-moved/rc_build_saved_manually_with_1_tree_moved.rbxl"),
    ) else {
        return;
    };
    let (added, removed, _modified, moved) = counts(&diffs);
    // A spatial (CFrame) move must not appear as add/remove/reparent
    assert_eq!((added, removed, moved), (0, 0, 0), "{diffs:?}");
    let tree4_changed = diffs.iter().any(|d| matches!(
        d,
        DiffEntry::Modified { path, .. } if path.contains("Tree4")
    ));
    assert!(tree4_changed, "expected Tree4 CFrame modifications: {diffs:?}");
}

#[test]
#[ignore = "46MB fixtures; run with cargo test --release -- --ignored"]
fn fresh_build_to_manual_save_is_only_known_studio_materialization() {
    let Some(diffs) = diff_files(
        &fixture_str("rc-builds/rc_fresh_build.rbxl"),
        &fixture_str("rc-builds/rc_manually_saved_build.rbxl"),
    ) else {
        return;
    };
    assert_eq!(counts(&diffs), (28, 0, 2, 0), "{diffs:#?}");

    let modified_paths: Vec<_> = diffs
        .iter()
        .filter_map(|diff| match diff {
            DiffEntry::Modified { path, .. } => Some(path.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(modified_paths, vec!["Lighting", "Workspace"], "{diffs:#?}");
    assert!(
        diffs.iter().all(|diff| !matches!(
            diff,
            DiffEntry::Modified { path, .. } if path.contains("InteriorDoors")
        )),
        "a save must not manufacture InteriorDoors edits: {diffs:#?}"
    );
}

#[test]
#[ignore = "46MB fixtures; run with cargo test --release -- --ignored"]
fn two_tree_moves_collapse_to_two_pivots_and_camera_state() {
    let Some(base) = load_compact(&fixture_str("rc-builds/rc_manually_saved_build.rbxl")) else {
        return;
    };
    let Some(mut side) =
        load_compact(&fixture_str("models-moved/rc_build_saved_manually_with_2_trees_moved.rbxl"))
    else {
        return;
    };
    let (diffs, pivots) =
        diff_model_compact_doms_with_config(&base, &mut side, &DiffConfig::default());
    assert_eq!(pivots.unwrap().pivots.len(), 2, "{diffs:#?}");
    assert_eq!(counts(&diffs), (0, 0, 1, 0), "{diffs:#?}");

    let pivot_paths: std::collections::BTreeSet<_> = diffs
        .iter()
        .filter_map(|diff| match diff {
            DiffEntry::Pivoted { path, .. } => Some(path.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(pivot_paths.len(), 2, "{diffs:#?}");
    assert!(pivot_paths.iter().any(|path| path.ends_with("Tree3")));
    assert!(pivot_paths.iter().any(|path| path.ends_with("Tree4")));

    let camera = diffs.iter().find_map(|diff| match diff {
        DiffEntry::Modified {
            path,
            property_changes,
            ..
        } if path == "Workspace.Camera" => Some(property_changes),
        _ => None,
    });
    let camera = camera.expect("the manual save also records the viewing camera");
    let properties: std::collections::BTreeSet<_> =
        camera.iter().map(|change| change.name.as_str()).collect();
    assert_eq!(
        properties,
        std::collections::BTreeSet::from(["CFrame", "Focus"])
    );
}

#[test]
#[ignore = "46MB fixtures; run with cargo test --release -- --ignored"]
fn police_station_and_nested_moves_collapse_to_three_pivots() {
    let Some(base) = load_compact(&fixture_str("rc-builds/rc_manually_saved_build.rbxl")) else {
        return;
    };
    let Some(mut side) = load_compact(
        &fixture_str("models-moved/rc_police_station_model_moved_with_internal_models_moved_too.rbxl"),
    ) else {
        return;
    };

    let (diffs, frames) =
        diff_model_compact_doms_with_config(&base, &mut side, &DiffConfig::default());
    let frames = frames.expect("the fixture contains three hierarchical model moves");
    assert_eq!(frames.pivots.len(), 3, "{:#?}", frames.pivots);
    assert_eq!(counts(&diffs), (0, 1, 5, 0), "{diffs:#?}");

    let residual_cframes: Vec<_> = diffs
        .iter()
        .filter_map(|diff| match diff {
            DiffEntry::Modified {
                path,
                property_changes,
                ..
            } if path.contains("PoliceStation")
                && property_changes
                    .iter()
                    .any(|change| change.name == "CFrame") =>
            {
                Some(path)
            }
            _ => None,
        })
        .collect();
    assert!(
        residual_cframes.is_empty(),
        "model moves must not leak descendant CFrames: {residual_cframes:#?}"
    );
}

#[test]
#[ignore = "46MB fixtures; run with cargo test --release -- --ignored"]
fn save_vs_save_menu_gui_removal() {
    let Some(diffs) = diff_files(
        &fixture_str("rc-builds/rc_manually_saved_build.rbxl"),
        &fixture_str("rc-builds/rc_menu_gui_removed.rbxl"),
    ) else {
        return;
    };
    let removed: Vec<_> = diffs
        .iter()
        .filter_map(|d| match d {
            DiffEntry::Removed { path, .. } => Some(path.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(removed, vec!["StarterGui.Menu"], "{diffs:?}");
}

#[test]
fn obj_value_identical_twins_diff_shape() {
    // Two identical "Uniform Giver" models: setting each PrimaryPart to its
    // own ClickPart must diff as exactly two modifications — twin identity
    // must not cross the refs up.
    let d = &fixture_str("referential-properties/obj-value");
    let Some(diffs) = diff_files(
        &format!("{d}/police-station.rbxm"),
        &format!("{d}/police-station-with-2-identical-uni-givers-with-primary-part.rbxm"),
    ) else {
        return;
    };
    let (added, removed, modified, moved) = counts(&diffs);
    assert_eq!((added, removed, modified, moved), (0, 0, 2, 0), "{diffs:#?}");
    for diff in &diffs {
        match diff {
            DiffEntry::Modified { property_changes, .. } => {
                assert_eq!(property_changes.len(), 1, "{property_changes:?}");
                assert_eq!(property_changes[0].name, "PrimaryPart");
            }
            other => panic!("expected Modified, got {other:?}"),
        }
    }

    // Adding cross-referencing ObjectValues (each pointing at the OTHER
    // twin's ClickPart) is exactly two additions — no ref fallout.
    let Some(diffs) = diff_files(
        &format!("{d}/police-station-with-2-identical-uni-givers-with-primary-part.rbxm"),
        &format!("{d}/police-station-with-the-uni-primary-parts-but-with-obj-value-that-references-the-other-uni-giver.rbxm"),
    ) else {
        return;
    };
    let (added, removed, modified, moved) = counts(&diffs);
    assert_eq!((added, removed, modified, moved), (2, 0, 0, 0), "{diffs:#?}");
}
