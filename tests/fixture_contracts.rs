//! Contracts for the real-world fixture directories that are too specialized
//! for the focused unit tests. The final inventory test makes adding a binary
//! in the fixture set an explicit decision: every asset must belong to a named
//! behavior contract rather than merely being parseable test data.

mod common;
use common::{fixture, fixtures_root};

use rbx_diff::{
    diff_doms, diff_model_doms_with_config, find_container, list_entries, DiffConfig, DiffEntry,
};
use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::{CFrame, Variant, Vector3};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const PIVOT_FRAME: &str = "model-pivot-reference-frame";
const ASSEMBLY: &str = "tow-truck/assembly-refactor-boundary";
const ROLLBACK: &str = "tow-truck/rollback-tow-remerge";

fn load(path: &Path) -> WeakDom {
    rbx_binary::from_reader(BufReader::new(File::open(path).unwrap())).unwrap()
}

fn scratch(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("rbx-diff-{label}-{}-{nonce}", std::process::id()));
    fs::create_dir(&path).unwrap();
    path
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_git-rbx"))
        .args(args)
        .output()
        .unwrap()
}

fn assert_status(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn counts(diffs: &[DiffEntry]) -> (usize, usize, usize, usize, usize) {
    let mut counts = (0, 0, 0, 0, 0);
    for diff in diffs {
        match diff {
            DiffEntry::Added { .. } => counts.0 += 1,
            DiffEntry::Removed { .. } => counts.1 += 1,
            DiffEntry::Modified { .. } => counts.2 += 1,
            DiffEntry::Moved { .. } => counts.3 += 1,
            DiffEntry::Pivoted { .. } => counts.4 += 1,
        }
    }
    counts
}

fn structural_path(dom: &WeakDom, referent: Ref) -> String {
    let mut segments = Vec::new();
    let mut current = referent;
    while current != dom.root_ref() {
        let Some(instance) = dom.get_by_ref(current) else {
            break;
        };
        segments.push(instance.name.clone());
        if instance.parent().is_none() {
            break;
        }
        current = instance.parent();
    }
    segments.reverse();
    segments.join(".")
}

fn content_uri(instance: &rbx_dom_weak::Instance) -> Option<String> {
    let raw = ["MeshContent", "MeshId"].iter().find_map(|name| {
        match instance.properties.get(&(*name).into())? {
            Variant::Content(content) => content.as_uri().map(str::to_owned),
            Variant::ContentId(content) => Some(content.as_str().to_owned()),
            Variant::String(content) => Some(content.clone()),
            _ => None,
        }
    })?;
    let id = raw
        .strip_prefix("rbxassetid://")
        .or_else(|| {
            raw.split("?id=")
                .nth(1)
                .map(|value| value.split('&').next().unwrap())
        })
        .filter(|value| !value.is_empty());
    Some(
        id.map(|value| format!("rbxassetid://{value}"))
            .unwrap_or(raw),
    )
}

fn mesh_set(dom: &WeakDom) -> BTreeMap<String, usize> {
    let mut result = BTreeMap::new();
    for instance in dom
        .descendants()
        .filter(|instance| instance.class.as_str() == "MeshPart")
    {
        if let Some(id) = content_uri(instance).filter(|id| !id.is_empty()) {
            *result.entry(id).or_default() += 1;
        }
    }
    result
}

fn reference_part(dom: &WeakDom, name: &str) -> Option<CFrame> {
    dom.descendants().find_map(|instance| {
        if instance.name != name {
            return None;
        }
        match instance.properties.get(&"CFrame".into()) {
            Some(Variant::CFrame(frame)) => Some(*frame),
            _ => None,
        }
    })
}

fn object_space_position(reference: CFrame, world: CFrame) -> Vector3 {
    let delta = Vector3::new(
        world.position.x - reference.position.x,
        world.position.y - reference.position.y,
        world.position.z - reference.position.z,
    );
    Vector3::new(
        reference.orientation.x.x * delta.x
            + reference.orientation.y.x * delta.y
            + reference.orientation.z.x * delta.z,
        reference.orientation.x.y * delta.x
            + reference.orientation.y.y * delta.y
            + reference.orientation.z.y * delta.z,
        reference.orientation.x.z * delta.x
            + reference.orientation.y.z * delta.y
            + reference.orientation.z.z * delta.z,
    )
}

fn mesh_offsets(dom: &WeakDom, reference_name: &str) -> BTreeMap<String, Vec<Vector3>> {
    let Some(reference) = reference_part(dom, reference_name) else {
        return BTreeMap::new();
    };
    let mut result: BTreeMap<String, Vec<Vector3>> = BTreeMap::new();
    for instance in dom
        .descendants()
        .filter(|instance| instance.class.as_str() == "MeshPart")
    {
        let (Some(id), Some(Variant::CFrame(frame))) = (
            content_uri(instance),
            instance.properties.get(&"CFrame".into()),
        ) else {
            continue;
        };
        result
            .entry(id)
            .or_default()
            .push(object_space_position(reference, *frame));
    }
    result
}

fn assert_shared_single_mesh_offsets(actual: &WeakDom, expected: &WeakDom) {
    let actual = mesh_offsets(actual, "DriveSeat");
    let expected = mesh_offsets(expected, "DriveSeat");
    for (id, actual_offsets) in &actual {
        let Some(expected_offsets) = expected.get(id) else {
            continue;
        };
        if actual_offsets.len() != 1 || expected_offsets.len() != 1 {
            continue;
        }
        let a = actual_offsets[0];
        let b = expected_offsets[0];
        let distance = ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt();
        assert!(distance <= 0.05, "mesh {id} moved {distance} studs");
    }
}

fn reference_name(dom: &WeakDom, value: Option<&Variant>) -> String {
    match value {
        Some(Variant::Ref(referent)) if !referent.is_none() => dom
            .get_by_ref(*referent)
            .map(|instance| instance.name.clone())
            .unwrap_or_else(|| "<missing>".to_string()),
        _ => "<nil>".to_string(),
    }
}

fn weld_map(dom: &WeakDom) -> BTreeMap<String, usize> {
    let mut result = BTreeMap::new();
    for instance in dom.descendants().filter(|instance| {
        matches!(
            instance.class.as_str(),
            "Weld" | "WeldConstraint" | "Motor6D"
        )
    }) {
        let mut names = [
            reference_name(dom, instance.properties.get(&"Part0".into())),
            reference_name(dom, instance.properties.get(&"Part1".into())),
        ];
        names.sort();
        *result
            .entry(format!("{}|{}", names[0], names[1]))
            .or_default() += 1;
    }
    result
}

fn attachment_set(dom: &WeakDom) -> BTreeMap<String, usize> {
    let mut result = BTreeMap::new();
    for instance in dom
        .descendants()
        .filter(|instance| instance.class.as_str() == "Attachment")
    {
        *result.entry(instance.name.clone()).or_default() += 1;
    }
    result
}

fn module_sources(dom: &WeakDom) -> BTreeMap<String, String> {
    dom.descendants()
        .filter(|instance| instance.class.as_str() == "ModuleScript")
        .filter_map(|instance| match instance.properties.get(&"Source".into()) {
            Some(Variant::String(source)) => {
                Some((structural_path(dom, instance.referent()), source.clone()))
            }
            _ => None,
        })
        .collect()
}

fn assert_one_sided_module_edits(
    base: &WeakDom,
    ours: &WeakDom,
    theirs: &WeakDom,
    result: &WeakDom,
) {
    let base = module_sources(base);
    let ours = module_sources(ours);
    let theirs = module_sources(theirs);
    let result = module_sources(result);
    let paths: BTreeSet<_> = base
        .keys()
        .chain(ours.keys())
        .chain(theirs.keys())
        .cloned()
        .collect();
    for path in paths {
        let b = base.get(&path);
        let o = ours.get(&path);
        let t = theirs.get(&path);
        let expected = if o == t {
            o
        } else if o == b {
            t
        } else if t == b {
            o
        } else {
            continue;
        };
        assert_eq!(
            result.get(&path),
            expected,
            "one-sided source edit at {path}"
        );
    }
}

fn spatial_values(dom: &WeakDom) -> BTreeMap<String, CFrame> {
    let mut values = BTreeMap::new();
    for instance in dom.descendants() {
        let path = structural_path(dom, instance.referent());
        if let Some(Variant::CFrame(value)) = instance.properties.get(&"CFrame".into()) {
            values.insert(format!("{path}.CFrame"), *value);
        }
        if let Some(Variant::OptionalCFrame(Some(value))) =
            instance.properties.get(&"WorldPivotData".into())
        {
            values.insert(format!("{path}.WorldPivotData"), *value);
        }
    }
    values
}

fn assert_cframe_close(actual: CFrame, expected: CFrame, label: &str) {
    let a = [
        actual.position.x,
        actual.position.y,
        actual.position.z,
        actual.orientation.x.x,
        actual.orientation.x.y,
        actual.orientation.x.z,
        actual.orientation.y.x,
        actual.orientation.y.y,
        actual.orientation.y.z,
        actual.orientation.z.x,
        actual.orientation.z.y,
        actual.orientation.z.z,
    ];
    let b = [
        expected.position.x,
        expected.position.y,
        expected.position.z,
        expected.orientation.x.x,
        expected.orientation.x.y,
        expected.orientation.x.z,
        expected.orientation.y.x,
        expected.orientation.y.y,
        expected.orientation.y.z,
        expected.orientation.z.x,
        expected.orientation.z.y,
        expected.orientation.z.z,
    ];
    for (index, (actual, expected)) in a.into_iter().zip(b).enumerate() {
        let tolerance = if index < 3 { 0.01 } else { 1.0e-4 };
        assert!(
            (actual - expected).abs() <= tolerance,
            "{label}: component {index} differs ({actual} vs {expected})"
        );
    }
}

fn assert_spatial_match(actual: &WeakDom, expected: &WeakDom) {
    let actual = spatial_values(actual);
    let expected = spatial_values(expected);
    assert_eq!(
        actual.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>()
    );
    for (path, expected) in expected {
        assert_cframe_close(actual[&path], expected, &path);
    }
}

fn named_property(dom: &WeakDom, instance_name: &str, property: &str) -> Variant {
    dom.descendants()
        .find(|instance| instance.name == instance_name)
        .and_then(|instance| instance.properties.get(&property.into()))
        .unwrap_or_else(|| panic!("missing {instance_name}.{property}"))
        .clone()
}

#[test]
fn model_pivot_reference_frame_separates_content_placement_from_world_pivot() {
    let Some(dir) = fixture(PIVOT_FRAME) else {
        return;
    };
    let base_path = dir.join("base.rbxm");
    let ours_path = dir.join("ours.rbxm");
    let theirs_path = dir.join("theirs.rbxm");
    let expected_path = dir.join("expected.rbxm");
    let base = load(&base_path);
    let ours = load(&ours_path);
    let theirs = load(&theirs_path);
    let expected = load(&expected_path);

    for (path, wanted_properties) in [
        (&ours_path, ["WorldPivotData", "Color"]),
        (&theirs_path, ["WorldPivotData", "Transparency"]),
    ] {
        let mut normalized = load(path);
        let (diffs, pivots) =
            diff_model_doms_with_config(&base, &mut normalized, &DiffConfig::default());
        assert_eq!(counts(&diffs), (0, 0, 2, 0, 1), "{diffs:#?}");
        assert_eq!(pivots.unwrap().pivots.len(), 1);
        let properties: BTreeSet<_> = diffs
            .iter()
            .filter_map(|diff| match diff {
                DiffEntry::Modified {
                    property_changes, ..
                } => Some(property_changes),
                _ => None,
            })
            .flatten()
            .map(|change| change.name.as_str())
            .collect();
        assert_eq!(properties, BTreeSet::from(wanted_properties));
    }
    let expected_diff = diff_doms(&base, &expected);
    assert_eq!(
        counts(&expected_diff),
        (0, 0, 2, 0, 0),
        "{expected_diff:#?}"
    );

    let scratch = scratch("pivot-frame");
    let conflicted = scratch.join("conflicted.rbxm");
    let output = run(&[
        "merge",
        base_path.to_str().unwrap(),
        ours_path.to_str().unwrap(),
        theirs_path.to_str().unwrap(),
        "--output",
        conflicted.to_str().unwrap(),
    ]);
    assert_status(&output, 1);
    let stamped = load(&conflicted);
    let entries = list_entries(&stamped, find_container(&stamped).unwrap());
    assert_eq!(entries.len(), 2, "{:#?}", entries);
    assert!(entries.iter().any(|entry| {
        entry.kind == "Property" && entry.property.as_deref() == Some("WorldPivotData")
    }));
    assert!(entries.iter().any(|entry| entry.kind == "Pivot"));

    for (side, spatial_source) in [("ours", &ours), ("theirs", &theirs)] {
        let resolved = scratch.join(format!("resolved-{side}.rbxm"));
        fs::copy(&conflicted, &resolved).unwrap();
        let output = run(&[
            "resolve",
            resolved.to_str().unwrap(),
            "--take",
            side,
            "--all",
        ]);
        assert_status(&output, 0);
        let output = run(&["resolve", resolved.to_str().unwrap(), "--finalize"]);
        assert_status(&output, 0);
        let result = load(&resolved);
        assert_spatial_match(&result, spatial_source);
        assert_eq!(
            named_property(&result, "Chassis", "Color"),
            named_property(&ours, "Chassis", "Color")
        );
        assert_eq!(
            named_property(&result, "Bed", "Transparency"),
            named_property(&theirs, "Bed", "Transparency")
        );
    }
    fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn assembly_refactor_fixture_preserves_graph_markers_and_stable_geometry() {
    let Some(dir) = fixture(ASSEMBLY) else {
        return;
    };
    let before = load(&dir.join("before-refactor.rbxm"));
    let after = load(&dir.join("after-refactor-part-graphs.rbxm"));
    let diffs = diff_doms(&before, &after);
    assert_eq!(counts(&diffs), (56, 116, 1608, 24, 0), "{diffs:#?}");

    let names: BTreeSet<_> = after
        .descendants()
        .map(|instance| instance.name.as_str())
        .collect();
    for name in [
        "door:rear",
        "wall:front",
        "wall:left",
        "wall:right",
        "paint:accessory",
        "paint:body",
        "AccessorySocketAttachments",
        "AssemblySocketAttachments",
        "MountAttachment",
    ] {
        assert!(
            names.contains(name),
            "post-refactor model is missing {name}"
        );
    }

    let before_meshes = mesh_set(&before);
    let after_meshes = mesh_set(&after);
    let stable_meshes = before_meshes
        .keys()
        .filter(|id| after_meshes.contains_key(*id))
        .count();
    assert!(
        stable_meshes >= 11,
        "expected at least the documented 11 cab meshes to retain identity, got {stable_meshes}"
    );
}

#[test]
fn rollback_remerge_resolves_and_preserves_the_semantic_oracle() {
    let Some(dir) = fixture(ROLLBACK) else {
        return;
    };
    let base_path = dir.join("base.rbxm");
    let ours_path = dir.join("ours-rollback-tow-new-model.rbxm");
    let theirs_path = dir.join("theirs-tow-truck-improvements.rbxm");
    let expected_path = dir.join("merged-expected.rbxm");
    let scratch = scratch("rollback-remerge");
    let conflicted = scratch.join("conflicted.rbxm");

    let output = run(&[
        "merge",
        base_path.to_str().unwrap(),
        ours_path.to_str().unwrap(),
        theirs_path.to_str().unwrap(),
        "--output",
        conflicted.to_str().unwrap(),
    ]);
    assert_status(&output, 1);
    let stamped = load(&conflicted);
    let entries = list_entries(&stamped, find_container(&stamped).unwrap());
    assert_eq!(entries.len(), 1, "{:#?}", entries);
    assert_eq!(entries[0].kind, "DeleteVsEdit");
    assert!(entries[0].path.ends_with("rollback-tow.WheelLift"));

    let mut ours_result_path = None;
    for side in ["ours", "theirs"] {
        let resolved = scratch.join(format!("resolved-{side}.rbxm"));
        fs::copy(&conflicted, &resolved).unwrap();
        assert_status(
            &run(&[
                "resolve",
                resolved.to_str().unwrap(),
                "--take",
                side,
                "--all",
            ]),
            0,
        );
        assert_status(
            &run(&["resolve", resolved.to_str().unwrap(), "--finalize"]),
            0,
        );
        assert!(find_container(&load(&resolved)).is_none());
        if side == "ours" {
            ours_result_path = Some(resolved);
        }
    }

    let result = load(ours_result_path.as_ref().unwrap());
    let expected = load(&expected_path);
    assert_eq!(mesh_set(&result), mesh_set(&expected));
    assert_eq!(weld_map(&result), weld_map(&expected));
    assert_eq!(attachment_set(&result), attachment_set(&expected));
    assert_shared_single_mesh_offsets(&result, &expected);
    assert!(result
        .descendants()
        .any(|instance| instance.class.as_str() == "HingeConstraint"));
    let residual = diff_doms(&expected, &result);
    assert!(
        residual.iter().all(|diff| match diff {
            DiffEntry::Modified {
                property_changes, ..
            } => property_changes
                .iter()
                .all(|change| matches!(change.name.as_str(), "CFrame" | "Source")),
            _ => false,
        }),
        "collision structure or authored physics drifted: {residual:#?}"
    );
    assert_one_sided_module_edits(
        &load(&base_path),
        &load(&ours_path),
        &load(&theirs_path),
        &result,
    );
    fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn every_tests_new_binary_fixture_has_an_explicit_contract() {
    const COVERED: &[&str] = &[
        "rc-builds/rc_fresh_build.rbxl",
        "rc-builds/rc_manually_saved_build.rbxl",
        "rc-builds/rc_menu_gui_removed.rbxl",
        "rcdev-maps/case_1/map_2.rbxm",
        "rcdev-maps/case_1/rcdev_map_current.rbxm",
        "rcdev-maps/case_2/rcdev_current.rbxl",
        "rcdev-maps/case_2/rcdev_old.rbxl",
        "rcdev-maps/case_3/rcdev.rbxl",
        "model-pivot-reference-frame/base.rbxm",
        "model-pivot-reference-frame/expected.rbxm",
        "model-pivot-reference-frame/ours.rbxm",
        "model-pivot-reference-frame/theirs.rbxm",
        "models-moved/rc_build_saved_manually_with_1_tree_moved.rbxl",
        "models-moved/rc_build_saved_manually_with_2_trees_moved.rbxl",
        "models-moved/rc_police_station_model_moved_with_internal_models_moved_too.rbxl",
        "referential-properties/obj-value/police-station-with-2-identical-uni-givers-with-primary-part.rbxm",
        "referential-properties/obj-value/police-station-with-the-uni-primary-parts-but-with-obj-value-that-references-the-other-uni-giver.rbxm",
        "referential-properties/obj-value/police-station.rbxm",
        "referential-properties/primary-part/model-with-grey-primary-part-and-has-dupe-children-names.rbxm",
        "referential-properties/primary-part/model-with-yellow-primary-part-and-has-dupe-children-names.rbxm",
        "tow-truck-rotation/base.rbxm",
        "tow-truck-rotation/ours.rbxm",
        "tow-truck-rotation/theirs.rbxm",
        "tow-truck/assembly-refactor-boundary/after-refactor-part-graphs.rbxm",
        "tow-truck/assembly-refactor-boundary/before-refactor.rbxm",
        "tow-truck/origin-merge-into-tow-truck/base.rbxm",
        "tow-truck/origin-merge-into-tow-truck/merged-expected.rbxm",
        "tow-truck/origin-merge-into-tow-truck/ours-tow-truck-rig.rbxm",
        "tow-truck/origin-merge-into-tow-truck/theirs-origin-assembly.rbxm",
        "tow-truck/rollback-tow-remerge/base.rbxm",
        "tow-truck/rollback-tow-remerge/merged-expected.rbxm",
        "tow-truck/rollback-tow-remerge/ours-rollback-tow-new-model.rbxm",
        "tow-truck/rollback-tow-remerge/theirs-tow-truck-improvements.rbxm",
        "union-operation/separated-parts.rbxm",
        "union-operation/unioned-parts-in-same-spot-but-diff-geometry.rbxm",
        "union-operation/unioned-parts.rbxm",
    ];

    fn visit(root: &Path, current: &Path, files: &mut BTreeSet<String>) {
        for entry in fs::read_dir(current).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else if matches!(
                path.extension().and_then(|extension| extension.to_str()),
                Some("rbxm" | "rbxmx" | "rbxl" | "rbxlx")
            ) {
                files.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }

    let Some(root) = fixtures_root() else {
        return;
    };
    let mut actual = BTreeSet::new();
    visit(&root, &root, &mut actual);
    let covered: BTreeSet<_> = COVERED.iter().map(|path| (*path).to_string()).collect();
    assert_eq!(
        actual, covered,
        "update a behavioral contract for fixture changes"
    );
    let directories: BTreeSet<_> = COVERED
        .iter()
        .filter_map(|path| Path::new(path).parent())
        .collect();
    for directory in directories {
        assert!(
            root.join(directory).join("README.md").is_file(),
            "{} needs a README explaining its fixture contract",
            directory.display()
        );
    }
}
