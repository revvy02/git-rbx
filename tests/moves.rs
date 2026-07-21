//! Move detection integration tests: build old/new DOMs directly and assert
//! on the diff taxonomy (added / removed / modified / moved).

use rbx_diff::{diff_doms, DiffEntry};
use rbx_dom_weak::{InstanceBuilder, WeakDom};
use rbx_types::Variant;

fn folder(name: &str) -> InstanceBuilder {
    InstanceBuilder::new("Folder").with_name(name)
}

fn part(name: &str) -> InstanceBuilder {
    InstanceBuilder::new("Part")
        .with_name(name)
        .with_property("Anchored", Variant::Bool(true))
        .with_property("Transparency", Variant::Float32(0.0))
}

fn summarize(diffs: &[DiffEntry]) -> (usize, usize, usize, usize) {
    let mut counts = (0, 0, 0, 0);
    for d in diffs {
        match d {
            DiffEntry::Added { .. } => counts.0 += 1,
            DiffEntry::Removed { .. } => counts.1 += 1,
            DiffEntry::Modified { .. } => counts.2 += 1,
            DiffEntry::Moved { .. } => counts.3 += 1,
        }
    }
    counts
}

#[test]
fn pure_move_is_reported_as_moved_not_add_remove() {
    // old: A { P }, B {}    new: A {}, B { P }
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

    let diffs = diff_doms(&old, &new);
    let (added, removed, modified, moved) = summarize(&diffs);

    assert_eq!(moved, 1, "expected exactly one move, got diffs: {:?}", diffs);
    assert_eq!(added, 0);
    assert_eq!(removed, 0);
    assert_eq!(modified, 0);

    match diffs.iter().find(|d| matches!(d, DiffEntry::Moved { .. })).unwrap() {
        DiffEntry::Moved { old_path, path, class, property_changes, .. } => {
            assert_eq!(old_path, "root.A.P");
            assert_eq!(path, "root.B.P");
            assert_eq!(class, "Part");
            assert!(property_changes.is_empty(), "pure move should have no property changes");
        }
        _ => unreachable!(),
    }
}

#[test]
fn move_with_edit_reports_moved_with_property_changes() {
    // P moves from A to B and its Transparency changes
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
                        .with_property("Transparency", Variant::Float32(0.5)),
                ),
            ),
    );

    let diffs = diff_doms(&old, &new);
    let (added, removed, _modified, moved) = summarize(&diffs);

    assert_eq!(moved, 1, "expected one move, got diffs: {:?}", diffs);
    assert_eq!(added, 0);
    assert_eq!(removed, 0);

    match diffs.iter().find(|d| matches!(d, DiffEntry::Moved { .. })).unwrap() {
        DiffEntry::Moved { property_changes, .. } => {
            assert_eq!(property_changes.len(), 1, "changes: {:?}", property_changes);
            assert_eq!(property_changes[0].name, "Transparency");
        }
        _ => unreachable!(),
    }
}

#[test]
fn moved_subtree_with_nested_edit_reports_nested_modification() {
    // A whole folder moves, and a part inside it is edited
    let old = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(folder("Sub").with_child(part("P"))))
            .with_child(folder("B")),
    );
    let new = WeakDom::new(
        folder("root")
            .with_child(folder("A"))
            .with_child(
                folder("B").with_child(
                    folder("Sub").with_child(
                        InstanceBuilder::new("Part")
                            .with_name("P")
                            .with_property("Anchored", Variant::Bool(false))
                            .with_property("Transparency", Variant::Float32(0.0)),
                    ),
                ),
            ),
    );

    let diffs = diff_doms(&old, &new);
    let (added, removed, modified, moved) = summarize(&diffs);

    assert_eq!(moved, 1, "expected one move (Sub), got diffs: {:?}", diffs);
    assert_eq!(modified, 1, "expected nested edit on P, got diffs: {:?}", diffs);
    assert_eq!(added, 0);
    assert_eq!(removed, 0);
}

#[test]
fn dissimilar_same_name_instances_stay_added_and_removed() {
    // A Model full of parts is deleted from A; an unrelated empty Model with the
    // same name appears in B. Similarity should stay below threshold.
    let old = WeakDom::new(
        folder("root")
            .with_child(
                folder("A").with_child(
                    InstanceBuilder::new("Model")
                        .with_name("Thing")
                        .with_child(part("P1"))
                        .with_child(part("P2"))
                        .with_child(part("P3"))
                        .with_child(part("P4")),
                ),
            )
            .with_child(folder("B")),
    );
    let new = WeakDom::new(
        folder("root")
            .with_child(folder("A"))
            .with_child(
                folder("B").with_child(
                    InstanceBuilder::new("Model")
                        .with_name("Thing")
                        .with_child(part("Other1"))
                        .with_child(part("Other2"))
                        .with_child(part("Other3"))
                        .with_child(part("Other4")),
                ),
            ),
    );

    let diffs = diff_doms(&old, &new);
    let (added, removed, _modified, moved) = summarize(&diffs);

    assert_eq!(moved, 0, "dissimilar instances must not pair as a move: {:?}", diffs);
    assert_eq!(added, 1);
    assert_eq!(removed, 1);
}

#[test]
fn plain_add_and_remove_still_work() {
    // Different classes so neither the per-parent class fallback (which pairs
    // same-class siblings as renames) nor move detection can pair them.
    let old = WeakDom::new(
        folder("root").with_child(folder("A").with_child(part("Gone"))),
    );
    let new = WeakDom::new(
        folder("root").with_child(
            folder("A").with_child(InstanceBuilder::new("SpotLight").with_name("New")),
        ),
    );

    let diffs = diff_doms(&old, &new);
    let (added, removed, modified, moved) = summarize(&diffs);

    assert_eq!(added, 1);
    assert_eq!(removed, 1);
    assert_eq!(modified, 0);
    assert_eq!(moved, 0);
}

#[test]
fn unrelated_same_class_siblings_are_not_positional_renames() {
    // A deleted Model and two newly-added Models share only their class. The
    // old instance must not be consumed as a rename of the first new sibling.
    let old = WeakDom::new(folder("root").with_child(
        InstanceBuilder::new("Model")
            .with_name("Gone")
            .with_child(folder("OldOnly")),
    ));
    let new = WeakDom::new(
        folder("root")
            .with_child(
                InstanceBuilder::new("Model")
                    .with_name("NewOne")
                    .with_child(InstanceBuilder::new("SpotLight").with_name("NewOnly")),
            )
            .with_child(
                InstanceBuilder::new("Model")
                    .with_name("NewTwo")
                    .with_child(InstanceBuilder::new("Attachment").with_name("AlsoNew")),
            ),
    );

    let diffs = diff_doms(&old, &new);
    let (added, removed, modified, moved) = summarize(&diffs);
    assert_eq!((added, removed, modified, moved), (2, 1, 0, 0), "{diffs:#?}");
}

#[test]
fn descendants_of_replaced_containers_are_not_moves() {
    // Both boundary containers are unrelated replacements. Even though each
    // contains an identical `Shared` Part, pairing the two interior nodes as a
    // move would cannibalize the deleted tree when a merge keeps it.
    let old = WeakDom::new(folder("root").with_child(
        folder("Deleted")
            .with_child(part_with_color("Shared", 0.25))
            .with_child(folder("OldOnly")),
    ));
    let new = WeakDom::new(folder("root").with_child(
        InstanceBuilder::new("Model")
            .with_name("Added")
            .with_child(part_with_color("Shared", 0.25))
            .with_child(folder("NewOnly")),
    ));

    let diffs = diff_doms(&old, &new);
    let (added, removed, modified, moved) = summarize(&diffs);
    assert_eq!((added, removed, modified, moved), (1, 1, 0, 0), "{diffs:#?}");
}

#[test]
fn move_into_added_group_is_detected() {
    // The group_workspace_dupes pattern: existing instances get gathered
    // under a brand-new container. The container is added; the contents are
    // moves, not remove+add pairs.
    let old = WeakDom::new(
        folder("root").with_child(
            folder("A")
                .with_child(part_with_color("P", 0.1))
                .with_child(part_with_color("P", 0.2)),
        ),
    );
    let new = WeakDom::new(
        folder("root").with_child(
            folder("A").with_child(
                folder("Group")
                    .with_child(part_with_color("P", 0.1))
                    .with_child(part_with_color("P", 0.2)),
            ),
        ),
    );

    let diffs = diff_doms(&old, &new);
    let (added, removed, modified, moved) = summarize(&diffs);
    assert_eq!(moved, 2, "both parts should move into the group: {diffs:?}");
    assert_eq!(added, 1, "only the group itself is new: {diffs:?}");
    assert_eq!(removed, 0, "{diffs:?}");
    assert_eq!(modified, 0, "{diffs:?}");
}

#[test]
fn move_out_of_removed_folder_is_detected() {
    // A folder is deleted but one of its children was rescued elsewhere.
    let old = WeakDom::new(
        folder("root")
            .with_child(
                folder("Doomed")
                    .with_child(part_with_color("Keep", 0.3))
                    .with_child(part_with_color("Junk", 0.4)),
            )
            .with_child(folder("B")),
    );
    let new = WeakDom::new(
        folder("root").with_child(folder("B").with_child(part_with_color("Keep", 0.3))),
    );

    let diffs = diff_doms(&old, &new);
    let (added, removed, modified, moved) = summarize(&diffs);
    assert_eq!(moved, 1, "Keep should be a move: {diffs:?}");
    assert_eq!(removed, 1, "Doomed (with Junk inside) is removed: {diffs:?}");
    assert_eq!(added, 0, "{diffs:?}");
    assert_eq!(modified, 0, "{diffs:?}");
}

fn part_with_color(name: &str, transparency: f32) -> InstanceBuilder {
    InstanceBuilder::new("Part")
        .with_name(name)
        .with_property("Anchored", Variant::Bool(true))
        .with_property("Transparency", Variant::Float32(transparency))
}
