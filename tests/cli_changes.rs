//! `diff --format markdown` and `changes <base> <head>`: the outputs a CI
//! job posts to a step summary or pull-request comment, exercised against
//! a real repository (add, modify, rename, delete, and nothing-changed).

mod common;
use common::*;

use rbx_dom_weak::{InstanceBuilder, WeakDom};
use rbx_types::Variant;

fn prepare(repo: &Repo) {
    repo.git(&["commit", "-q", "--allow-empty", "-m", "root"]);
}

#[test]
fn diff_markdown_renders_count_line_and_tables() {
    let dir = std::env::temp_dir().join(format!("git-rbx-md-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let old = dir.join("old.rbxm");
    let new = dir.join("new.rbxm");
    write_model(&old, &map(0.0, 0.0));
    // Q modified; a new part with a pipe in its name (must be escaped).
    write_model(
        &new,
        &WeakDom::new(
            folder("root")
                .with_child(part_with("Q", 0.5))
                .with_child(part_with("R", 0.0))
                .with_child(part_with("Pipe|Name", 0.0)),
        ),
    );
    let output = std::process::Command::new(BIN)
        .args(["diff", old.to_str().unwrap(), new.to_str().unwrap(), "--format", "markdown"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let md = String::from_utf8(output.stdout).unwrap();
    assert!(md.contains("**1 added · 0 removed · 1 modified · 0 moved · 0 pivoted**"), "{md}");
    assert!(md.contains("<summary><b>Modified</b> (1)</summary>"), "{md}");
    assert!(md.contains("| `Q` | Transparency | `0` | `0.5` |"), "{md}");
    assert!(md.contains("<summary><b>Added</b> (1)</summary>"), "{md}");
    assert!(md.contains("`Pipe\\|Name`"), "pipes must be escaped inside tables: {md}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn diff_markdown_caps_rows() {
    let dir = std::env::temp_dir().join(format!("git-rbx-md-cap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let old = dir.join("old.rbxm");
    let new = dir.join("new.rbxm");
    write_model(&old, &WeakDom::new(folder("root")));
    let mut root = folder("root");
    for i in 0..12 {
        root = root.with_child(part_with(&format!("P{i}"), 0.0));
    }
    write_model(&new, &WeakDom::new(root));
    let output = std::process::Command::new(BIN)
        .args([
            "diff",
            old.to_str().unwrap(),
            new.to_str().unwrap(),
            "--format",
            "markdown",
            "--max-rows",
            "5",
        ])
        .output()
        .unwrap();
    let md = String::from_utf8(output.stdout).unwrap();
    assert!(md.contains("_… and 7 more_"), "{md}");
    assert_eq!(md.matches("| `P").count(), 5, "{md}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn changes_renders_one_section_per_changed_roblox_file() {
    let repo = Repo::new("changes");
    prepare(&repo);
    write_model(&repo.dir.join("map.rbxm"), &map(0.0, 0.0));
    write_model(&repo.dir.join("gone.rbxm"), &map(0.0, 0.0));
    write_model(&repo.dir.join("old-name.rbxm"), &map(0.3, 0.3));
    std::fs::write(repo.dir.join("notes.txt"), "not a roblox file\n").unwrap();
    repo.git(&["add", "."]);
    repo.git(&["commit", "-q", "-m", "base"]);

    // modify, delete, rename (identical content), add, plus a text change
    // that must be ignored.
    write_model(&repo.dir.join("map.rbxm"), &map(0.7, 0.0));
    std::fs::remove_file(repo.dir.join("gone.rbxm")).unwrap();
    repo.git(&["mv", "old-name.rbxm", "new-name.rbxm"]);
    // Structurally different content, or git's rename detection (50%
    // similarity) pairs it with the deleted gone.rbxm.
    let mut fresh = folder("root");
    for i in 0..12 {
        fresh = fresh.with_child(part_with(&format!("Brick{i}_{:x}", i * 7919), 0.0));
    }
    write_model(&repo.dir.join("fresh.rbxm"), &WeakDom::new(fresh));
    std::fs::write(repo.dir.join("notes.txt"), "changed\n").unwrap();
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "changes"]);

    let output = repo.rbx(&["changes", "HEAD~1", "HEAD"]);
    let md = String::from_utf8(output.stdout).unwrap();
    assert!(md.contains("### `map.rbxm`\n"), "{md}");
    assert!(md.contains("| `Q` | Transparency | `0` | `0.7` |"), "{md}");
    assert!(md.contains("### `gone.rbxm` (deleted)"), "{md}");
    assert!(md.contains("### `fresh.rbxm` (added)"), "{md}");
    assert!(md.contains("**12 added · 0 removed"), "added file lists its instances: {md}");
    assert!(md.contains("### `new-name.rbxm` (renamed from `old-name.rbxm`)"), "{md}");
    assert!(md.contains("_No semantic differences._"), "pure rename has no content diff: {md}");
    assert!(!md.contains("notes.txt"), "{md}");

    // JSON carries per-file counts and the diff entries.
    let output = repo.rbx(&["changes", "HEAD~1", "HEAD", "--format", "json"]);
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let files = json.as_array().unwrap();
    assert_eq!(files.len(), 4, "{json:#}");
    let map_entry = files.iter().find(|f| f["path"] == "map.rbxm").unwrap();
    assert_eq!(map_entry["status"], "M");
    assert_eq!(map_entry["counts"]["modified"], 1);
    assert_eq!(map_entry["diffs"][0]["type"], "modified");
    let renamed = files.iter().find(|f| f["path"] == "new-name.rbxm").unwrap();
    assert_eq!(renamed["status"], "R");
    assert_eq!(renamed["oldPath"], "old-name.rbxm");
}

#[test]
fn changes_with_no_roblox_files_says_so() {
    let repo = Repo::new("changes-none");
    prepare(&repo);
    std::fs::write(repo.dir.join("notes.txt"), "a\n").unwrap();
    repo.git(&["add", "."]);
    repo.git(&["commit", "-q", "-m", "text only"]);
    let output = repo.rbx(&["changes", "HEAD~1", "HEAD"]);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "_No Roblox files changed._"
    );
    let output = repo.rbx(&["changes", "HEAD~1", "HEAD", "--format", "json"]);
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "[]");
}

#[test]
fn changes_handles_instances_with_attributes() {
    // Attribute edits render as granular `Attributes.<key>` rows.
    let repo = Repo::new("changes-attrs");
    prepare(&repo);
    let with_attr = |value: f64| {
        let attrs = rbx_types::Attributes::new().with("Speed", Variant::Float64(value));
        WeakDom::new(folder("root").with_child(
            InstanceBuilder::new("Part")
                .with_name("Car")
                .with_property("Attributes", Variant::Attributes(attrs)),
        ))
    };
    write_model(&repo.dir.join("car.rbxm"), &with_attr(1.0));
    repo.git(&["add", "."]);
    repo.git(&["commit", "-q", "-m", "base"]);
    write_model(&repo.dir.join("car.rbxm"), &with_attr(2.0));
    repo.git(&["commit", "-q", "-am", "faster"]);
    let output = repo.rbx(&["changes", "HEAD~1", "HEAD"]);
    let md = String::from_utf8(output.stdout).unwrap();
    assert!(md.contains("| `Car` | Attributes.Speed | `1` | `2` |"), "{md}");
}
