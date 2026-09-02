use git_rbx::{
    apply_pivot_ops, diff_doms, diff_model_compact_doms_with_config,
    diff_model_compact_old_with_config, diff_model_doms_with_config, finalize, find_container,
    list_entries, mark_entry, merge_compact_doms_with_matches_and_pivots, merge_doms,
    normalize_model_dom_to_base, normalize_model_merge_compact_pivots,
    normalize_model_merge_pivots, stamp_compact_conflicts, ConflictKind, DiffConfig, DiffDom,
    DiffEntry,
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

fn large_asset(offset: f32, part_count: usize) -> WeakDom {
    let mut model = InstanceBuilder::new("Model")
        .with_name("LargeAsset")
        .with_property(
            "WorldPivotData",
            Variant::OptionalCFrame(Some(cframe(offset, 0.0, 0.0))),
        );
    for index in 0..part_count {
        model = model.with_child(part(&format!("Part{index}"), offset + index as f32));
    }
    WeakDom::new(
        InstanceBuilder::new("DataModel")
            .with_name("root")
            .with_child(model),
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
fn pivot_free_merge_preparation_does_not_rewrite_either_branch() {
    let base = asset(0.0, 0.0, 0.0, None, false);
    let mut ours = asset(0.0, 0.0, 0.0, Some("ours"), false);
    let mut theirs = asset(0.0, 0.0, 0.0, Some("theirs"), false);
    let ours_before = asset(0.0, 0.0, 0.0, Some("ours"), false);
    let theirs_before = asset(0.0, 0.0, 0.0, Some("theirs"), false);

    assert!(normalize_model_merge_pivots(&base, &mut ours, &mut theirs).is_none());
    assert!(diff_doms(&ours_before, &ours).is_empty());
    assert!(diff_doms(&theirs_before, &theirs).is_empty());
}

#[test]
fn core_merge_planner_owns_primitive_pivot_decisions() {
    let mut base = asset(0.0, 0.0, 0.0, None, false);
    let mut ours = DiffDom::from_weak_dom_owned(asset(100.0, 100.0, 0.0, None, false));
    let mut theirs = DiffDom::from_weak_dom_owned(asset(-50.0, -50.0, 0.0, None, false));
    let preparation = normalize_model_merge_compact_pivots(&base, &mut ours, &mut theirs).unwrap();
    let ours_pivots = preparation.ours_pivots();
    let theirs_pivots = preparation.theirs_pivots();

    assert_eq!(ours_pivots.len(), 1);
    assert_eq!(theirs_pivots.len(), 1);
    let result = merge_compact_doms_with_matches_and_pivots(
        &mut base,
        &ours,
        &theirs,
        &DiffConfig::default(),
        &preparation.ours_identity,
        &preparation.theirs_identity,
        ours_pivots,
        theirs_pivots,
    );

    assert!(result.pivots.is_empty());
    assert_eq!(result.conflicts.len(), 1);
    assert!(matches!(
        result.conflicts[0].kind,
        ConflictKind::Pivot { .. }
    ));
    stamp_compact_conflicts(&mut base, &ours, &theirs, &result);
    let entries = list_entries(&base, find_container(&base).unwrap());
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, "Pivot");
    assert_eq!(entries[0].pivot_order, Some(0));
}

#[test]
fn core_merge_planner_materializes_one_sided_pivot_op() {
    let mut base = asset(0.0, 0.0, 0.0, None, false);
    let expected = asset(100.0, 100.0, 0.0, None, false);
    let mut ours = DiffDom::from_weak_dom_owned(asset(100.0, 100.0, 0.0, None, false));
    let mut theirs = DiffDom::from_weak_dom_owned(asset(0.0, 0.0, 0.0, None, false));
    let preparation = normalize_model_merge_compact_pivots(&base, &mut ours, &mut theirs).unwrap();
    let result = merge_compact_doms_with_matches_and_pivots(
        &mut base,
        &ours,
        &theirs,
        &DiffConfig::default(),
        &preparation.ours_identity,
        &preparation.theirs_identity,
        preparation.ours_pivots(),
        preparation.theirs_pivots(),
    );

    assert!(result.conflicts.is_empty());
    assert_eq!(result.pivots.len(), 1);
    apply_pivot_ops(&mut base, &result.pivots);
    assert!(diff_doms(&base, &expected).is_empty());
}

#[test]
fn delete_vs_pivot_is_one_reconstructable_structural_conflict() {
    let build_conflicted = || {
        let mut base = asset(0.0, 0.0, 0.0, None, false);
        let mut ours = DiffDom::from_weak_dom_owned(asset(100.0, 100.0, 0.0, None, false));
        let mut theirs = DiffDom::from_weak_dom_owned(WeakDom::new(
            InstanceBuilder::new("DataModel").with_name("root"),
        ));
        let preparation =
            normalize_model_merge_compact_pivots(&base, &mut ours, &mut theirs).unwrap();
        let result = merge_compact_doms_with_matches_and_pivots(
            &mut base,
            &ours,
            &theirs,
            &DiffConfig::default(),
            &preparation.ours_identity,
            &preparation.theirs_identity,
            preparation.ours_pivots(),
            preparation.theirs_pivots(),
        );
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].kind, ConflictKind::DeleteVsEdit);
        assert_eq!(result.conflicts[0].ours.pivots.len(), 1);
        stamp_compact_conflicts(&mut base, &ours, &theirs, &result);
        let mut bytes = Vec::new();
        rbx_binary::to_writer(&mut bytes, &base, base.root().children()).unwrap();
        rbx_binary::from_reader(bytes.as_slice()).unwrap()
    };

    for side in ["ours", "theirs"] {
        let mut merged = build_conflicted();
        let container = find_container(&merged).unwrap();
        let entries = list_entries(&merged, container);
        assert_eq!(entries.len(), 1);
        mark_entry(&mut merged, entries[0].entry_ref, side).unwrap();
        finalize(&mut merged).unwrap();

        let expected = if side == "ours" {
            asset(100.0, 100.0, 0.0, None, false)
        } else {
            WeakDom::new(InstanceBuilder::new("DataModel").with_name("root"))
        };
        assert!(diff_doms(&merged, &expected).is_empty(), "{side}");
    }
}

#[test]
fn nested_model_move_becomes_its_own_local_pivot() {
    let mut base = nested_majority_asset(0.0);
    let mut ours = nested_majority_asset(100.0);
    let mut theirs = nested_majority_asset(0.0);

    let frames = normalize_model_merge_pivots(&base, &mut ours, &mut theirs)
        .expect("nested model establishes a local frame");
    assert_eq!(frames.affected_boundaries(), 1);
    assert_eq!(
        base.get_by_ref(frames.ours_pivots()[0].target_ref)
            .unwrap()
            .name,
        "Nested"
    );
    assert_eq!(frames.ours_pivots().len(), 1);
    assert!(frames.theirs_pivots().is_empty());

    // The branch is canonical while ordinary merge semantics run.
    assert!((pivot_x_of(&ours, "Asset") - 0.0).abs() < 1e-4);
    assert!((x_of(&ours, "Outer") - 0.0).abs() < 1e-4);
    assert!((pivot_x_of(&ours, "Nested") - 20.0).abs() < 1e-4);

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    apply_pivot_ops(&mut base, frames.ours_pivots());

    // Materializing the local plan restores the authored nested movement
    // without moving the parent boundary.
    assert!((pivot_x_of(&base, "Asset") - 0.0).abs() < 1e-4);
    assert!((x_of(&base, "Outer") - 0.0).abs() < 1e-4);
    assert!((pivot_x_of(&base, "Nested") - 120.0).abs() < 1e-4);
    assert!((x_of(&base, "NestedA") - 120.0).abs() < 1e-4);
    assert!((x_of(&base, "NestedB") - 124.0).abs() < 1e-4);
    assert!((x_of(&base, "NestedC") - 128.0).abs() < 1e-4);
}

#[test]
fn two_way_diff_collapses_nested_movement_into_one_pivoted_entry() {
    let base = nested_majority_asset(0.0);
    let mut side = nested_majority_asset(100.0);

    let (diffs, normalization) =
        diff_model_doms_with_config(&base, &mut side, &DiffConfig::default());
    let normalization = normalization.expect("nested frame should be detected");
    assert_eq!(normalization.pivots.len(), 1);
    assert_eq!(diffs.len(), 1, "{diffs:#?}");
    match &diffs[0] {
        DiffEntry::Pivoted {
            path, delta, class, ..
        } => {
            assert!(path.ends_with("Asset.Nested"), "{path}");
            assert_eq!(class, "Model");
            assert!((delta.components[0] - 100.0).abs() < 1e-4, "{delta:?}");
        }
        other => panic!("expected one pivot, got {other:#?}"),
    }
}

#[test]
fn compact_old_side_preserves_hierarchical_diff_semantics() {
    let base = nested_majority_asset(0.0);
    let compact_base = DiffDom::from_weak_dom(&base);
    let mut weak_side = nested_majority_asset(100.0);
    let mut compact_side = nested_majority_asset(100.0);
    let mut all_compact_side = DiffDom::from_weak_dom_owned(nested_majority_asset(100.0));

    let (weak_diffs, weak_frames) =
        diff_model_doms_with_config(&base, &mut weak_side, &DiffConfig::default());
    let (compact_diffs, compact_frames) = diff_model_compact_old_with_config(
        &compact_base,
        &mut compact_side,
        &DiffConfig::default(),
    );
    let (all_compact_diffs, all_compact_frames) = diff_model_compact_doms_with_config(
        &compact_base,
        &mut all_compact_side,
        &DiffConfig::default(),
    );

    let without_source_refs = |diffs: &[DiffEntry]| {
        let mut value = serde_json::to_value(diffs).unwrap();
        for entry in value.as_array_mut().unwrap() {
            let entry = entry.as_object_mut().unwrap();
            entry.remove("old_ref");
            entry.remove("new_ref");
        }
        value
    };
    assert_eq!(
        without_source_refs(&compact_diffs),
        without_source_refs(&weak_diffs)
    );
    assert_eq!(
        without_source_refs(&all_compact_diffs),
        without_source_refs(&weak_diffs)
    );
    assert_eq!(
        compact_frames.as_ref().map(|frames| frames.pivots.len()),
        weak_frames.as_ref().map(|frames| frames.pivots.len())
    );
    assert_eq!(
        all_compact_frames
            .as_ref()
            .map(|frames| frames.pivots.len()),
        weak_frames.as_ref().map(|frames| frames.pivots.len())
    );
    assert!(diff_doms(&weak_side, &compact_side).is_empty());
}

#[test]
fn two_way_diff_keeps_world_pivot_edit_separate_from_content_pivot() {
    let base = asset(0.0, 0.0, 0.0, None, false);
    // All content moves +100, while the pivot moves +125.
    let mut side = asset(100.0, 125.0, 0.0, None, false);

    let (diffs, _) = diff_model_doms_with_config(&base, &mut side, &DiffConfig::default());
    assert_eq!(diffs.len(), 2, "{diffs:#?}");
    assert!(matches!(&diffs[0], DiffEntry::Pivoted { delta, .. }
        if (delta.components[0] - 100.0).abs() < 1e-4));
    assert!(diffs.iter().any(|diff| matches!(
        diff,
        DiffEntry::Modified { property_changes, .. }
            if property_changes.len() == 1
                && property_changes[0].name == "WorldPivotData"
    )));
}

#[test]
fn two_way_diff_collapses_a_thousand_cframes_into_one_pivoted_entry() {
    let base = large_asset(0.0, 1_000);
    let mut side = large_asset(250.0, 1_000);

    let (diffs, normalization) =
        diff_model_doms_with_config(&base, &mut side, &DiffConfig::default());
    assert_eq!(normalization.unwrap().pivots.len(), 1);
    assert_eq!(diffs.len(), 1, "{diffs:#?}");
    assert!(matches!(diffs[0], DiffEntry::Pivoted { .. }));
}

#[test]
fn studio_rotation_normalization_does_not_invent_large_world_frame() {
    let fresh = Matrix3::new(
        Vector3::new(1.000009, -0.0000000093191375, 0.0002928916),
        Vector3::new(-0.000000011180918, 1.0000004, -0.000000000004998568),
        Vector3::new(-0.0002936968, -0.00000000000208967, 1.0000066),
    );
    let studio = Matrix3::new(
        Vector3::new(1.0, 0.000000011180819, 0.00029369417),
        Vector3::new(-0.000000011180818, 1.0, 0.000000000004826645),
        Vector3::new(-0.00029369417, -0.0000000000081103865, 1.0),
    );
    let asset = |orientation: Matrix3| {
        let positioned = |name: &str, x: f32| {
            InstanceBuilder::new("Part").with_name(name).with_property(
                "CFrame",
                Variant::CFrame(CFrame::new(
                    Vector3::new(x, 7.4760227, 189.35971),
                    orientation,
                )),
            )
        };
        WeakDom::new(
            InstanceBuilder::new("DataModel").with_child(
                InstanceBuilder::new("Model")
                    .with_name("FarFromOrigin")
                    .with_child(positioned("A", -168.445))
                    .with_child(positioned("B", -208.445)),
            ),
        )
    };
    let base = asset(fresh);
    let mut side = asset(studio);

    let (diffs, normalization) =
        diff_model_doms_with_config(&base, &mut side, &DiffConfig::default());
    assert!(
        normalization.is_none(),
        "representation noise is not a frame"
    );
    assert!(diffs.is_empty(), "{diffs:#?}");
}

#[test]
fn translated_far_model_ignores_studio_rotation_normalization_residue() {
    let fresh = Matrix3::new(
        Vector3::new(1.000009, -0.0000000093191375, 0.0002928916),
        Vector3::new(-0.000000011180918, 1.0000004, -0.000000000004998568),
        Vector3::new(-0.0002936968, -0.00000000000208967, 1.0000066),
    );
    let studio = Matrix3::new(
        Vector3::new(1.0, 0.000000011180819, 0.00029369417),
        Vector3::new(-0.000000011180818, 1.0, 0.000000000004826645),
        Vector3::new(-0.00029369417, -0.0000000000081103865, 1.0),
    );
    let asset = |orientation: Matrix3, translation: f32| {
        let positioned = |name: &str, x: f32| {
            InstanceBuilder::new("Part").with_name(name).with_property(
                "CFrame",
                Variant::CFrame(CFrame::new(
                    Vector3::new(x + translation, -108.0, 7_050.0),
                    orientation,
                )),
            )
        };
        WeakDom::new(
            InstanceBuilder::new("DataModel").with_child(
                InstanceBuilder::new("Model")
                    .with_name("FarFromOrigin")
                    .with_property(
                        "WorldPivotData",
                        Variant::OptionalCFrame(Some(CFrame::new(
                            Vector3::new(-500.0 + translation, -108.0, 7_050.0),
                            orientation,
                        ))),
                    )
                    .with_child(positioned("A", -490.0))
                    .with_child(positioned("B", -510.0)),
            ),
        )
    };
    let base = asset(fresh, 0.0);
    let mut side = asset(studio, -1.0);

    let (diffs, normalization) =
        diff_model_doms_with_config(&base, &mut side, &DiffConfig::default());
    let normalization = normalization.expect("the translation should establish a frame");
    assert_eq!(normalization.pivots.len(), 1, "{:#?}", normalization.pivots);
    assert_eq!(diffs.len(), 1, "{diffs:#?}");
    assert!(matches!(
        &diffs[0],
        DiffEntry::Pivoted { delta, .. }
            if (delta.components[0] + 1.0).abs() < 1e-4
                && delta.components[1].abs() < 1e-4
                && delta.components[2].abs() < 1e-4
    ));
}

#[test]
fn frame_consensus_averages_f32_translation_quantization() {
    let asset = |a_shift: f32, b_shift: f32, pivot_shift: f32| {
        WeakDom::new(
            InstanceBuilder::new("DataModel").with_child(
                InstanceBuilder::new("Model")
                    .with_name("FarFromOrigin")
                    .with_property(
                        "WorldPivotData",
                        Variant::OptionalCFrame(Some(cframe(
                            -500.0 + pivot_shift,
                            -108.0,
                            7_050.0,
                        ))),
                    )
                    .with_child(InstanceBuilder::new("Part").with_name("A").with_property(
                        "CFrame",
                        Variant::CFrame(cframe(-490.0 + a_shift, -108.0, 7_050.0)),
                    ))
                    .with_child(InstanceBuilder::new("Part").with_name("B").with_property(
                        "CFrame",
                        Variant::CFrame(cframe(-510.0 + b_shift, -108.0, 7_050.0)),
                    )),
            ),
        )
    };
    let base = asset(0.0, 0.0, 0.0);
    // At large world coordinates the same authored translation can round to
    // slightly different f32 deltas on different parts.
    let mut side = asset(-0.99945, -1.00055, -1.0);

    let (diffs, normalization) =
        diff_model_doms_with_config(&base, &mut side, &DiffConfig::default());
    assert_eq!(normalization.unwrap().pivots.len(), 1);
    assert_eq!(diffs.len(), 1, "{diffs:#?}");
    assert!(matches!(
        &diffs[0],
        DiffEntry::Pivoted { delta, .. } if (delta.components[0] + 1.0).abs() < 1e-4
    ));
}
