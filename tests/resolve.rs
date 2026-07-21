//! In-file conflict state tests: conflicted merge → stamp → (serialize/reload)
//! → mark → finalize → clean file with the chosen content.

use rbx_diff::{
    diff_doms, finalize, find_container, list_entries, mark_entry, merge_doms, stamp_conflicts,
    DiffConfig,
};
use rbx_dom_weak::{InstanceBuilder, WeakDom};
use rbx_types::Variant;

fn folder(name: &str) -> InstanceBuilder {
    InstanceBuilder::new("Folder").with_name(name)
}

fn part_with(name: &str, transparency: f32) -> InstanceBuilder {
    InstanceBuilder::new("Part")
        .with_name(name)
        .with_property("Anchored", Variant::Bool(true))
        .with_property("Transparency", Variant::Float32(transparency))
}

fn base_dom() -> WeakDom {
    WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part_with("P", 0.0)))
            .with_child(folder("B")),
    )
}

/// Merge with a Transparency conflict on root.A.P (ours 0.25, theirs 0.75),
/// stamp the container, and round-trip through the binary format.
fn conflicted_merge() -> WeakDom {
    let mut base = base_dom();
    let ours = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part_with("P", 0.25)))
            .with_child(folder("B")),
    );
    let theirs = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part_with("P", 0.75)))
            .with_child(folder("B")),
    );

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert_eq!(result.conflicts.len(), 1);
    stamp_conflicts(&mut base, &ours, &theirs, &result);

    // Serialize + reload: the conflict state must survive the file format
    let mut buffer = Vec::new();
    rbx_binary::to_writer(&mut buffer, &base, base.root().children()).unwrap();
    rbx_binary::from_reader(buffer.as_slice()).unwrap()
}

fn transparency_of(dom: &WeakDom, part_name: &str) -> f32 {
    let inst = dom
        .descendants()
        .find(|i| i.name == part_name && i.class.as_str() == "Part")
        .expect("part exists");
    match inst.properties.get(&"Transparency".into()) {
        Some(Variant::Float32(v)) => *v,
        other => panic!("unexpected transparency: {other:?}"),
    }
}

#[test]
fn conflict_state_survives_serialization() {
    let dom = conflicted_merge();
    let container = find_container(&dom).expect("container present after reload");
    let entries = list_entries(&dom, container);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, "Property");
    assert_eq!(entries[0].path, "root.A.P");
    assert_eq!(entries[0].property.as_deref(), Some("Transparency"));
    assert!(entries[0].resolved.is_none());

    // Target instance is tagged for GetTagged discovery
    let tagged = dom.descendants().any(|i| {
        matches!(
            i.properties.get(&"Tags".into()),
            Some(Variant::Tags(tags)) if tags.iter().any(|t| t == "RbxDiffConflict")
        ) && i.name == "P"
    });
    assert!(tagged, "conflicted target should carry the RbxDiffConflict tag");
}

#[test]
fn finalize_take_ours() {
    let mut dom = conflicted_merge();
    let container = find_container(&dom).unwrap();
    let entry = list_entries(&dom, container)[0].entry_ref;
    mark_entry(&mut dom, entry, "ours").unwrap();
    finalize(&mut dom).unwrap();

    assert!(find_container(&dom).is_none(), "container stripped");
    assert_eq!(transparency_of(&dom, "P"), 0.25);

    // Fully clean: equal to a hand-built expected DOM (tags stripped too)
    let expected = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part_with("P", 0.25)))
            .with_child(folder("B")),
    );
    let residual = diff_doms(&dom, &expected);
    assert!(residual.is_empty(), "{residual:#?}");
}

#[test]
fn finalize_take_theirs() {
    let mut dom = conflicted_merge();
    let container = find_container(&dom).unwrap();
    let entry = list_entries(&dom, container)[0].entry_ref;
    mark_entry(&mut dom, entry, "theirs").unwrap();
    finalize(&mut dom).unwrap();
    assert_eq!(transparency_of(&dom, "P"), 0.75);
}

#[test]
fn finalize_refuses_unresolved() {
    let mut dom = conflicted_merge();
    let err = finalize(&mut dom).unwrap_err();
    assert!(err.to_string().contains("unresolved"), "{err}");
    assert!(find_container(&dom).is_some(), "container untouched on error");
}

#[test]
fn delete_vs_edit_finalize_both_ways() {
    let build = || {
        let mut base = WeakDom::new(
            folder("root")
                .with_child(
                    folder("A")
                        .with_child(part_with("P", 0.0))
                        .with_child(part_with("Q", 0.0)),
                )
                .with_child(folder("B")),
        );
        // Ours deletes A while theirs independently edits two descendants.
        // This is one subtree-level choice, not one choice per edit operation.
        let ours = WeakDom::new(folder("root").with_child(folder("B")));
        let theirs = WeakDom::new(
            folder("root")
                .with_child(
                    folder("A")
                        .with_child(part_with("P", 0.5))
                        .with_child(part_with("Q", 0.75)),
                )
                .with_child(folder("B")),
        );
        let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
        assert_eq!(result.conflicts.len(), 1, "{:?}", result.conflicts);
        assert_eq!(result.conflicts[0].path, "root.A");
        assert_eq!(result.conflicts[0].ours.len(), 1);
        assert_eq!(result.conflicts[0].theirs.len(), 2);
        stamp_conflicts(&mut base, &ours, &theirs, &result);
        let container = find_container(&base).unwrap();
        assert_eq!(list_entries(&base, container).len(), 1);
        base
    };

    // Take ours: the deletion wins, A disappears
    let mut dom = build();
    let container = find_container(&dom).unwrap();
    for entry in list_entries(&dom, container) {
        mark_entry(&mut dom, entry.entry_ref, "ours").unwrap();
    }
    finalize(&mut dom).unwrap();
    let expected = WeakDom::new(folder("root").with_child(folder("B")));
    let residual = diff_doms(&dom, &expected);
    assert!(residual.is_empty(), "take-ours: {residual:#?}");

    // Take theirs: the edited subtree wins
    let mut dom = build();
    let container = find_container(&dom).unwrap();
    for entry in list_entries(&dom, container) {
        mark_entry(&mut dom, entry.entry_ref, "theirs").unwrap();
    }
    finalize(&mut dom).unwrap();
    let expected = WeakDom::new(
        folder("root")
            .with_child(
                folder("A")
                    .with_child(part_with("P", 0.5))
                    .with_child(part_with("Q", 0.75)),
            )
            .with_child(folder("B")),
    );
    let residual = diff_doms(&dom, &expected);
    assert!(residual.is_empty(), "take-theirs: {residual:#?}");
}

#[test]
fn stamped_file_diffs_cleanly_against_base() {
    // The container is tool metadata: diffing a conflicted file against base
    // must not report it (the conflicted property stays at base value).
    let dom = conflicted_merge();
    let residual = diff_doms(&base_dom(), &dom);
    let structural: Vec<_> = residual
        .iter()
        .filter(|d| !matches!(d, rbx_diff::DiffEntry::Modified { .. }))
        .collect();
    assert!(
        structural.is_empty(),
        "container should be invisible to the differ: {structural:#?}"
    );
}


#[test]
fn custom_resolution_applies_the_supplied_value() {
    // The VS Code-style third option: neither ours (0.25) nor theirs (0.75),
    // but a hand-picked value — coerced to the property's real type
    // (Float32) and stored in the file as the entry's CustomValue attribute.
    let mut dom = conflicted_merge();
    let container = find_container(&dom).unwrap();
    let entry = list_entries(&dom, container)[0].entry_ref;

    rbx_diff::mark_entry_custom(&mut dom, entry, &serde_json::json!(0.5)).unwrap();

    // The custom value must survive the file format like every other bit of
    // conflict state
    let mut buffer = Vec::new();
    rbx_binary::to_writer(&mut buffer, &dom, dom.root().children()).unwrap();
    let mut dom: WeakDom = rbx_binary::from_reader(buffer.as_slice()).unwrap();

    let container = find_container(&dom).unwrap();
    assert_eq!(list_entries(&dom, container)[0].resolved.as_deref(), Some("custom"));

    finalize(&mut dom).unwrap();
    assert_eq!(transparency_of(&dom, "P"), 0.5);
    assert!(find_container(&dom).is_none());
}

#[test]
fn custom_resolution_rejects_wrong_shapes() {
    let mut dom = conflicted_merge();
    let container = find_container(&dom).unwrap();
    let entry = list_entries(&dom, container)[0].entry_ref;

    // Transparency is a number; a string must be rejected with the property named
    let err = rbx_diff::mark_entry_custom(&mut dom, entry, &serde_json::json!("nope")).unwrap_err();
    assert!(err.to_string().contains("Transparency"), "{err}");

    // Unresolved after the failed mark: finalize still refuses
    assert!(finalize(&mut dom).is_err());
}

#[test]
fn custom_resolution_rejects_non_property_conflicts() {
    // delete-vs-edit conflicts have no single property to override
    let mut base = base_dom();
    let ours = WeakDom::new(folder("root").with_child(folder("B")));
    let theirs = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part_with("P", 0.5)))
            .with_child(folder("B")),
    );
    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    stamp_conflicts(&mut base, &ours, &theirs, &result);

    let container = find_container(&base).unwrap();
    let entry = list_entries(&base, container)[0].entry_ref;
    let err = rbx_diff::mark_entry_custom(&mut base, entry, &serde_json::json!(1.0)).unwrap_err();
    assert!(err.to_string().contains("DeleteVsEdit"), "{err}");
}

#[test]
fn custom_resolution_handles_color3uint8_properties() {
    // Part.Color serializes as Color3uint8 — the custom value stores as a
    // Color3 attribute and finalize narrows it back to the property's type.
    let color_part = |r: u8, g: u8, b: u8| {
        InstanceBuilder::new("Part")
            .with_name("P")
            .with_property("Color", Variant::Color3uint8(rbx_types::Color3uint8::new(r, g, b)))
    };
    let mut base = WeakDom::new(folder("root").with_child(color_part(100, 100, 100)));
    let ours = WeakDom::new(folder("root").with_child(color_part(255, 0, 0)));
    let theirs = WeakDom::new(folder("root").with_child(color_part(0, 0, 255)));
    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert_eq!(result.conflicts.len(), 1, "{:?}", result.conflicts);
    stamp_conflicts(&mut base, &ours, &theirs, &result);

    let container = find_container(&base).unwrap();
    let entry = list_entries(&base, container)[0].entry_ref;
    rbx_diff::mark_entry_custom(&mut base, entry, &serde_json::json!([0.2353, 0.4706, 1.0]))
        .unwrap();

    // Survive the file format (Color3 attribute round-trip)
    let mut buffer = Vec::new();
    rbx_binary::to_writer(&mut buffer, &base, base.root().children()).unwrap();
    let mut dom: WeakDom = rbx_binary::from_reader(buffer.as_slice()).unwrap();

    finalize(&mut dom).unwrap();
    let part = dom
        .descendants()
        .find(|i| i.name == "P")
        .unwrap();
    match part.properties.get(&"Color".into()) {
        Some(Variant::Color3uint8(c)) => {
            assert_eq!((c.r, c.g, c.b), (60, 120, 255));
        }
        other => panic!("expected Color3uint8, got {other:?}"),
    }
}
