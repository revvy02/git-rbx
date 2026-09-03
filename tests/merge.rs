//! Three-way merge combiner tests: clean composition, dedupe, and each
//! conflict kind. The merged DOM is verified by diffing against a hand-built
//! expected DOM (empty diff = exact merge).

use git_rbx::{
    diff_doms, merge_compact_doms, merge_doms, ConflictKind, DiffConfig, DiffDom, MergeResult,
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
        .with_property("Transparency", Variant::Float32(0.0))
}

fn part_with(name: &str, transparency: f32) -> InstanceBuilder {
    InstanceBuilder::new("Part")
        .with_name(name)
        .with_property("Anchored", Variant::Bool(true))
        .with_property("Transparency", Variant::Float32(transparency))
}

/// base: root { A { P }, B }
fn base_dom() -> WeakDom {
    WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part("P")))
            .with_child(folder("B")),
    )
}

fn assert_merged_equals(merged: &WeakDom, expected: &WeakDom) {
    let residual = diff_doms(merged, expected);
    assert!(
        residual.is_empty(),
        "merged DOM differs from expected: {residual:#?}"
    );
}

fn conflict_signature(result: &MergeResult) -> Vec<(String, ConflictKind, usize, usize)> {
    result
        .conflicts
        .iter()
        .map(|conflict| {
            (
                conflict.path.clone(),
                conflict.kind.clone(),
                conflict.ours.edits.len(),
                conflict.theirs.edits.len(),
            )
        })
        .collect()
}

#[test]
fn disjoint_changes_compose() {
    // ours: edit P.Transparency; theirs: add Q under B
    let mut base = base_dom();
    let ours = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part_with("P", 0.5)))
            .with_child(folder("B")),
    );
    let theirs = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part("P")))
            .with_child(folder("B").with_child(part("Q"))),
    );

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);

    let expected = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part_with("P", 0.5)))
            .with_child(folder("B").with_child(part("Q"))),
    );
    assert_merged_equals(&base, &expected);
}

#[test]
fn identical_changes_dedupe() {
    let mut base = base_dom();
    let branch = || {
        WeakDom::new(
            folder("root")
                .with_child(folder("A").with_child(part_with("P", 0.5)))
                .with_child(folder("B")),
        )
    };
    let result = merge_doms(&mut base, &branch(), &branch(), &DiffConfig::default());
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert!(result.stats.deduped >= 1, "expected dedupe: {:?}", result.stats);
    assert_merged_equals(&base, &branch());
}

#[test]
fn property_conflict_keeps_base_and_reports() {
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
    assert_eq!(result.conflicts.len(), 1, "{:?}", result.conflicts);
    let conflict = &result.conflicts[0];
    assert_eq!(conflict.kind, ConflictKind::Property { name: "Transparency".to_string() });
    assert_eq!(conflict.path, "root.A.P");

    // Base content retained for the contested property
    assert_merged_equals(&base, &base_dom());
}

#[test]
fn reparent_composes_with_edit() {
    // The marquee case: ours reparents P from A to B; theirs edits P's property.
    let mut base = base_dom();
    let ours = WeakDom::new(
        folder("root")
            .with_child(folder("A"))
            .with_child(folder("B").with_child(part("P"))),
    );
    let theirs = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part_with("P", 0.5)))
            .with_child(folder("B")),
    );

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);

    let expected = WeakDom::new(
        folder("root")
            .with_child(folder("A"))
            .with_child(folder("B").with_child(part_with("P", 0.5))),
    );
    assert_merged_equals(&base, &expected);
}

#[test]
fn conflicting_reparent_destinations() {
    // ours: P → B; theirs: P → C
    let mut base = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part("P")))
            .with_child(folder("B"))
            .with_child(folder("C")),
    );
    let ours = WeakDom::new(
        folder("root")
            .with_child(folder("A"))
            .with_child(folder("B").with_child(part("P")))
            .with_child(folder("C")),
    );
    let theirs = WeakDom::new(
        folder("root")
            .with_child(folder("A"))
            .with_child(folder("B"))
            .with_child(folder("C").with_child(part("P"))),
    );

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert_eq!(result.conflicts.len(), 1, "{:?}", result.conflicts);
    assert_eq!(result.conflicts[0].kind, ConflictKind::ReparentTarget);
    assert_eq!(result.conflicts[0].path, "root.A.P");
}

#[test]
fn delete_vs_edit_conflicts() {
    // ours: remove folder A entirely; theirs: edit P inside A
    let mut base = base_dom();
    let ours = WeakDom::new(folder("root").with_child(folder("B")));
    let theirs = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part_with("P", 0.5)))
            .with_child(folder("B")),
    );

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert!(
        result.conflicts.iter().any(|c| c.kind == ConflictKind::DeleteVsEdit && c.path == "root.A"),
        "{:?}",
        result.conflicts
    );
    // Neither the removal nor the edit applied: base retained
    assert_merged_equals(&base, &base_dom());
}

#[test]
fn multiple_edits_under_one_deleted_subtree_are_one_conflict() {
    let mut base = WeakDom::new(
        folder("root").with_child(
            folder("A")
                .with_child(part("P"))
                .with_child(part("Q")),
        ),
    );
    let ours = WeakDom::new(folder("root"));
    let theirs = WeakDom::new(
        folder("root").with_child(
            folder("A")
                .with_child(part_with("P", 0.25))
                .with_child(part_with("Q", 0.75)),
        ),
    );

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());

    assert_eq!(result.conflicts.len(), 1, "{:?}", result.conflicts);
    let conflict = &result.conflicts[0];
    assert_eq!(conflict.kind, ConflictKind::DeleteVsEdit);
    assert_eq!(conflict.path, "root.A");
    assert_eq!(conflict.ours.edits.len(), 1, "one subtree deletion");
    assert_eq!(
        conflict.theirs.edits.len(),
        2,
        "both descendant edits retained"
    );
}

#[test]
fn reparent_into_deleted_subtree_conflicts() {
    // ours: remove B; theirs: move P into B
    let mut base = base_dom();
    let ours = WeakDom::new(folder("root").with_child(folder("A").with_child(part("P"))));
    let theirs = WeakDom::new(
        folder("root")
            .with_child(folder("A"))
            .with_child(folder("B").with_child(part("P"))),
    );

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert!(
        result.conflicts.iter().any(|c| c.kind == ConflictKind::DeleteVsEdit),
        "{:?}",
        result.conflicts
    );
}

#[test]
fn both_delete_same_subtree_composes() {
    let mut base = base_dom();
    let branch = || WeakDom::new(folder("root").with_child(folder("B")));
    let result = merge_doms(&mut base, &branch(), &branch(), &DiffConfig::default());
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert_merged_equals(&base, &branch());
}

#[test]
fn both_add_identical_content_dedupes() {
    let mut base = base_dom();
    let branch = || {
        WeakDom::new(
            folder("root")
                .with_child(folder("A").with_child(part("P")))
                .with_child(folder("B").with_child(part("Q"))),
        )
    };
    let result = merge_doms(&mut base, &branch(), &branch(), &DiffConfig::default());
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert_merged_equals(&base, &branch());
}

#[test]
fn identical_additions_dedupe_one_to_one() {
    let mut base = base_dom();
    let ours = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part("P")))
            .with_child(folder("B").with_child(part("Q"))),
    );
    let theirs = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part("P")))
            .with_child(folder("B").with_child(part("Q")).with_child(part("Q"))),
    );

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);
    assert_eq!(result.stats.deduped, 1);

    let expected = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part("P")))
            .with_child(folder("B").with_child(part("Q")).with_child(part("Q"))),
    );
    assert_merged_equals(&base, &expected);
}

#[test]
fn both_add_different_content_composes() {
    let mut base = base_dom();
    let ours = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part("P")))
            .with_child(folder("B").with_child(part("FromOurs"))),
    );
    let theirs = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part("P")))
            .with_child(folder("B").with_child(part("FromTheirs"))),
    );
    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert!(result.conflicts.is_empty(), "{:?}", result.conflicts);

    let expected = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part("P")))
            .with_child(
                folder("B")
                    .with_child(part("FromOurs"))
                    .with_child(part("FromTheirs")),
            ),
    );
    assert_merged_equals(&base, &expected);
}

#[test]
fn rename_conflict() {
    let mut base = base_dom();
    let ours = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part("RenamedByUs")))
            .with_child(folder("B")),
    );
    let theirs = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part("RenamedByThem")))
            .with_child(folder("B")),
    );
    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert!(
        result
            .conflicts
            .iter()
            .any(|c| c.kind == ConflictKind::Property { name: "Name".to_string() }),
        "{:?}",
        result.conflicts
    );
}

#[test]
fn compact_branches_match_weak_merge_planning_and_materialization() {
    let ours = WeakDom::new(
        folder("root")
            .with_child(folder("A"))
            .with_child(
                folder("B")
                    .with_child(part("P"))
                    .with_child(part("FromOurs")),
            ),
    );
    let theirs = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part_with("P", 0.5)))
            .with_child(folder("B").with_child(part("FromTheirs"))),
    );
    let compact_ours = DiffDom::from_weak_dom(&ours);
    let compact_theirs = DiffDom::from_weak_dom(&theirs);

    let mut weak_base = base_dom();
    let weak_result = merge_doms(
        &mut weak_base,
        &ours,
        &theirs,
        &DiffConfig::default(),
    );
    let mut compact_base = base_dom();
    let compact_result = merge_compact_doms(
        &mut compact_base,
        &compact_ours,
        &compact_theirs,
        &DiffConfig::default(),
    );

    assert_eq!(
        conflict_signature(&compact_result),
        conflict_signature(&weak_result)
    );
    assert_eq!(compact_result.stats.ours_applied, weak_result.stats.ours_applied);
    assert_eq!(
        compact_result.stats.theirs_applied,
        weak_result.stats.theirs_applied
    );
    assert_eq!(compact_result.stats.deduped, weak_result.stats.deduped);
    assert_merged_equals(&compact_base, &weak_base);
}

#[test]
fn compact_branches_preserve_conflict_ownership() {
    let ours =
        WeakDom::new(folder("root").with_child(folder("B").with_child(part("FromOurs"))));
    let theirs = WeakDom::new(
        folder("root")
            .with_child(
                folder("A")
                    .with_child(part_with("P", 0.25))
                    .with_child(part("FromTheirs")),
            )
            .with_child(folder("B")),
    );
    let compact_ours = DiffDom::from_weak_dom(&ours);
    let compact_theirs = DiffDom::from_weak_dom(&theirs);

    let mut weak_base = base_dom();
    let weak_result = merge_doms(
        &mut weak_base,
        &ours,
        &theirs,
        &DiffConfig::default(),
    );
    let mut compact_base = base_dom();
    let compact_result = merge_compact_doms(
        &mut compact_base,
        &compact_ours,
        &compact_theirs,
        &DiffConfig::default(),
    );

    assert_eq!(
        conflict_signature(&compact_result),
        conflict_signature(&weak_result)
    );
    assert_merged_equals(&compact_base, &weak_base);
}
