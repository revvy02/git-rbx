//! Identity stress scenarios for the 3-way merge: ambiguous same-name
//! siblings edited on both branches, rename+reparent in one commit, and
//! cross-branch Ref properties into deduplicated added subtrees. Each case
//! here started as a confirmed false conflict (or lost move) — keep them
//! passing when touching match_instances, move_detect, or the merge combiner.

use rbx_diff::{diff_doms, merge_doms, DiffConfig};
use rbx_dom_weak::{types::Ref, InstanceBuilder, WeakDom};
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

fn find_by_path(dom: &WeakDom, path: &[&str]) -> Option<Ref> {
    let mut current = dom.root_ref();
    for segment in path {
        let inst = dom.get_by_ref(current)?;
        current = *inst
            .children()
            .iter()
            .find(|c| dom.get_by_ref(**c).map(|i| i.name.as_str()) == Some(*segment))?;
    }
    Some(current)
}

fn transparencies(dom: &WeakDom, parent: &[&str]) -> Vec<f32> {
    let parent_ref = find_by_path(dom, parent).unwrap();
    dom.get_by_ref(parent_ref)
        .unwrap()
        .children()
        .iter()
        .filter_map(|c| {
            let inst = dom.get_by_ref(*c)?;
            match inst.properties.get(&"Transparency".into()) {
                Some(Variant::Float32(t)) => Some(*t),
                _ => None,
            }
        })
        .collect()
}

// ---------- Item 1: ambiguous same-name/same-class siblings ----------

/// Distinct-content twins, each branch edits a different one. Should merge
/// cleanly with each edit landing on the right twin.
#[test]
fn probe_distinct_twins_parallel_edits() {
    let mut base = WeakDom::new(
        folder("root").with_child(
            folder("G")
                .with_child(part_with("P", 0.0))
                .with_child(part_with("P", 0.5)),
        ),
    );
    let ours = WeakDom::new(
        folder("root").with_child(
            folder("G")
                .with_child(part_with("P", 0.1)) // edited the 0.0 twin
                .with_child(part_with("P", 0.5)),
        ),
    );
    let theirs = WeakDom::new(
        folder("root").with_child(
            folder("G")
                .with_child(part_with("P", 0.0))
                .with_child(part_with("P", 0.6)), // edited the 0.5 twin
        ),
    );
    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    eprintln!(
        "[probe] distinct twins: conflicts={:?} result={:?}",
        result
            .conflicts
            .iter()
            .map(|c| (&c.path, &c.kind))
            .collect::<Vec<_>>(),
        transparencies(&base, &["G"]),
    );
    assert!(result.conflicts.is_empty(), "{:#?}", result.conflicts);
    let mut got = transparencies(&base, &["G"]);
    got.sort_by(f32::total_cmp);
    assert_eq!(got, vec![0.1, 0.6]);
}

/// IDENTICAL twins, ours edits the first (by position), theirs edits the
/// second. Position-aware hash tiebreaking pairs both branch mappings
/// positionally, so the edits land on different twins and compose cleanly.
#[test]
fn probe_identical_twins_parallel_edits() {
    let mut base = WeakDom::new(
        folder("root").with_child(
            folder("G")
                .with_child(part_with("P", 0.0))
                .with_child(part_with("P", 0.0)),
        ),
    );
    let ours = WeakDom::new(
        folder("root").with_child(
            folder("G")
                .with_child(part_with("P", 0.1)) // first twin edited
                .with_child(part_with("P", 0.0)),
        ),
    );
    let theirs = WeakDom::new(
        folder("root").with_child(
            folder("G")
                .with_child(part_with("P", 0.0))
                .with_child(part_with("P", 0.2)), // second twin edited
        ),
    );
    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    eprintln!(
        "[probe] identical twins: conflicts={:?} result={:?}",
        result
            .conflicts
            .iter()
            .map(|c| (&c.path, &c.kind))
            .collect::<Vec<_>>(),
        transparencies(&base, &["G"]),
    );
    assert!(
        result.conflicts.is_empty(),
        "false conflict on identical twins: {:#?}",
        result.conflicts
    );
    let mut got = transparencies(&base, &["G"]);
    got.sort_by(f32::total_cmp);
    assert_eq!(got, vec![0.1, 0.2]);
}

/// Identical twins: ours deletes one, theirs edits one. Whether this is a
/// conflict depends on which twin the delete binds to — probe what happens.
#[test]
fn probe_identical_twins_delete_vs_edit() {
    let mut base = WeakDom::new(
        folder("root").with_child(
            folder("G")
                .with_child(part_with("P", 0.0))
                .with_child(part_with("P", 0.0)),
        ),
    );
    // ours: one twin left
    let ours = WeakDom::new(folder("root").with_child(folder("G").with_child(part_with("P", 0.0))));
    // theirs: first twin edited
    let theirs = WeakDom::new(
        folder("root").with_child(
            folder("G")
                .with_child(part_with("P", 0.9))
                .with_child(part_with("P", 0.0)),
        ),
    );
    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    eprintln!(
        "[probe] twins delete-vs-edit: conflicts={:?} result={:?}",
        result
            .conflicts
            .iter()
            .map(|c| (&c.path, &c.kind))
            .collect::<Vec<_>>(),
        transparencies(&base, &["G"]),
    );
    // No assertion — reporting behavior. The benign reading is: delete binds
    // to the untouched twin, edit survives → [0.9], no conflict.
}

// ---------- Item 2: rename + reparent ----------

/// A subtree that is renamed AND reparented in one commit pairs through the
/// name-less exact-hash move pass: one Moved entry plus a Name change, no
/// remove+add.
#[test]
fn probe_rename_plus_reparent_diff() {
    let old = WeakDom::new(
        folder("root")
            .with_child(folder("Src").with_child(folder("Box").with_child(part_with("P", 0.25))))
            .with_child(folder("Dst")),
    );
    let new = WeakDom::new(folder("root").with_child(folder("Src")).with_child(
        folder("Dst").with_child(folder("BoxRenamed").with_child(part_with("P", 0.25))),
    ));
    let diffs = diff_doms(&old, &new);
    eprintln!("[probe] rename+reparent diff: {diffs:#?}");
    assert!(
        diffs
            .iter()
            .any(|d| matches!(d, rbx_diff::DiffEntry::Moved { .. })),
        "expected a Moved entry: {diffs:#?}"
    );
    assert!(
        !diffs.iter().any(|d| matches!(
            d,
            rbx_diff::DiffEntry::Removed { .. } | rbx_diff::DiffEntry::Added { .. }
        )),
        "rename+reparent must not decay to remove+add: {diffs:#?}"
    );
}

/// Merge consequence: ours renames+reparents the container, theirs edits a
/// descendant. With the rename+move visible, both compose cleanly: the
/// container ends up renamed at its new location with the edit applied.
#[test]
fn probe_rename_plus_reparent_merge() {
    let mut base = WeakDom::new(
        folder("root")
            .with_child(folder("Src").with_child(folder("Box").with_child(part_with("P", 0.25))))
            .with_child(folder("Dst")),
    );
    let ours = WeakDom::new(folder("root").with_child(folder("Src")).with_child(
        folder("Dst").with_child(folder("BoxRenamed").with_child(part_with("P", 0.25))),
    ));
    let theirs = WeakDom::new(
        folder("root")
            .with_child(folder("Src").with_child(folder("Box").with_child(part_with("P", 0.75))))
            .with_child(folder("Dst")),
    );
    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    eprintln!(
        "[probe] rename+reparent merge: conflicts={:?}",
        result
            .conflicts
            .iter()
            .map(|c| (&c.path, &c.kind))
            .collect::<Vec<_>>(),
    );
    assert!(result.conflicts.is_empty(), "{:#?}", result.conflicts);
    let final_transparency = find_by_path(&base, &["Dst", "BoxRenamed", "P"])
        .and_then(|r| base.get_by_ref(r))
        .and_then(|i| match i.properties.get(&"Transparency".into()) {
            Some(Variant::Float32(t)) => Some(*t),
            _ => None,
        });
    assert_eq!(
        final_transparency,
        Some(0.75),
        "theirs' edit must survive at the renamed destination"
    );
}

/// Both branches evacuate P from A to the same destination, then delete A —
/// identical intent on both sides. The identical moves dedupe and the common
/// delete composes: no delete-vs-edit conflict, P survives at B, A is gone.
/// (Asymmetric evacuation — only one side moves P out — must still conflict;
/// that case is covered by resolve.rs's move-out tests.)
#[test]
fn symmetric_evacuation_composes() {
    let mut base = WeakDom::new(
        folder("root")
            .with_child(folder("A").with_child(part_with("P", 0.25)))
            .with_child(folder("B")),
    );
    let branch = || {
        WeakDom::new(folder("root").with_child(folder("B").with_child(part_with("P", 0.25))))
    };
    let (ours, theirs) = (branch(), branch());
    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    assert!(result.conflicts.is_empty(), "{:#?}", result.conflicts);
    assert!(
        find_by_path(&base, &["A"]).is_none(),
        "the commonly-deleted container must not survive"
    );
    assert!(
        find_by_path(&base, &["B", "P"]).is_some(),
        "the evacuated instance must survive at its destination"
    );
}

// ---------- Item 4: refs into deduplicated identical adds ----------

/// Both branches add the identical subtree AND point an existing instance's
/// Ref property at something inside it. The adds dedupe, and the refs —
/// each addressing its branch's own copy — compare equal through the
/// deduplicated-add equivalence: no conflict, and the merged Holder points
/// at the single merged Target.
#[test]
fn probe_ref_into_deduped_add() {
    let mut base = WeakDom::new(
        folder("root")
            .with_child(InstanceBuilder::new("ObjectValue").with_name("Holder"))
            .with_child(folder("B")),
    );

    let build_branch = || {
        let mut dom = WeakDom::new(
            folder("root")
                .with_child(InstanceBuilder::new("ObjectValue").with_name("Holder"))
                .with_child(folder("B"))
                .with_child(folder("New").with_child(part_with("Target", 0.0))),
        );
        let target = find_by_path(&dom, &["New", "Target"]).unwrap();
        let holder = find_by_path(&dom, &["Holder"]).unwrap();
        dom.get_by_ref_mut(holder)
            .unwrap()
            .properties
            .insert("Value".into(), Variant::Ref(target));
        dom
    };
    let ours = build_branch();
    let theirs = build_branch();

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());
    eprintln!(
        "[probe] ref into deduped add: conflicts={:?} stats: ours={} theirs={} deduped={}",
        result
            .conflicts
            .iter()
            .map(|c| (&c.path, &c.kind))
            .collect::<Vec<_>>(),
        result.stats.ours_applied,
        result.stats.theirs_applied,
        result.stats.deduped,
    );
    assert!(result.conflicts.is_empty(), "{:#?}", result.conflicts);
    let holder = find_by_path(&base, &["Holder"]).unwrap();
    let value = base
        .get_by_ref(holder)
        .unwrap()
        .properties
        .get(&"Value".into())
        .cloned();
    let target = find_by_path(&base, &["New", "Target"]).expect("deduped add must materialize");
    assert_eq!(
        value,
        Some(Variant::Ref(target)),
        "Holder.Value must resolve to the merged Target"
    );
}
