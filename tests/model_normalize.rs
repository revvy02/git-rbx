use rbx_diff::{
    merge_doms, normalize_model_dom_to_base, normalize_model_merge_frames, ConflictKind, DiffConfig,
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

fn cframe(x: f32, y: f32, z: f32) -> CFrame {
    CFrame::new(Vector3::new(x, y, z), identity())
}

fn part(name: &str, x: f32) -> InstanceBuilder {
    InstanceBuilder::new("Part")
        .with_name(name)
        .with_property("CFrame", Variant::CFrame(cframe(x, 0.0, 0.0)))
}

fn asset(
    content_offset: f32,
    pivot_x: f32,
    c_local_offset: f32,
    edit: Option<&str>,
    add_part: bool,
) -> WeakDom {
    let attachment = InstanceBuilder::new("Attachment")
        .with_name("Socket")
        .with_property("CFrame", Variant::CFrame(cframe(1.0, 2.0, 3.0)));
    let mut a = part("A", content_offset).with_child(attachment);
    let mut b = part("B", content_offset + 4.0);
    if edit == Some("ours") {
        a = a.with_property("Transparency", Variant::Float32(0.25));
    }
    if edit == Some("theirs") {
        b = b.with_property("Reflectance", Variant::Float32(0.5));
    }

    let mut model = InstanceBuilder::new("Model")
        .with_name("Truck")
        .with_property(
            "WorldPivotData",
            Variant::OptionalCFrame(Some(cframe(pivot_x, 0.0, 0.0))),
        )
        .with_child(a)
        .with_child(b)
        .with_child(part("C", content_offset + 8.0 + c_local_offset));
    if add_part {
        model = model.with_child(part("D", content_offset + 16.0));
    }

    WeakDom::new(
        InstanceBuilder::new("DataModel")
            .with_name("root")
            .with_child(model),
    )
}

/// An asset where the nested model owns three of four parts. Moving only the
/// nested model therefore wins the raw strict-majority vote even though its
/// consensus boundary differs from the unchanged branch's asset-wide vote.
fn nested_majority_asset(nested_offset: f32) -> WeakDom {
    let nested_origin = 20.0 + nested_offset;
    WeakDom::new(
        InstanceBuilder::new("DataModel")
            .with_name("root")
            .with_child(
                InstanceBuilder::new("Model")
                    .with_name("Asset")
                    .with_property(
                        "WorldPivotData",
                        Variant::OptionalCFrame(Some(cframe(0.0, 0.0, 0.0))),
                    )
                    .with_child(part("Outer", 0.0))
                    .with_child(
                        InstanceBuilder::new("Model")
                            .with_name("Nested")
                            .with_property(
                                "WorldPivotData",
                                Variant::OptionalCFrame(Some(cframe(nested_origin, 0.0, 0.0))),
                            )
                            .with_child(part("NestedA", nested_origin))
                            .with_child(part("NestedB", nested_origin + 4.0))
                            .with_child(part("NestedC", nested_origin + 8.0)),
                    ),
            ),
    )
}

fn property<'a>(dom: &'a WeakDom, name: &str, property: &str) -> &'a Variant {
    dom.descendants()
        .find(|instance| instance.name == name)
        .unwrap_or_else(|| panic!("missing {name}"))
        .properties
        .get(&property.into())
        .unwrap_or_else(|| panic!("missing {name}.{property}"))
}

fn x_of(dom: &WeakDom, name: &str) -> f32 {
    match property(dom, name, "CFrame") {
        Variant::CFrame(value) => value.position.x,
        other => panic!("expected CFrame, got {other:?}"),
    }
}

fn pivot_x_of(dom: &WeakDom, name: &str) -> f32 {
    match property(dom, name, "WorldPivotData") {
        Variant::OptionalCFrame(Some(value)) => value.position.x,
        other => panic!("expected WorldPivotData, got {other:?}"),
    }
}

#[test]
fn dominant_frame_normalizes_all_world_content_but_not_attachments() {
    let base = asset(0.0, 0.0, 0.0, None, false);
    // A and B support +100. C has an additional local +5 edit.
    let mut side = asset(100.0, 125.0, 5.0, None, true);

    let normalization = normalize_model_dom_to_base(&base, &mut side).expect("majority frame");
    assert_eq!(normalization.matched_parts, 3);
    assert_eq!(normalization.supporting_parts, 2);
    assert!((normalization.side_delta.position.x - 100.0).abs() < 1e-4);

    assert!((x_of(&side, "A") - 0.0).abs() < 1e-4);
    assert!((x_of(&side, "B") - 4.0).abs() < 1e-4);
    assert!((x_of(&side, "C") - 13.0).abs() < 1e-4);
    assert!((x_of(&side, "D") - 16.0).abs() < 1e-4);

    // Attachment.CFrame is parent-local and must not receive the world frame.
    assert!((x_of(&side, "Socket") - 1.0).abs() < 1e-4);
    match property(&side, "Truck", "WorldPivotData") {
        Variant::OptionalCFrame(Some(value)) => {
            assert!((value.position.x - 25.0).abs() < 1e-4)
        }
        other => panic!("expected WorldPivotData, got {other:?}"),
    }
}

#[test]
fn normalization_removes_global_cframe_conflicts_but_keeps_pivot_edits() {
    let mut base = asset(0.0, 0.0, 0.0, None, false);
    let mut ours = asset(100.0, 130.0, 0.0, Some("ours"), false);
    let mut theirs = asset(-50.0, -80.0, 0.0, Some("theirs"), false);

    normalize_model_dom_to_base(&base, &mut ours).expect("ours frame");
    normalize_model_dom_to_base(&base, &mut theirs).expect("theirs frame");
    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());

    assert_eq!(result.conflicts.len(), 1, "{:?}", result.conflicts);
    assert_eq!(
        result.conflicts[0].kind,
        ConflictKind::Property {
            name: "WorldPivotData".to_string()
        }
    );
    assert_eq!(
        property(&base, "A", "Transparency"),
        &Variant::Float32(0.25)
    );
    assert_eq!(property(&base, "B", "Reflectance"), &Variant::Float32(0.5));
    assert!((x_of(&base, "A") - 0.0).abs() < 1e-4);
    assert!((x_of(&base, "B") - 4.0).abs() < 1e-4);
    assert!((x_of(&base, "C") - 8.0).abs() < 1e-4);
}

#[test]
fn dominant_nested_model_move_is_not_promoted_to_asset_frame() {
    let mut base = nested_majority_asset(0.0);
    let mut ours = nested_majority_asset(100.0);
    let mut theirs = nested_majority_asset(0.0);

    assert!(
        normalize_model_merge_frames(&base, &mut ours, &mut theirs).is_none(),
        "a nested-only move is not an asset-wide frame choice"
    );

    // Declining normalization leaves both branches untouched. Ordinary
    // one-sided merge semantics then apply the nested move exactly.
    assert!((pivot_x_of(&ours, "Asset") - 0.0).abs() < 1e-4);
    assert!((x_of(&ours, "Outer") - 0.0).abs() < 1e-4);
    assert!((pivot_x_of(&ours, "Nested") - 120.0).abs() < 1e-4);

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);

    assert!((pivot_x_of(&base, "Asset") - 0.0).abs() < 1e-4);
    assert!((x_of(&base, "Outer") - 0.0).abs() < 1e-4);
    assert!((pivot_x_of(&base, "Nested") - 120.0).abs() < 1e-4);
    assert!((x_of(&base, "NestedA") - 120.0).abs() < 1e-4);
    assert!((x_of(&base, "NestedB") - 124.0).abs() < 1e-4);
    assert!((x_of(&base, "NestedC") - 128.0).abs() < 1e-4);
}
