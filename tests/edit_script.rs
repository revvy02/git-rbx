//! Edit-script round-trip tests: applying compute_edit_script(old, new) to
//! `old` must produce a DOM with an empty diff against `new`.

use rbx_diff::{apply_edit_script, compute_edit_script, diff_doms, DiffConfig, EditOp};
use rbx_dom_weak::{InstanceBuilder, WeakDom};
use rbx_types::Variant;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

fn folder(name: &str) -> InstanceBuilder {
    InstanceBuilder::new("Folder").with_name(name)
}

fn part(name: &str) -> InstanceBuilder {
    InstanceBuilder::new("Part")
        .with_name(name)
        .with_property("Anchored", Variant::Bool(true))
        .with_property("Transparency", Variant::Float32(0.0))
}

/// Round-trip: compute script old→new, apply to old, assert diff is empty.
fn assert_round_trip(mut old: WeakDom, new: WeakDom) {
    let config = DiffConfig::default();
    let script = compute_edit_script(&old, &new, &config);
    apply_edit_script(&mut old, &new, &script);
    let residual = diff_doms(&old, &new);
    assert!(
        residual.is_empty(),
        "apply(old, script) should equal new; residual diff: {residual:#?}"
    );
}

#[test]
fn round_trip_add_remove_and_edit() {
    let old = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part("Keep")).with_child(part("Delete")))
            .with_child(folder("B")),
    );
    let new = WeakDom::new(
        folder("root")
            .with_child(
                folder("A").with_child(
                    InstanceBuilder::new("Part")
                        .with_name("Keep")
                        .with_property("Anchored", Variant::Bool(false))
                        .with_property("Transparency", Variant::Float32(0.5)),
                ),
            )
            .with_child(
                folder("B").with_child(folder("NewTree").with_child(part("Inner"))),
            ),
    );
    assert_round_trip(old, new);
}

#[test]
fn round_trip_rename() {
    let old = WeakDom::new(folder("root").with_child(folder("A").with_child(part("OldName"))));
    let new = WeakDom::new(folder("root").with_child(folder("A").with_child(part("NewName"))));
    assert_round_trip(old, new);
}

#[test]
fn round_trip_move_with_edit() {
    let old = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part("P")))
            .with_child(folder("B")),
    );
    let new = WeakDom::new(
        folder("root")
            .with_child(folder("A"))
            .with_child(
                folder("B").with_child(
                    InstanceBuilder::new("Part")
                        .with_name("P")
                        .with_property("Anchored", Variant::Bool(true))
                        .with_property("Transparency", Variant::Float32(0.75)),
                ),
            ),
    );
    assert_round_trip(old, new);
}

#[test]
fn round_trip_move_into_added_subtree() {
    // P moves into a folder that itself is newly added — exercises the
    // Anchor::Added path in apply.
    let old = WeakDom::new(
        folder("root").with_child(folder("A").with_child(part("P"))),
    );
    let new = WeakDom::new(
        folder("root")
            .with_child(folder("A"))
            .with_child(folder("NewParent").with_child(part("P"))),
    );
    assert_round_trip(old, new);
}

#[test]
fn round_trip_parent_swap() {
    // A was parent of B; now B is parent of A — stresses move ordering
    // (transfer into an unsettled ancestor would panic or corrupt).
    let old = WeakDom::new(
        folder("root").with_child(
            folder("A")
                .with_property("Anchored", Variant::Bool(true))
                .with_child(folder("B")),
        ),
    );
    let new = WeakDom::new(
        folder("root").with_child(
            folder("B").with_child(
                folder("A").with_property("Anchored", Variant::Bool(true)),
            ),
        ),
    );
    assert_round_trip(old, new);
}

#[test]
fn round_trip_ref_retarget() {
    // Model.PrimaryPart retargets from one same-named sibling to another;
    // the applied value must be remapped through the identity mapping.
    let old = WeakDom::new(folder("root").with_child({
        let p1 = part("P").with_property("Transparency", Variant::Float32(0.1));
        let p2 = part("P").with_property("Transparency", Variant::Float32(0.2));
        let p1_ref = p1.referent();
        InstanceBuilder::new("Model")
            .with_name("M")
            .with_property("PrimaryPart", Variant::Ref(p1_ref))
            .with_child(p1)
            .with_child(p2)
    }));
    let new = WeakDom::new(folder("root").with_child({
        let p1 = part("P").with_property("Transparency", Variant::Float32(0.1));
        let p2 = part("P").with_property("Transparency", Variant::Float32(0.2));
        let p2_ref = p2.referent();
        InstanceBuilder::new("Model")
            .with_name("M")
            .with_property("PrimaryPart", Variant::Ref(p2_ref))
            .with_child(p1)
            .with_child(p2)
    }));
    assert_round_trip(old, new);
}

#[test]
fn round_trip_ref_into_added_subtree() {
    // An existing instance gains a Ref property pointing INTO a newly added
    // subtree — apply must remap through the created-instances table.
    let old = WeakDom::new(folder("root").with_child(
        InstanceBuilder::new("Model").with_name("M"),
    ));
    let new = WeakDom::new(folder("root").with_child({
        let target = part("Target");
        let target_ref = target.referent();
        InstanceBuilder::new("Model")
            .with_name("M")
            .with_property("PrimaryPart", Variant::Ref(target_ref))
            .with_child(target)
    }));
    assert_round_trip(old, new);
}

#[test]
fn script_uses_move_op_for_pure_move() {
    let old = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part("P")))
            .with_child(folder("B")),
    );
    let new = WeakDom::new(
        folder("root")
            .with_child(folder("A"))
            .with_child(folder("B").with_child(part("P"))),
    );
    let script = compute_edit_script(&old, &new, &DiffConfig::default());
    let moves = script.ops.iter().filter(|op| matches!(op, EditOp::Move { .. })).count();
    let structural = script
        .ops
        .iter()
        .filter(|op| matches!(op, EditOp::AddSubtree { .. } | EditOp::RemoveSubtree { .. }))
        .count();
    assert_eq!(moves, 1, "pure move should be a Move op: {:?}", script.ops);
    assert_eq!(structural, 0, "no add/remove for a pure move: {:?}", script.ops);
}

// ============================================================================
// Real fixture round-trips
// ============================================================================

fn load(path: &str) -> Option<WeakDom> {
    if !Path::new(path).exists() {
        eprintln!("SKIP: fixture {path} not present");
        return None;
    }
    let file = BufReader::new(File::open(path).unwrap());
    Some(rbx_binary::from_reader(file).unwrap())
}

fn assert_file_round_trip(old_path: &str, new_path: &str) {
    let (Some(old), Some(new)) = (load(old_path), load(new_path)) else {
        return;
    };
    assert_round_trip(old, new);
}

#[test]
fn round_trip_union_fixture() {
    assert_file_round_trip(
        "tests-new/union-operation/separated-parts.rbxm",
        "tests-new/union-operation/unioned-parts.rbxm",
    );
}

#[test]
fn round_trip_union_geometry_fixture() {
    assert_file_round_trip(
        "tests-new/union-operation/unioned-parts.rbxm",
        "tests-new/union-operation/unioned-parts-in-same-spot-but-diff-geometry.rbxm",
    );
}

#[test]
fn round_trip_primary_part_fixture() {
    assert_file_round_trip(
        "tests-new/referential-properties/primary-part/model-with-grey-primary-part-and-has-dupe-children-names.rbxm",
        "tests-new/referential-properties/primary-part/model-with-yellow-primary-part-and-has-dupe-children-names.rbxm",
    );
}

#[test]
#[ignore = "46MB fixtures; run with cargo test --release -- --ignored"]
fn round_trip_full_place() {
    assert_file_round_trip(
        "tests-new/fixtures/rc_manually_saved_build.rbxl",
        "tests-new/models-moved/rc_build_saved_manually_with_1_tree_moved.rbxl",
    );
}

#[test]
fn round_trip_group_dupes_into_new_container() {
    // Multiple same-named instances gathered under a new container: the
    // clone of the added container must not duplicate the moved-in content.
    let dup = |t: f32| {
        InstanceBuilder::new("Part")
            .with_name("P")
            .with_property("Anchored", Variant::Bool(true))
            .with_property("Transparency", Variant::Float32(t))
    };
    let old = WeakDom::new(
        folder("root").with_child(folder("A").with_child(dup(0.1)).with_child(dup(0.2))),
    );
    let new = WeakDom::new(
        folder("root").with_child(
            folder("A").with_child(folder("Group").with_child(dup(0.1)).with_child(dup(0.2))),
        ),
    );
    let script = compute_edit_script(&old, &new, &DiffConfig::default());
    let move_ops = script.ops.iter().filter(|op| matches!(op, EditOp::Move { .. })).count();
    let removes = script.ops.iter().filter(|op| matches!(op, EditOp::RemoveSubtree { .. })).count();
    assert_eq!(move_ops, 2, "{:?}", script.ops);
    assert_eq!(removes, 0, "{:?}", script.ops);

    assert_round_trip(old, new);
}

#[test]
fn round_trip_move_out_of_removed_folder() {
    let old = WeakDom::new(
        folder("root")
            .with_child(folder("Doomed").with_child(part("Keep")).with_child(folder("Junk")))
            .with_child(folder("B")),
    );
    let new = WeakDom::new(
        folder("root").with_child(folder("B").with_child(part("Keep"))),
    );
    assert_round_trip(old, new);
}

#[test]
fn round_trip_obj_value_cross_refs_between_identical_twins() {
    // Two identical "Uniform Giver" twins where each holds an ObjectValue
    // pointing at the OTHER twin's ClickPart. After applying the edit script,
    // the topology the fixture README specifies must hold with the right
    // polarity — a symmetric swap would slip past a diff-empty check alone.
    let d = "tests-new/referential-properties/obj-value";
    let old_path = format!("{d}/police-station-with-2-identical-uni-givers-with-primary-part.rbxm");
    let new_path = format!("{d}/police-station-with-the-uni-primary-parts-but-with-obj-value-that-references-the-other-uni-giver.rbxm");
    let (Some(mut old), Some(new)) = (load(&old_path), load(&new_path)) else {
        return;
    };

    let script = compute_edit_script(&old, &new, &DiffConfig::default());
    apply_edit_script(&mut old, &new, &script);
    let residual = diff_doms(&old, &new);
    assert!(residual.is_empty(), "{residual:#?}");

    let applied = &old;
    let givers: Vec<_> = applied
        .descendants()
        .filter(|i| i.name == "Uniform Giver" && i.class.as_str() == "Model")
        .collect();
    assert_eq!(givers.len(), 2);

    let primary_of = |giver: &rbx_dom_weak::Instance| -> rbx_dom_weak::types::Ref {
        match giver.properties.get(&"PrimaryPart".into()) {
            Some(Variant::Ref(r)) => *r,
            other => panic!("PrimaryPart missing: {other:?}"),
        }
    };
    let is_inside = |node: rbx_dom_weak::types::Ref, root: rbx_dom_weak::types::Ref| {
        let mut current = node;
        while let Some(inst) = applied.get_by_ref(current) {
            if current == root {
                return true;
            }
            current = inst.parent();
        }
        false
    };

    for (index, giver) in givers.iter().enumerate() {
        let own_primary = primary_of(giver);
        let primary_inst = applied.get_by_ref(own_primary).expect("primary exists");
        assert_eq!(primary_inst.name, "ClickPart");
        assert!(
            is_inside(own_primary, giver.referent()),
            "giver {index}: PrimaryPart must be its own descendant"
        );

        let value = applied
            .get_by_ref(giver.referent())
            .unwrap()
            .children()
            .iter()
            .find_map(|&c| {
                applied
                    .get_by_ref(c)
                    .filter(|i| i.class.as_str() == "ObjectValue" && i.name == "Value")
            })
            .expect("ObjectValue child");
        let pointee = match value.properties.get(&"Value".into()) {
            Some(Variant::Ref(r)) => *r,
            other => panic!("ObjectValue.Value missing: {other:?}"),
        };

        let other_giver = &givers[1 - index];
        assert_eq!(
            pointee,
            primary_of(other_giver),
            "giver {index}: ObjectValue must point at the OTHER twin's PrimaryPart"
        );
        assert!(
            !is_inside(pointee, giver.referent()),
            "giver {index}: cross-ref must not point inside itself"
        );
    }
}
