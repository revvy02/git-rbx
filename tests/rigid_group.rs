//! Rigid-transform grouping: both sides move the same set of parts as rigid
//! units (every part shares one world delta per side) — the per-part CFrame
//! conflicts must cluster into a single group, while a part moved
//! differently stays ungrouped. Resolving by group name fans out to members.

use git_rbx::{
    detect_rigid_groups, finalize, find_container, list_entries, mark_entry, merge_doms,
    stamp_conflicts, stamp_rigid_groups, DiffConfig,
};
use rbx_dom_weak::{InstanceBuilder, WeakDom};
use rbx_types::{CFrame, Matrix3, Variant, Vector3};

fn identity() -> Matrix3 {
    Matrix3::new(
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    )
}

fn part_at(name: &str, x: f32, y: f32, z: f32) -> InstanceBuilder {
    part_with_cframe(name, CFrame::new(Vector3::new(x, y, z), identity()))
}

fn part_with_cframe(name: &str, cframe: CFrame) -> InstanceBuilder {
    InstanceBuilder::new("Part")
        .with_name(name)
        .with_property("CFrame", Variant::CFrame(cframe))
}

fn position_of(dom: &WeakDom, name: &str) -> Vector3 {
    let instance = dom
        .descendants()
        .find(|instance| instance.name == name && instance.class.as_str() == "Part")
        .expect("part exists");
    match instance.properties.get(&"CFrame".into()) {
        Some(Variant::CFrame(cframe)) => cframe.position,
        other => panic!("expected CFrame, got {other:?}"),
    }
}

/// Three parts in a model, plus one loner part.
fn dom_with_offsets(rig_offset: (f32, f32, f32), loner_offset: (f32, f32, f32)) -> WeakDom {
    let (rx, ry, rz) = rig_offset;
    let (lx, ly, lz) = loner_offset;
    WeakDom::new(
        InstanceBuilder::new("Folder").with_name("root").with_child(
            InstanceBuilder::new("Model")
                .with_name("Rig")
                .with_child(part_at("A", 0.0 + rx, 0.0 + ry, 0.0 + rz))
                .with_child(part_at("B", 4.0 + rx, 0.0 + ry, 0.0 + rz))
                .with_child(part_at("C", 0.0 + rx, 4.0 + ry, 0.0 + rz))
                .with_child(part_at("Loner", 20.0 + lx, 0.0 + ly, 0.0 + lz)),
        ),
    )
}

#[test]
fn rigid_moves_group_and_outliers_stay_single() {
    let mut base = dom_with_offsets((0.0, 0.0, 0.0), (0.0, 0.0, 0.0));
    // Ours: whole rig +100 on X (Loner moved the same way — one drag).
    let ours = dom_with_offsets((100.0, 0.0, 0.0), (100.0, 0.0, 0.0));
    // Theirs: rig +50 on Z, but Loner independently moved +7 on Y.
    let theirs = dom_with_offsets((0.0, 0.0, 50.0), (0.0, 7.0, 50.0));

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert_eq!(result.conflicts.len(), 4, "{:?}", result.conflicts);

    let groups = detect_rigid_groups(&base, &result.conflicts);
    assert_eq!(groups.len(), 1, "expected exactly one rigid group");
    let group = &groups[0];
    assert_eq!(group.members.len(), 3);
    assert_eq!(group.path, "root.Rig");
    assert!((group.delta_ours.position.x - 100.0).abs() < 1e-3);
    assert!((group.delta_theirs.position.z - 50.0).abs() < 1e-3);

    // Stamp and verify the file-level contract: members carry the Group
    // attribute and the loner does not.
    stamp_conflicts(&mut base, &ours, &theirs, &result);
    stamp_rigid_groups(&mut base, &groups);

    // Group metadata and its CFrame attributes must survive the actual file
    // format before they drive resolution.
    let mut buffer = Vec::new();
    rbx_binary::to_writer(&mut buffer, &base, base.root().children()).unwrap();
    let mut base: WeakDom = rbx_binary::from_reader(buffer.as_slice()).unwrap();

    let container = find_container(&base).expect("container");
    let entries = list_entries(&base, container);
    assert_eq!(entries.len(), 4, "group folders must not appear as entries");
    let grouped: Vec<_> = entries.iter().filter(|e| e.group.is_some()).collect();
    assert_eq!(grouped.len(), 3);
    assert!(grouped
        .iter()
        .all(|e| e.group.as_deref() == Some("Group_1")));
    let loner = entries.iter().find(|e| e.path.ends_with("Loner")).unwrap();
    assert!(loner.group.is_none(), "outlier must stay ungrouped");

    // Taking the group fans out to exact stored branch values. No computed
    // group delta is applied to the result.
    let decisions: Vec<_> = entries
        .iter()
        .map(|entry| {
            let side = if entry.group.as_deref() == Some("Group_1") {
                "theirs"
            } else {
                "ours"
            };
            (entry.entry_ref, side)
        })
        .collect();
    for (entry_ref, side) in decisions {
        mark_entry(&mut base, entry_ref, side).unwrap();
    }
    finalize(&mut base).unwrap();
    assert_eq!(position_of(&base, "A"), Vector3::new(0.0, 0.0, 50.0));
    assert_eq!(position_of(&base, "B"), Vector3::new(4.0, 0.0, 50.0));
    assert_eq!(position_of(&base, "C"), Vector3::new(0.0, 4.0, 50.0));
    assert_eq!(position_of(&base, "Loner"), Vector3::new(120.0, 0.0, 0.0));
}

fn anchored_model(name: &str, origin_x: f32, offset: (f32, f32, f32)) -> InstanceBuilder {
    let (x, y, z) = offset;
    InstanceBuilder::new("Model")
        .with_name(name)
        .with_property(
            "WorldPivotData",
            Variant::OptionalCFrame(Some(CFrame::new(
                Vector3::new(origin_x + x, y, z),
                identity(),
            ))),
        )
        .with_child(part_at("Left", origin_x - 2.0 + x, y, z))
        .with_child(part_at("Right", origin_x + 2.0 + x, y, z))
}

fn two_anchored_models(offset: (f32, f32, f32)) -> WeakDom {
    WeakDom::new(
        InstanceBuilder::new("Folder")
            .with_name("root")
            .with_child(anchored_model("A", 0.0, offset))
            .with_child(anchored_model("B", 100.0, offset)),
    )
}

#[test]
fn model_pivots_keep_unrelated_identical_moves_separate() {
    let mut base = two_anchored_models((0.0, 0.0, 0.0));
    let ours = two_anchored_models((10.0, 0.0, 0.0));
    let theirs = two_anchored_models((0.0, 0.0, 20.0));

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert_eq!(result.conflicts.len(), 6, "{:?}", result.conflicts);
    let groups = detect_rigid_groups(&base, &result.conflicts);

    assert_eq!(groups.len(), 2, "one anchored group per model: {groups:?}");
    let mut summaries: Vec<_> = groups
        .iter()
        .map(|group| (group.path.as_str(), group.members.len()))
        .collect();
    summaries.sort_unstable();
    assert_eq!(summaries, vec![("root.A", 3), ("root.B", 3)]);

    for group in groups {
        let properties: Vec<_> = group
            .members
            .iter()
            .map(|&index| &result.conflicts[index].kind)
            .collect();
        assert!(properties.iter().any(|kind| matches!(
            kind,
            git_rbx::ConflictKind::Property { name } if name == "WorldPivotData"
        )));
    }
}

fn independently_pivoted_model(
    content_offset: (f32, f32, f32),
    pivot_offset: (f32, f32, f32),
) -> WeakDom {
    let (x, y, z) = content_offset;
    let (px, py, pz) = pivot_offset;
    WeakDom::new(
        InstanceBuilder::new("Folder").with_name("root").with_child(
            InstanceBuilder::new("Model")
                .with_name("TowTruck")
                .with_property(
                    "WorldPivotData",
                    Variant::OptionalCFrame(Some(CFrame::new(
                        Vector3::new(px, py, pz),
                        identity(),
                    ))),
                )
                .with_child(part_at("Chassis", x, y, z))
                .with_child(part_at("Bed", x + 4.0, y + 1.0, z))
                .with_child(part_at("Beacon", x - 2.0, y + 2.0, z - 1.0)),
        ),
    )
}

#[test]
fn independent_model_pivot_joins_descendant_content_move() {
    let mut base = independently_pivoted_model((0.0, 0.0, 0.0), (0.0, 0.0, 0.0));
    let ours = independently_pivoted_model((140.0, 12.0, 299.0), (210.0, 31.0, 323.0));
    let theirs = independently_pivoted_model((-85.0, 20.0, -110.0), (-63.0, 13.0, -305.0));

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert_eq!(result.conflicts.len(), 4, "{:?}", result.conflicts);

    let groups = detect_rigid_groups(&base, &result.conflicts);
    assert_eq!(groups.len(), 1, "pivot and content should be one decision");
    assert_eq!(groups[0].path, "root.TowTruck");
    assert_eq!(groups[0].members.len(), 4);
    assert!((groups[0].delta_ours.position.x - 140.0).abs() < 1e-3);
    assert!((groups[0].delta_theirs.position.z + 110.0).abs() < 1e-3);
}

fn attachment_dom(offset: (f32, f32, f32)) -> WeakDom {
    let (x, y, z) = offset;
    let attachment = |name: &str, base_x: f32| {
        InstanceBuilder::new("Attachment")
            .with_name(name)
            .with_property(
                "CFrame",
                Variant::CFrame(CFrame::new(Vector3::new(base_x + x, y, z), identity())),
            )
    };
    WeakDom::new(
        InstanceBuilder::new("Folder").with_name("root").with_child(
            InstanceBuilder::new("Part")
                .with_name("Host")
                .with_child(attachment("A", 0.0))
                .with_child(attachment("B", 4.0)),
        ),
    )
}

#[test]
fn local_cframe_properties_are_not_rigid_move_candidates() {
    let mut base = attachment_dom((0.0, 0.0, 0.0));
    let ours = attachment_dom((10.0, 0.0, 0.0));
    let theirs = attachment_dom((0.0, 0.0, 20.0));

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert_eq!(result.conflicts.len(), 2, "{:?}", result.conflicts);
    assert!(
        detect_rigid_groups(&base, &result.conflicts).is_empty(),
        "Attachment.CFrame is parent-local, not a world placement"
    );
}

fn rotation_z(radians: f32) -> Matrix3 {
    let (sin, cos) = radians.sin_cos();
    Matrix3::new(
        Vector3::new(cos, -sin, 0.0),
        Vector3::new(sin, cos, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    )
}

fn rotate(rotation: Matrix3, vector: Vector3) -> Vector3 {
    Vector3::new(
        rotation.x.x * vector.x + rotation.x.y * vector.y + rotation.x.z * vector.z,
        rotation.y.x * vector.x + rotation.y.y * vector.y + rotation.y.z * vector.z,
        rotation.z.x * vector.x + rotation.z.y * vector.y + rotation.z.z * vector.z,
    )
}

fn multiply(a: Matrix3, b: Matrix3) -> Matrix3 {
    let column = |matrix: Matrix3, index: usize| match index {
        0 => Vector3::new(matrix.x.x, matrix.y.x, matrix.z.x),
        1 => Vector3::new(matrix.x.y, matrix.y.y, matrix.z.y),
        _ => Vector3::new(matrix.x.z, matrix.y.z, matrix.z.z),
    };
    let dot = |row: Vector3, col: Vector3| row.x * col.x + row.y * col.y + row.z * col.z;
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

fn apply_world_delta(base: CFrame, rotation: Matrix3, translation: Vector3) -> CFrame {
    let position = rotate(rotation, base.position);
    CFrame::new(
        Vector3::new(
            position.x + translation.x,
            position.y + translation.y,
            position.z + translation.z,
        ),
        multiply(rotation, base.orientation),
    )
}

fn rotated_parts(rotation: Matrix3, translation: Vector3) -> WeakDom {
    let a = CFrame::new(Vector3::new(1.0, 0.0, 0.0), identity());
    let b = CFrame::new(Vector3::new(0.0, 2.0, 0.0), rotation_z(0.3));
    WeakDom::new(
        InstanceBuilder::new("Folder")
            .with_name("root")
            .with_child(part_with_cframe(
                "A",
                apply_world_delta(a, rotation, translation),
            ))
            .with_child(part_with_cframe(
                "B",
                apply_world_delta(b, rotation, translation),
            )),
    )
}

#[test]
fn loose_parts_group_under_shared_rotations() {
    let mut base = rotated_parts(identity(), Vector3::new(0.0, 0.0, 0.0));
    let ours_rotation = rotation_z(std::f32::consts::FRAC_PI_2);
    let theirs_rotation = rotation_z(-std::f32::consts::FRAC_PI_2);
    let ours = rotated_parts(ours_rotation, Vector3::new(10.0, 0.0, 0.0));
    let theirs = rotated_parts(theirs_rotation, Vector3::new(0.0, 20.0, 0.0));

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert_eq!(result.conflicts.len(), 2, "{:?}", result.conflicts);
    let groups = detect_rigid_groups(&base, &result.conflicts);
    assert_eq!(groups.len(), 1, "{groups:?}");
    assert_eq!(groups[0].members.len(), 2);
    assert!((groups[0].delta_ours.orientation.x.y + 1.0).abs() < 1e-4);
    assert!((groups[0].delta_theirs.orientation.x.y - 1.0).abs() < 1e-4);
}
