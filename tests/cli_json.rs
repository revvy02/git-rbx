//! End-to-end CLI contract for automation: the built binary driven exactly as
//! an agent would drive it after `git merge` hands back a conflicted file.
//!
//!   merge --json  →  resolve --list --json  →  resolve --take …
//!   →  resolve --finalize  →  check --json
//!
//! Exit codes and stdout JSON are the interface; stderr is human chatter.

use rbx_dom_weak::{InstanceBuilder, WeakDom};
use rbx_types::Variant;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_git-rbx");

fn folder(name: &str) -> InstanceBuilder {
    InstanceBuilder::new("Folder").with_name(name)
}

fn part_with(name: &str, transparency: f32) -> InstanceBuilder {
    InstanceBuilder::new("Part")
        .with_name(name)
        .with_property("Anchored", Variant::Bool(true))
        .with_property("Transparency", Variant::Float32(transparency))
}

fn write_model(path: &Path, dom: &WeakDom) {
    let file = std::fs::File::create(path).unwrap();
    rbx_binary::to_writer(file, dom, dom.root().children()).unwrap();
}

fn read_model(path: &Path) -> WeakDom {
    rbx_binary::from_reader(std::fs::File::open(path).unwrap()).unwrap()
}

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rbx-diff-cli-json-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .output()
        .expect("git-rbx binary runs")
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}):\n--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// Property values use the same typed shape as `diff --json`.
fn f32_value(value: f64) -> Value {
    serde_json::json!({"type": "float32", "value": {"value": value}})
}

fn transparency(dom: &WeakDom, name: &str) -> Option<f32> {
    dom.descendants().find(|i| i.name == name).and_then(|i| {
        match i.properties.get(&"Transparency".into()) {
            Some(Variant::Float32(t)) => Some(*t),
            _ => None,
        }
    })
}

#[test]
fn agent_loop_merge_list_take_finalize_check() {
    let dir = scratch_dir("loop");
    let base_path = dir.join("base.rbxm");
    let ours_path = dir.join("ours.rbxm");
    let theirs_path = dir.join("theirs.rbxm");
    let merged_path = dir.join("merged.rbxm");
    let merged = merged_path.to_str().unwrap();

    // Q conflicts (0.1 vs 0.2); R composes (only theirs touched it).
    write_model(
        &base_path,
        &WeakDom::new(
            folder("root")
                .with_child(part_with("Q", 0.0))
                .with_child(part_with("R", 0.0)),
        ),
    );
    write_model(
        &ours_path,
        &WeakDom::new(
            folder("root")
                .with_child(part_with("Q", 0.1))
                .with_child(part_with("R", 0.0)),
        ),
    );
    write_model(
        &theirs_path,
        &WeakDom::new(
            folder("root")
                .with_child(part_with("Q", 0.2))
                .with_child(part_with("R", 0.9)),
        ),
    );

    // merge --json: exit 1 (conflicts), JSON summary on stdout
    let output = run(&[
        "merge",
        base_path.to_str().unwrap(),
        ours_path.to_str().unwrap(),
        theirs_path.to_str().unwrap(),
        "--output",
        merged,
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let summary = stdout_json(&output);
    assert_eq!(summary["clean"], false);
    assert_eq!(summary["output"], merged);
    assert_eq!(summary["stats"]["conflicted"], 1);
    assert_eq!(summary["stats"]["theirsApplied"], 1, "R's edit composes");
    assert_eq!(summary["conflictCount"], 1);
    assert_eq!(summary["unresolvedCount"], 1);
    let conflict = &summary["conflicts"][0];
    assert_eq!(conflict["name"], "Conflict_1");
    assert_eq!(conflict["kind"], "Property");
    assert_eq!(conflict["path"], "Q");
    assert_eq!(conflict["property"], "Transparency");
    assert_eq!(
        conflict["ours"]["impact"]["ops"][0]["after"],
        f32_value(0.1)
    );
    assert_eq!(
        conflict["theirs"]["impact"]["ops"][0]["after"],
        f32_value(0.2)
    );

    // The written file carries the state; the composed edit is already in.
    let stamped = read_model(&merged_path);
    assert_eq!(transparency(&stamped, "R"), Some(0.9));
    assert_eq!(transparency(&stamped, "Q"), Some(0.0), "conflicted target keeps base");

    // check --json: exit 1 while unresolved
    let output = run(&["check", merged, "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let check = stdout_json(&output);
    assert_eq!(check["clean"], false);
    assert_eq!(check["unresolvedCount"], 1);

    // resolve --list --json: identical report to what merge printed
    let output = run(&["resolve", merged, "--list", "--json"]);
    assert!(output.status.success(), "{output:?}");
    let listed = stdout_json(&output);
    assert_eq!(listed["conflicts"], summary["conflicts"]);
    assert_eq!(listed["unresolvedCount"], 1);

    // resolve --take theirs by the entry name the JSON gave us
    let output = run(&["resolve", merged, "--take", "theirs", "--entry", "Conflict_1"]);
    assert!(output.status.success(), "{output:?}");
    let output = run(&["resolve", merged, "--list", "--json"]);
    let listed = stdout_json(&output);
    assert_eq!(listed["unresolvedCount"], 0);
    assert_eq!(listed["conflicts"][0]["resolved"], "theirs");

    // finalize → clean file with theirs' value, check exits 0
    let output = run(&["resolve", merged, "--finalize"]);
    assert!(output.status.success(), "{output:?}");
    let output = run(&["check", merged, "--json"]);
    assert!(output.status.success(), "{output:?}");
    let check = stdout_json(&output);
    assert_eq!(check["clean"], true);
    assert_eq!(check["unresolvedCount"], 0);

    let final_dom = read_model(&merged_path);
    assert_eq!(transparency(&final_dom, "Q"), Some(0.2));
    assert_eq!(transparency(&final_dom, "R"), Some(0.9));
    assert!(
        final_dom.descendants().all(|i| i.name != git_rbx::CONTAINER_NAME),
        "finalize strips the container"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn clean_merge_json_reports_no_conflicts_and_exits_zero() {
    let dir = scratch_dir("clean");
    let base_path = dir.join("base.rbxm");
    let ours_path = dir.join("ours.rbxm");
    let theirs_path = dir.join("theirs.rbxm");
    let merged_path = dir.join("merged.rbxm");

    write_model(
        &base_path,
        &WeakDom::new(
            folder("root")
                .with_child(part_with("Q", 0.0))
                .with_child(part_with("R", 0.0)),
        ),
    );
    write_model(
        &ours_path,
        &WeakDom::new(
            folder("root")
                .with_child(part_with("Q", 0.1))
                .with_child(part_with("R", 0.0)),
        ),
    );
    write_model(
        &theirs_path,
        &WeakDom::new(
            folder("root")
                .with_child(part_with("Q", 0.0))
                .with_child(part_with("R", 0.9)),
        ),
    );

    let output = run(&[
        "merge",
        base_path.to_str().unwrap(),
        ours_path.to_str().unwrap(),
        theirs_path.to_str().unwrap(),
        "--output",
        merged_path.to_str().unwrap(),
        "--json",
    ]);
    assert!(output.status.success(), "{output:?}");
    let summary = stdout_json(&output);
    assert_eq!(summary["clean"], true);
    assert_eq!(summary["conflictCount"], 0);
    assert_eq!(summary["conflicts"], serde_json::json!([]));
    assert_eq!(summary["stats"]["oursApplied"], 1);
    assert_eq!(summary["stats"]["theirsApplied"], 1);

    let merged = read_model(&merged_path);
    assert_eq!(transparency(&merged, "Q"), Some(0.1));
    assert_eq!(transparency(&merged, "R"), Some(0.9));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn json_flag_requires_list_on_resolve() {
    // `--json` alone is a usage error, not silent text output.
    let output = run(&["resolve", "whatever.rbxm", "--json"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--list"),
        "{output:?}"
    );
}
