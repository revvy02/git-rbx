//! Chained-conflict scenario: both sides move the same subtree to different
//! parents (MoveTarget conflict), while ONE side also adds new content inside
//! that subtree. The add is an independent, non-conflicting op — it must merge
//! into the subtree regardless of how (or before) the move conflict is
//! resolved, and travel with the subtree to whichever destination wins.

use rbx_diff::{
    finalize, find_container, list_entries, mark_entry, merge_doms, stamp_conflicts,
    ConflictKind, DiffConfig,
};
use rbx_dom_weak::{InstanceBuilder, WeakDom};
use rbx_types::Variant;

fn folder(name: &str) -> InstanceBuilder {
    InstanceBuilder::new("Folder").with_name(name)
}

fn part(name: &str) -> InstanceBuilder {
    InstanceBuilder::new("Part")
        .with_name(name)
        .with_property("Anchored", Variant::Bool(true))
}

/// base: root { A, B, S { Child } }
fn base_dom() -> WeakDom {
    WeakDom::new(
        folder("root")
            .with_child(folder("A"))
            .with_child(folder("B"))
            .with_child(folder("S").with_child(part("Child"))),
    )
}

/// ours: S moved under A.            root { A { S { Child } }, B }
/// theirs: S moved under B, plus a   root { A, B { S { Child, New } } }
/// new part added inside S.
fn merged_with_chain() -> WeakDom {
    let mut base = base_dom();
    let ours = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(folder("S").with_child(part("Child"))))
            .with_child(folder("B")),
    );
    let theirs = WeakDom::new(
        folder("root")
            .with_child(folder("A"))
            .with_child(
                folder("B").with_child(
                    folder("S").with_child(part("Child")).with_child(part("New")),
                ),
            ),
    );

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert_eq!(result.conflicts.len(), 1, "{:?}", result.conflicts);
    assert!(
        matches!(result.conflicts[0].kind, ConflictKind::MoveTarget),
        "{:?}",
        result.conflicts[0].kind
    );

    stamp_conflicts(&mut base, &ours, &theirs, &result);
    base
}

fn child_names(dom: &WeakDom, of: rbx_dom_weak::types::Ref) -> Vec<String> {
    let mut names: Vec<String> = dom
        .get_by_ref(of)
        .unwrap()
        .children()
        .iter()
        .map(|c| dom.get_by_ref(*c).unwrap().name.clone())
        .collect();
    names.sort();
    names
}

fn find_named(dom: &WeakDom, name: &str) -> rbx_dom_weak::types::Ref {
    fn walk(dom: &WeakDom, at: rbx_dom_weak::types::Ref, name: &str) -> Option<rbx_dom_weak::types::Ref> {
        let inst = dom.get_by_ref(at)?;
        if inst.name == name {
            return Some(at);
        }
        inst.children().iter().find_map(|c| walk(dom, *c, name))
    }
    walk(dom, dom.root_ref(), name).expect(name)
}

/// Before resolution: the theirs-side add already lives inside S, and S is
/// parked at its BASE location while the move conflict is unresolved.
#[test]
fn add_inside_contested_subtree_merges_immediately() {
    let dom = merged_with_chain();

    let s = find_named(&dom, "S");
    assert_eq!(child_names(&dom, s), vec!["Child", "New"]);

    let root = find_named(&dom, "root");
    let parent_of_s = dom.get_by_ref(s).unwrap().parent();
    assert_eq!(parent_of_s, root, "S must stay at base position while contested");
}

/// Resolving the move either way carries the merged contents (including the
/// other side's add) to the chosen destination.
#[test]
fn move_resolution_carries_merged_contents_both_ways() {
    for (side, dest_name) in [("ours", "A"), ("theirs", "B")] {
        let mut dom = merged_with_chain();
        let container = find_container(&dom).unwrap();
        let entry = list_entries(&dom, container)[0].entry_ref;

        mark_entry(&mut dom, entry, side).unwrap();
        finalize(&mut dom).unwrap();

        assert!(find_container(&dom).is_none(), "container must be stripped");
        let s = find_named(&dom, "S");
        let dest = find_named(&dom, dest_name);
        assert_eq!(
            dom.get_by_ref(s).unwrap().parent(),
            dest,
            "take {side}: S should land under {dest_name}"
        );
        assert_eq!(
            child_names(&dom, s),
            vec!["Child", "New"],
            "take {side}: theirs' add must ride along"
        );
    }
}
