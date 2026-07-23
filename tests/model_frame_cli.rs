//! End-to-end regression for model-asset frame conflicts. Normalized diffs
//! cannot prove this behavior: they deliberately erase the placement being
//! tested. These assertions compare the raw serialized CFrames after each
//! resolution against the corresponding original branch.

use rbx_diff::{diff_model_doms_with_config, DiffConfig, DiffEntry};
use rbx_dom_weak::{InstanceBuilder, WeakDom};
use rbx_types::{CFrame, Matrix3, Variant, Vector3};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::Command;

fn load(path: &Path) -> WeakDom {
    rbx_binary::from_reader(BufReader::new(File::open(path).unwrap())).unwrap()
}

fn save(path: &Path, dom: &WeakDom) {
    rbx_binary::to_writer(
        BufWriter::new(File::create(path).unwrap()),
        dom,
        dom.root().children(),
    )
    .unwrap();
}

fn translated(x: f32) -> CFrame {
    CFrame::new(
        Vector3::new(x, 0.0, 0.0),
        Matrix3::new(
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        ),
    )
}

fn rotation_z(radians: f32) -> Matrix3 {
    let (sin, cos) = radians.sin_cos();
    Matrix3::new(
        Vector3::new(cos, -sin, 0.0),
        Vector3::new(sin, cos, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    )
}

fn rotate(matrix: Matrix3, vector: Vector3) -> Vector3 {
    Vector3::new(
        matrix.x.x * vector.x + matrix.x.y * vector.y + matrix.x.z * vector.z,
        matrix.y.x * vector.x + matrix.y.y * vector.y + matrix.y.z * vector.z,
        matrix.z.x * vector.x + matrix.z.y * vector.y + matrix.z.z * vector.z,
    )
}

fn multiply(a: Matrix3, b: Matrix3) -> Matrix3 {
    let column = |matrix: Matrix3, index: usize| match index {
        0 => Vector3::new(matrix.x.x, matrix.y.x, matrix.z.x),
        1 => Vector3::new(matrix.x.y, matrix.y.y, matrix.z.y),
        _ => Vector3::new(matrix.x.z, matrix.y.z, matrix.z.z),
    };
    let dot =
        |row: Vector3, column: Vector3| row.x * column.x + row.y * column.y + row.z * column.z;
    Matrix3::new(
        Vector3::new(
            dot(a.x, column(b, 0)),
            dot(a.x, column(b, 1)),
            dot(a.x, column(b, 2)),
        ),
        Vector3::new(
            dot(a.y, column(b, 0)),
            dot(a.y, column(b, 1)),
            dot(a.y, column(b, 2)),
        ),
        Vector3::new(
            dot(a.z, column(b, 0)),
            dot(a.z, column(b, 1)),
            dot(a.z, column(b, 2)),
        ),
    )
}

fn apply_delta(delta: CFrame, base: CFrame) -> CFrame {
    let rotated = rotate(delta.orientation, base.position);
    CFrame::new(
        Vector3::new(
            delta.position.x + rotated.x,
            delta.position.y + rotated.y,
            delta.position.z + rotated.z,
        ),
        multiply(delta.orientation, base.orientation),
    )
}

fn small_asset(offset: f32, transparency: f32) -> WeakDom {
    let part = |name: &str, x: f32| {
        InstanceBuilder::new("Part")
            .with_name(name)
            .with_property("CFrame", Variant::CFrame(translated(offset + x)))
    };
    WeakDom::new(
        InstanceBuilder::new("DataModel").with_child(
            InstanceBuilder::new("Model")
                .with_name("Truck")
                .with_property(
                    "WorldPivotData",
                    Variant::OptionalCFrame(Some(translated(offset))),
                )
                .with_child(part("A", 0.0))
                .with_child(
                    part("B", 4.0).with_property("Transparency", Variant::Float32(transparency)),
                ),
        ),
    )
}

/// Ours moves `Outer` (including `Inner`), while theirs moves only `Inner`.
/// Outer has four of the asset's five parts and Inner has three, so the two
/// branches establish different, overlapping strict-majority frames.
fn overlapping_nested_asset(outer_offset: f32, inner_offset: f32) -> WeakDom {
    let part = |name: &str, x: f32| {
        InstanceBuilder::new("Part")
            .with_name(name)
            .with_property("CFrame", Variant::CFrame(translated(x)))
    };
    let inner_origin = 20.0 + outer_offset + inner_offset;
    WeakDom::new(
        InstanceBuilder::new("DataModel").with_child(
            InstanceBuilder::new("Model")
                .with_name("Asset")
                .with_property(
                    "WorldPivotData",
                    Variant::OptionalCFrame(Some(translated(0.0))),
                )
                .with_child(part("Static", 0.0))
                .with_child(
                    InstanceBuilder::new("Model")
                        .with_name("Outer")
                        .with_property(
                            "WorldPivotData",
                            Variant::OptionalCFrame(Some(translated(10.0 + outer_offset))),
                        )
                        .with_child(part("OuterPart", 10.0 + outer_offset))
                        .with_child(
                            InstanceBuilder::new("Model")
                                .with_name("Inner")
                                .with_property(
                                    "WorldPivotData",
                                    Variant::OptionalCFrame(Some(translated(inner_origin))),
                                )
                                .with_child(part("InnerA", inner_origin))
                                .with_child(part("InnerB", inner_origin + 4.0))
                                .with_child(part("InnerC", inner_origin + 8.0)),
                        ),
                ),
        ),
    )
}

fn nested_rotated_asset(parent_delta: CFrame, child_local: CFrame) -> WeakDom {
    let world = |base: CFrame, child: bool| {
        let parent_world = apply_delta(parent_delta, base);
        if child {
            apply_delta(child_local, parent_world)
        } else {
            parent_world
        }
    };
    let part = |name: &str, base: CFrame, child: bool| {
        InstanceBuilder::new("Part")
            .with_name(name)
            .with_property("CFrame", Variant::CFrame(world(base, child)))
    };
    let parent_pivot = translated(5.0);
    let child_pivot = CFrame::new(Vector3::new(12.0, 3.0, 0.0), rotation_z(0.2));
    WeakDom::new(
        InstanceBuilder::new("DataModel").with_child(
            InstanceBuilder::new("Model")
                .with_name("Asset")
                .with_property(
                    "WorldPivotData",
                    Variant::OptionalCFrame(Some(world(parent_pivot, false))),
                )
                .with_child(part("ParentA", translated(2.0), false))
                .with_child(part(
                    "ParentB",
                    CFrame::new(Vector3::new(6.0, 2.0, 0.0), rotation_z(-0.15)),
                    false,
                ))
                .with_child(
                    InstanceBuilder::new("Model")
                        .with_name("Child")
                        .with_property(
                            "WorldPivotData",
                            Variant::OptionalCFrame(Some(world(child_pivot, true))),
                        )
                        .with_child(part(
                            "ChildA",
                            CFrame::new(Vector3::new(10.0, 2.0, 0.0), rotation_z(0.1)),
                            true,
                        ))
                        .with_child(part(
                            "ChildB",
                            CFrame::new(Vector3::new(14.0, 4.0, 0.0), rotation_z(-0.3)),
                            true,
                        )),
                ),
        ),
    )
}

fn spatial_values(dom: &WeakDom) -> Vec<(String, String, CFrame)> {
    let mut values = Vec::new();
    for instance in dom.descendants() {
        if let Some(Variant::CFrame(value)) = instance.properties.get(&"CFrame".into()) {
            values.push((instance.class.to_string(), instance.name.clone(), *value));
        }
        if let Some(Variant::OptionalCFrame(Some(value))) =
            instance.properties.get(&"WorldPivotData".into())
        {
            values.push((
                instance.class.to_string(),
                format!("{}.WorldPivotData", instance.name),
                *value,
            ));
        }
    }
    values
}

fn assert_cframe_close(actual: &CFrame, expected: &CFrame, label: &str) {
    let actual_rows = [
        actual.orientation.x,
        actual.orientation.y,
        actual.orientation.z,
    ];
    let expected_rows = [
        expected.orientation.x,
        expected.orientation.y,
        expected.orientation.z,
    ];
    let dx = actual.position.x - expected.position.x;
    let dy = actual.position.y - expected.position.y;
    let dz = actual.position.z - expected.position.z;
    let position_error = (dx * dx + dy * dy + dz * dz).sqrt();
    assert!(
        position_error < 0.01,
        "{label}: position error {position_error}; actual={actual:?}, expected={expected:?}"
    );
    for row in 0..3 {
        for column in 0..3 {
            let actual_component =
                [actual_rows[row].x, actual_rows[row].y, actual_rows[row].z][column];
            let expected_component = [
                expected_rows[row].x,
                expected_rows[row].y,
                expected_rows[row].z,
            ][column];
            assert!(
                (actual_component - expected_component).abs() < 1e-4,
                "{label}: orientation mismatch; actual={actual:?}, expected={expected:?}"
            );
        }
    }
}

fn assert_raw_spatial_match(actual_path: &Path, expected_path: &Path) {
    let actual = spatial_values(&load(actual_path));
    let expected = spatial_values(&load(expected_path));
    assert_eq!(actual.len(), expected.len());
    for ((actual_class, actual_name, actual_cf), (expected_class, expected_name, expected_cf)) in
        actual.iter().zip(&expected)
    {
        assert_eq!(actual_class, expected_class);
        assert_eq!(actual_name, expected_name);
        assert_cframe_close(
            actual_cf,
            expected_cf,
            &format!("{actual_class} {actual_name}"),
        );
    }
}

#[test]
fn two_way_diff_factors_rotated_parent_and_child_frames() {
    let identity = translated(0.0);
    let parent = CFrame::new(Vector3::new(45.0, -12.0, 8.0), rotation_z(0.55));
    let child = CFrame::new(Vector3::new(3.0, 7.0, -2.0), rotation_z(-0.35));
    let base = nested_rotated_asset(identity, identity);
    let mut side = nested_rotated_asset(parent, child);

    let (diffs, normalization) =
        diff_model_doms_with_config(&base, &mut side, &DiffConfig::default());
    let normalization = normalization.expect("parent and child frames should be detected");
    assert_eq!(normalization.frames.len(), 2, "{:#?}", normalization.frames);
    assert_eq!(diffs.len(), 2, "{diffs:#?}");
    assert!(diffs
        .iter()
        .all(|diff| matches!(diff, DiffEntry::ModelFrame { .. })));
    let parent_order = match &diffs[0] {
        DiffEntry::ModelFrame {
            path,
            order,
            parent_order,
            ..
        } => {
            assert!(path.ends_with("Asset"));
            assert_eq!(*parent_order, None);
            *order
        }
        _ => unreachable!(),
    };
    assert!(matches!(&diffs[1], DiffEntry::ModelFrame {
        path,
        parent_order: Some(order),
        ..
    } if path.ends_with("Asset.Child") && *order == parent_order));
}

#[test]
fn two_way_diff_cli_json_reports_frames_instead_of_descendant_cframes() {
    let identity = translated(0.0);
    let parent = CFrame::new(Vector3::new(45.0, -12.0, 8.0), rotation_z(0.55));
    let child = CFrame::new(Vector3::new(3.0, 7.0, -2.0), rotation_z(-0.35));
    let scratch = std::env::temp_dir().join(format!(
        "rbx-diff-model-frame-diff-test-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&scratch).unwrap();
    let base_path = scratch.join("base.rbxm");
    let side_path = scratch.join("side.rbxm");
    save(&base_path, &nested_rotated_asset(identity, identity));
    save(&side_path, &nested_rotated_asset(parent, child));

    let output = Command::new(env!("CARGO_BIN_EXE_rbx-diff"))
        .args([
            "diff",
            base_path.to_str().unwrap(),
            side_path.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let changes = json["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 2, "{json:#}");
    assert!(changes.iter().all(|change| change["type"] == "model_frame"));
    assert_eq!(json["summary"]["model_frames"], 2);
    assert_eq!(json["summary"]["modified"], 0);

    std::fs::remove_dir_all(scratch).unwrap();
}

fn run(binary: &str, args: &[&str], expected_success: bool) {
    let output = Command::new(binary).args(args).output().unwrap();
    assert_eq!(
        output.status.success(),
        expected_success,
        "command failed unexpectedly\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn tow_truck_independent_frame_and_pivot_choices_reconstruct_each_raw_branch() {
    let fixture = Path::new("tests-new/tow-truck-rotation");
    if !fixture.join("base.rbxm").exists() {
        eprintln!("SKIP: tow-truck-rotation fixture not present");
        return;
    }

    let binary = env!("CARGO_BIN_EXE_rbx-diff");
    let scratch =
        std::env::temp_dir().join(format!("rbx-diff-model-frame-test-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    let conflicted = scratch.join("conflicted.rbxm");

    let base = fixture.join("base.rbxm");
    let ours = fixture.join("ours.rbxm");
    let theirs = fixture.join("theirs.rbxm");
    run(
        binary,
        &[
            "merge",
            base.to_str().unwrap(),
            ours.to_str().unwrap(),
            theirs.to_str().unwrap(),
            "--output",
            conflicted.to_str().unwrap(),
        ],
        false,
    );

    for (side, expected) in [("ours", &ours), ("theirs", &theirs)] {
        let resolved: PathBuf = scratch.join(format!("resolved-{side}.rbxm"));
        std::fs::copy(&conflicted, &resolved).unwrap();
        run(
            binary,
            &[
                "resolve",
                resolved.to_str().unwrap(),
                "--take",
                side,
                "--entry",
                "Conflict_1",
            ],
            true,
        );
        run(
            binary,
            &[
                "resolve",
                resolved.to_str().unwrap(),
                "--take",
                side,
                "--entry",
                "Conflict_2",
            ],
            true,
        );
        run(
            binary,
            &["resolve", resolved.to_str().unwrap(), "--finalize"],
            true,
        );
        assert_raw_spatial_match(&resolved, expected);
    }

    std::fs::remove_dir_all(&scratch).unwrap();
}

#[test]
fn one_sided_frame_move_is_automatic_and_carries_the_other_sides_edit() {
    let binary = env!("CARGO_BIN_EXE_rbx-diff");
    let scratch = std::env::temp_dir().join(format!(
        "rbx-diff-model-frame-auto-test-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&scratch).unwrap();
    let base_path = scratch.join("base.rbxm");
    let ours_path = scratch.join("ours.rbxm");
    let theirs_path = scratch.join("theirs.rbxm");
    let output_path = scratch.join("output.rbxm");
    save(&base_path, &small_asset(0.0, 0.0));
    save(&ours_path, &small_asset(100.0, 0.0));
    save(&theirs_path, &small_asset(0.0, 0.5));

    run(
        binary,
        &[
            "merge",
            base_path.to_str().unwrap(),
            ours_path.to_str().unwrap(),
            theirs_path.to_str().unwrap(),
            "--output",
            output_path.to_str().unwrap(),
        ],
        true,
    );

    let output = load(&output_path);
    let a = output
        .descendants()
        .find(|instance| instance.name == "A")
        .unwrap();
    let b = output
        .descendants()
        .find(|instance| instance.name == "B")
        .unwrap();
    let Variant::CFrame(a_frame) = a.properties.get(&"CFrame".into()).unwrap() else {
        panic!("A.CFrame missing")
    };
    let Variant::CFrame(b_frame) = b.properties.get(&"CFrame".into()).unwrap() else {
        panic!("B.CFrame missing")
    };
    assert!((a_frame.position.x - 100.0).abs() < 1e-3);
    assert!((b_frame.position.x - 104.0).abs() < 1e-3);
    assert_eq!(
        b.properties.get(&"Transparency".into()),
        Some(&Variant::Float32(0.5))
    );
    std::fs::remove_dir_all(&scratch).unwrap();
}

#[test]
fn overlapping_nested_moves_compose_without_a_conflict() {
    let binary = env!("CARGO_BIN_EXE_rbx-diff");
    let scratch = std::env::temp_dir().join(format!(
        "rbx-diff-nested-model-frame-test-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&scratch).unwrap();
    let base_path = scratch.join("base.rbxm");
    let ours_path = scratch.join("ours.rbxm");
    let theirs_path = scratch.join("theirs.rbxm");
    let expected_path = scratch.join("expected.rbxm");
    let conflicted_path = scratch.join("conflicted.rbxm");
    save(&base_path, &overlapping_nested_asset(0.0, 0.0));
    save(&ours_path, &overlapping_nested_asset(100.0, 0.0));
    save(&theirs_path, &overlapping_nested_asset(0.0, -50.0));
    // The independent one-sided local frames compose: ours' outer +100, then
    // theirs' inner -50, leaving the inner content at base +50 in world space.
    save(&expected_path, &overlapping_nested_asset(100.0, -50.0));

    run(
        binary,
        &[
            "merge",
            base_path.to_str().unwrap(),
            ours_path.to_str().unwrap(),
            theirs_path.to_str().unwrap(),
            "--output",
            conflicted_path.to_str().unwrap(),
        ],
        true,
    );
    assert_raw_spatial_match(&conflicted_path, &expected_path);

    std::fs::remove_dir_all(&scratch).unwrap();
}

#[test]
fn nested_rotated_frame_choices_reconstruct_and_compose_in_top_down_order() {
    let binary = env!("CARGO_BIN_EXE_rbx-diff");
    let scratch = std::env::temp_dir().join(format!(
        "rbx-diff-nested-rotation-frame-test-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&scratch).unwrap();

    let identity = CFrame::identity();
    let ours_parent = CFrame::new(Vector3::new(30.0, -4.0, 0.0), rotation_z(0.65));
    let theirs_parent = CFrame::new(Vector3::new(-12.0, 18.0, 0.0), rotation_z(-0.4));
    let ours_child = CFrame::new(Vector3::new(3.0, 7.0, 0.0), rotation_z(-0.25));
    let theirs_child = CFrame::new(Vector3::new(-8.0, 2.0, 0.0), rotation_z(0.5));

    let base_path = scratch.join("base.rbxm");
    let ours_path = scratch.join("ours.rbxm");
    let theirs_path = scratch.join("theirs.rbxm");
    let mixed_path = scratch.join("mixed.rbxm");
    let conflicted_path = scratch.join("conflicted.rbxm");
    save(&base_path, &nested_rotated_asset(identity, identity));
    save(&ours_path, &nested_rotated_asset(ours_parent, ours_child));
    save(
        &theirs_path,
        &nested_rotated_asset(theirs_parent, theirs_child),
    );
    save(
        &mixed_path,
        &nested_rotated_asset(ours_parent, theirs_child),
    );

    run(
        binary,
        &[
            "merge",
            base_path.to_str().unwrap(),
            ours_path.to_str().unwrap(),
            theirs_path.to_str().unwrap(),
            "--output",
            conflicted_path.to_str().unwrap(),
        ],
        false,
    );

    for (label, parent_side, child_side, expected) in [
        ("ours", "ours", "ours", &ours_path),
        ("theirs", "theirs", "theirs", &theirs_path),
        ("mixed", "ours", "theirs", &mixed_path),
    ] {
        let resolved = scratch.join(format!("resolved-{label}.rbxm"));
        std::fs::copy(&conflicted_path, &resolved).unwrap();
        for (entry, side) in [("Conflict_1", parent_side), ("Conflict_2", child_side)] {
            run(
                binary,
                &[
                    "resolve",
                    resolved.to_str().unwrap(),
                    "--take",
                    side,
                    "--entry",
                    entry,
                ],
                true,
            );
        }
        run(
            binary,
            &["resolve", resolved.to_str().unwrap(), "--finalize"],
            true,
        );
        assert_raw_spatial_match(&resolved, expected);
    }

    std::fs::remove_dir_all(&scratch).unwrap();
}

#[test]
fn automatic_rotated_parent_is_deferred_with_a_child_frame_conflict() {
    let binary = env!("CARGO_BIN_EXE_rbx-diff");
    let scratch = std::env::temp_dir().join(format!(
        "rbx-diff-deferred-parent-frame-test-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&scratch).unwrap();

    let identity = CFrame::identity();
    let shared_parent = CFrame::new(Vector3::new(21.0, -9.0, 0.0), rotation_z(0.7));
    let ours_child = CFrame::new(Vector3::new(4.0, 6.0, 0.0), rotation_z(-0.35));
    let theirs_child = CFrame::new(Vector3::new(-7.0, 3.0, 0.0), rotation_z(0.45));
    let base_path = scratch.join("base.rbxm");
    let ours_path = scratch.join("ours.rbxm");
    let theirs_path = scratch.join("theirs.rbxm");
    let conflicted_path = scratch.join("conflicted.rbxm");
    save(&base_path, &nested_rotated_asset(identity, identity));
    save(&ours_path, &nested_rotated_asset(shared_parent, ours_child));
    save(
        &theirs_path,
        &nested_rotated_asset(shared_parent, theirs_child),
    );

    run(
        binary,
        &[
            "merge",
            base_path.to_str().unwrap(),
            ours_path.to_str().unwrap(),
            theirs_path.to_str().unwrap(),
            "--output",
            conflicted_path.to_str().unwrap(),
        ],
        false,
    );

    for (side, expected) in [("ours", &ours_path), ("theirs", &theirs_path)] {
        let resolved = scratch.join(format!("resolved-{side}.rbxm"));
        std::fs::copy(&conflicted_path, &resolved).unwrap();
        run(
            binary,
            &[
                "resolve",
                resolved.to_str().unwrap(),
                "--take",
                side,
                "--all",
            ],
            true,
        );
        run(
            binary,
            &["resolve", resolved.to_str().unwrap(), "--finalize"],
            true,
        );
        assert_raw_spatial_match(&resolved, expected);
    }

    std::fs::remove_dir_all(&scratch).unwrap();
}
