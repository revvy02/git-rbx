//! Granular Attributes/Tags semantics: container properties diff, merge, and
//! finalize per key (`Attributes.<key>` / `Tags.<tag>`), so branches touching
//! different keys compose instead of conflicting.

use rbx_diff::{
    apply_edit_script, compute_edit_script, diff_doms, finalize, find_container, list_entries,
    mark_entry, mark_entry_custom, merge_doms, stamp_conflicts, ConflictKind, DiffConfig,
    DiffEntry, PropertyValue,
};
use rbx_dom_weak::{InstanceBuilder, WeakDom};
use rbx_types::{Attributes, BinaryString, Tags, Variant};

fn folder(name: &str) -> InstanceBuilder {
    InstanceBuilder::new("Folder").with_name(name)
}

fn attrs(pairs: &[(&str, f64)]) -> Variant {
    let mut a = Attributes::new();
    for (k, v) in pairs {
        a.insert(k.to_string(), Variant::Float64(*v));
    }
    Variant::Attributes(a)
}

fn tags(names: &[&str]) -> Variant {
    let mut t = Tags::new();
    for n in names {
        t.push(n);
    }
    Variant::Tags(t)
}

/// root { Thing [attrs/tags] }
fn dom_with(attr_pairs: &[(&str, f64)], tag_names: &[&str]) -> WeakDom {
    let mut thing = InstanceBuilder::new("Folder").with_name("Thing");
    if !attr_pairs.is_empty() {
        thing = thing.with_property("Attributes", attrs(attr_pairs));
    }
    if !tag_names.is_empty() {
        thing = thing.with_property("Tags", tags(tag_names));
    }
    WeakDom::new(folder("root").with_child(thing))
}

fn dom_with_attribute(key: &str, value: Variant) -> WeakDom {
    let mut attributes = Attributes::new();
    attributes.insert(key.to_string(), value);
    WeakDom::new(
        folder("root").with_child(
            folder("Thing").with_property("Attributes", Variant::Attributes(attributes)),
        ),
    )
}

fn attr_of(dom: &WeakDom, key: &str) -> Option<f64> {
    let inst = dom.descendants().find(|i| i.name == "Thing").unwrap();
    match inst.properties.get(&"Attributes".into()) {
        Some(Variant::Attributes(a)) => match a.get(key) {
            Some(Variant::Float64(v)) => Some(*v),
            _ => None,
        },
        _ => None,
    }
}

fn tags_of(dom: &WeakDom) -> Vec<String> {
    let inst = dom.descendants().find(|i| i.name == "Thing").unwrap();
    match inst.properties.get(&"Tags".into()) {
        Some(Variant::Tags(t)) => {
            let mut v: Vec<String> = t.iter().map(|t| t.to_string()).collect();
            v.sort();
            v
        }
        _ => Vec::new(),
    }
}

// ============================================================================
// Diff granularity
// ============================================================================

#[test]
fn diff_reports_per_key_attribute_changes() {
    let old = dom_with(&[("keep", 1.0), ("changed", 1.0), ("removed", 1.0)], &[]);
    let new = dom_with(&[("keep", 1.0), ("changed", 2.0), ("added", 3.0)], &[]);

    let diffs = diff_doms(&old, &new);
    assert_eq!(diffs.len(), 1, "{diffs:#?}");
    let DiffEntry::Modified {
        property_changes, ..
    } = &diffs[0]
    else {
        panic!("expected Modified: {diffs:#?}");
    };
    let mut names: Vec<&str> = property_changes.iter().map(|c| c.name.as_str()).collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "Attributes.added",
            "Attributes.changed",
            "Attributes.removed"
        ],
        "{property_changes:#?}"
    );
}

#[test]
fn empty_container_on_one_side_is_not_a_change() {
    let old = dom_with(&[], &[]);
    let mut new_thing = InstanceBuilder::new("Folder").with_name("Thing");
    new_thing = new_thing.with_property("Attributes", Variant::Attributes(Attributes::new()));
    let new = WeakDom::new(folder("root").with_child(new_thing));

    let diffs = diff_doms(&old, &new);
    assert!(diffs.is_empty(), "{diffs:#?}");
}

#[test]
fn utf8_binary_attribute_displays_as_a_string() {
    let old = dom_with(&[], &[]);
    let session = "8719b00d-c64c-4fcb-8856-d124257d3411";
    let new = dom_with_attribute(
        "rodeoSession",
        Variant::BinaryString(BinaryString::from(session.as_bytes())),
    );

    let diffs = diff_doms(&old, &new);
    let [DiffEntry::Modified {
        property_changes, ..
    }] = diffs.as_slice()
    else {
        panic!("expected one attribute modification: {diffs:#?}");
    };
    let [change] = property_changes.as_slice() else {
        panic!("expected one attribute change: {property_changes:#?}");
    };
    assert_eq!(change.name, "Attributes.rodeoSession");
    assert!(matches!(
        &change.new_value,
        Some(PropertyValue::String { value }) if value == session
    ));
}

#[test]
fn string_and_binary_attribute_encodings_compare_equal() {
    let session = "8719b00d-c64c-4fcb-8856-d124257d3411";
    let old = dom_with_attribute("rodeoSession", Variant::String(session.to_string()));
    let new = dom_with_attribute(
        "rodeoSession",
        Variant::BinaryString(BinaryString::from(session.as_bytes())),
    );

    assert!(diff_doms(&old, &new).is_empty());
}

// ============================================================================
// Edit-script round trips
// ============================================================================

fn assert_round_trip(mut old: WeakDom, new: WeakDom) {
    let config = DiffConfig::default();
    let script = compute_edit_script(&old, &new, &config);
    apply_edit_script(&mut old, &new, &script);
    let residual = diff_doms(&old, &new);
    assert!(residual.is_empty(), "residual: {residual:#?}");
}

#[test]
fn round_trip_attribute_and_tag_changes() {
    assert_round_trip(
        dom_with(&[("a", 1.0), ("gone", 2.0)], &["t1", "t2"]),
        dom_with(&[("a", 9.0), ("new", 3.0)], &["t2", "t3"]),
    );
}

#[test]
fn round_trip_container_removal() {
    // All keys removed → containers disappear entirely
    assert_round_trip(dom_with(&[("a", 1.0)], &["t"]), dom_with(&[], &[]));
}

// ============================================================================
// Merge semantics
// ============================================================================

#[test]
fn different_attribute_keys_compose() {
    let mut base = dom_with(&[("shared", 1.0)], &[]);
    let ours = dom_with(&[("shared", 1.0), ("fromOurs", 2.0)], &[]);
    let theirs = dom_with(&[("shared", 1.0), ("fromTheirs", 3.0)], &[]);

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert_eq!(attr_of(&base, "shared"), Some(1.0));
    assert_eq!(attr_of(&base, "fromOurs"), Some(2.0));
    assert_eq!(attr_of(&base, "fromTheirs"), Some(3.0));
}

#[test]
fn same_attribute_key_conflicts() {
    let mut base = dom_with(&[("x", 1.0)], &[]);
    let ours = dom_with(&[("x", 2.0)], &[]);
    let theirs = dom_with(&[("x", 3.0)], &[]);

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert_eq!(result.conflicts.len(), 1, "{:?}", result.conflicts);
    assert_eq!(
        result.conflicts[0].kind,
        ConflictKind::Property {
            name: "Attributes.x".to_string()
        }
    );
    // Base value retained on the contested key
    assert_eq!(attr_of(&base, "x"), Some(1.0));
}

#[test]
fn identical_attribute_change_dedupes() {
    let mut base = dom_with(&[("x", 1.0)], &[]);
    let branch = || dom_with(&[("x", 5.0)], &[]);
    let result = merge_doms(&mut base, &branch(), &branch(), &DiffConfig::default());
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert!(result.stats.deduped >= 1);
    assert_eq!(attr_of(&base, "x"), Some(5.0));
}

#[test]
fn tags_always_compose() {
    // Presence-only values can't conflict: each side diverges from base in
    // only one direction per tag.
    let mut base = dom_with(&[], &["keep", "removedByOurs"]);
    let ours = dom_with(&[], &["keep", "addedByOurs"]);
    let theirs = dom_with(&[], &["keep", "removedByOurs", "addedByTheirs"]);

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert_eq!(tags_of(&base), vec!["addedByOurs", "addedByTheirs", "keep"]);
}

#[test]
fn both_remove_same_tag_dedupes() {
    let mut base = dom_with(&[], &["doomed", "keep"]);
    let branch = || dom_with(&[], &["keep"]);
    let result = merge_doms(&mut base, &branch(), &branch(), &DiffConfig::default());
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert_eq!(tags_of(&base), vec!["keep"]);
}

// ============================================================================
// Conflict file: stamp → mark → finalize on attribute conflicts
// ============================================================================

fn conflicted_attr_merge() -> WeakDom {
    let mut base = dom_with(&[("x", 1.0)], &[]);
    let ours = dom_with(&[("x", 2.0)], &[]);
    let theirs = dom_with(&[("x", 3.0)], &[]);
    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert_eq!(result.conflicts.len(), 1);
    stamp_conflicts(&mut base, &ours, &theirs, &result);

    // Round-trip the file format like the real flow does
    let mut buffer = Vec::new();
    rbx_binary::to_writer(&mut buffer, &base, base.root().children()).unwrap();
    rbx_binary::from_reader(buffer.as_slice()).unwrap()
}

#[test]
fn attribute_conflict_finalizes_both_ways() {
    for (side, expected) in [("ours", 2.0), ("theirs", 3.0)] {
        let mut dom = conflicted_attr_merge();
        let container = find_container(&dom).unwrap();
        let entries = list_entries(&dom, container);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].property.as_deref(), Some("Attributes.x"));
        mark_entry(&mut dom, entries[0].entry_ref, side).unwrap();
        finalize(&mut dom).unwrap();
        assert_eq!(attr_of(&dom, "x"), Some(expected), "side {side}");
        assert!(find_container(&dom).is_none());
    }
}

#[test]
fn attribute_conflict_accepts_custom_value() {
    let mut dom = conflicted_attr_merge();
    let container = find_container(&dom).unwrap();
    let entry = list_entries(&dom, container)[0].entry_ref;
    mark_entry_custom(&mut dom, entry, &serde_json::json!(7.5)).unwrap();
    finalize(&mut dom).unwrap();
    assert_eq!(attr_of(&dom, "x"), Some(7.5));
}
