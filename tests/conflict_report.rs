//! The machine-readable conflict report (`resolve --list --json`,
//! `merge --json`): everything an automated resolver needs is read back from
//! the stamped file, and it tracks resolution state as entries are marked.

use git_rbx::{
    conflict_report, find_container, list_entries, mark_entry, mark_entry_custom, merge_doms,
    stamp_conflicts, ConflictReport, DiffConfig, SCHEMA_VERSION,
};
use rbx_dom_weak::{InstanceBuilder, WeakDom};
use rbx_types::Variant;
use serde_json::{json, Value};

fn folder(name: &str) -> InstanceBuilder {
    InstanceBuilder::new("Folder").with_name(name)
}

fn part_with(name: &str, transparency: f32) -> InstanceBuilder {
    InstanceBuilder::new("Part")
        .with_name(name)
        .with_property("Anchored", Variant::Bool(true))
        .with_property("Transparency", Variant::Float32(transparency))
}

/// base: root { A { P(0.0) }, Q(0.0) }
/// ours: edits Q → 0.1, edits A.P → 0.5
/// theirs: edits Q → 0.2, deletes A
/// → Conflict on Q.Transparency (Property) and on A (DeleteVsEdit).
fn stamped_conflicted_file() -> WeakDom {
    let mut base = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part_with("P", 0.0)))
            .with_child(part_with("Q", 0.0)),
    );
    let ours = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part_with("P", 0.5)))
            .with_child(part_with("Q", 0.1)),
    );
    let theirs = WeakDom::new(folder("root").with_child(part_with("Q", 0.2)));
    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert_eq!(result.conflicts.len(), 2, "{:#?}", result.conflicts);
    stamp_conflicts(&mut base, &ours, &theirs, &result);

    // Round-trip through the binary format: the report must work on what a
    // resolver actually reads from disk, not on in-memory leftovers.
    let mut bytes = Vec::new();
    rbx_binary::to_writer(&mut bytes, &base, base.root().children()).unwrap();
    rbx_binary::from_reader(bytes.as_slice()).unwrap()
}

fn report_json(dom: &WeakDom) -> (ConflictReport, Value) {
    let container = find_container(dom).expect("stamped container");
    let report = conflict_report(dom, container);
    // Through text, exactly as the CLI emits it (f32s print shortest-repr,
    // not widened to f64 the way `to_value` would).
    let value = serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();
    (report, value)
}

/// Property values use the same typed shape as `diff --json`.
fn f32_value(value: f64) -> Value {
    json!({"type": "float32", "value": {"value": value}})
}

fn entry<'a>(value: &'a Value, name: &str) -> &'a Value {
    value["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap_or_else(|| panic!("no entry {name} in {value:#}"))
}

#[test]
fn report_describes_every_conflict_with_competing_patches() {
    let dom = stamped_conflicted_file();
    let (report, value) = report_json(&dom);

    assert_eq!(report.schema_version, SCHEMA_VERSION);
    assert_eq!(report.conflict_count, 2);
    assert_eq!(report.unresolved_count, 2);
    assert!(report.groups.is_empty());

    // Entries are sorted by path: A (delete-vs-edit) before Q (property).
    let names: Vec<_> = value["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, vec!["Conflict_1", "Conflict_2"]);

    let delete = entry(&value, "Conflict_1");
    assert_eq!(delete["kind"], "DeleteVsEdit");
    assert_eq!(delete["path"], "root.A");
    assert_eq!(delete["ours"]["deleted"], false);
    assert_eq!(delete["theirs"]["deleted"], true);
    assert!(delete.get("resolved").is_none(), "unresolved entries omit `resolved`");
    // Ours' side of the decision is the edit inside A; the impact says so.
    let our_ops = delete["ours"]["impact"]["operations"].as_array().unwrap();
    assert!(
        our_ops
            .iter()
            .any(|op| op["property"] == "Transparency" && op["after"] == f32_value(0.5)),
        "{our_ops:#?}"
    );

    let property = entry(&value, "Conflict_2");
    assert_eq!(property["kind"], "Property");
    assert_eq!(property["path"], "root.Q");
    assert_eq!(property["property"], "Transparency");
    // Both sides' exact patches: before is the base value, after is each
    // branch's value — this is what lets an agent decide without a GUI.
    let ours_op = &property["ours"]["impact"]["operations"][0];
    let theirs_op = &property["theirs"]["impact"]["operations"][0];
    assert_eq!(ours_op["before"], f32_value(0.0));
    assert_eq!(ours_op["after"], f32_value(0.1));
    assert_eq!(theirs_op["before"], f32_value(0.0));
    assert_eq!(theirs_op["after"], f32_value(0.2));
}

#[test]
fn report_tracks_resolution_state() {
    let mut dom = stamped_conflicted_file();
    let container = find_container(&dom).unwrap();
    let entries = list_entries(&dom, container);
    let property_entry = entries.iter().find(|e| e.kind == "Property").unwrap();
    let delete_entry = entries.iter().find(|e| e.kind == "DeleteVsEdit").unwrap();

    mark_entry(&mut dom, delete_entry.entry_ref, "theirs").unwrap();
    let (report, value) = report_json(&dom);
    assert_eq!(report.unresolved_count, 1);
    assert_eq!(entry(&value, "Conflict_1")["resolved"], "theirs");
    assert!(entry(&value, "Conflict_2").get("resolved").is_none());

    mark_entry_custom(&mut dom, property_entry.entry_ref, &json!(0.75)).unwrap();
    let (report, value) = report_json(&dom);
    assert_eq!(report.unresolved_count, 0);
    let custom = entry(&value, "Conflict_2");
    assert_eq!(custom["resolved"], "custom");
    assert_eq!(custom["customValue"], f32_value(0.75), "{custom:#}");
}

#[test]
fn empty_report_matches_a_clean_merge() {
    let report = ConflictReport::empty();
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(value["conflictCount"], 0);
    assert_eq!(value["unresolvedCount"], 0);
    assert_eq!(value["conflicts"], json!([]));
    assert_eq!(value["schemaVersion"], SCHEMA_VERSION);
}
