//! File-encoding resolution at the CLI boundary. Git merge drivers receive
//! extensionless temp copies (`.merge_file_XXXXXX`), so the tool must work
//! from the real path hint (`--path %P`) or from file content alone.

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

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rbx-diff-cli-formats-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_binary(path: &Path, dom: &WeakDom) {
    let file = std::fs::File::create(path).unwrap();
    rbx_binary::to_writer(file, dom, dom.root().children()).unwrap();
}

fn write_xml(path: &Path, dom: &WeakDom) {
    let file = std::fs::File::create(path).unwrap();
    rbx_xml::to_writer_default(file, dom, dom.root().children()).unwrap();
}

fn run(args: &[&str]) -> Output {
    Command::new(BIN).args(args).output().expect("binary runs")
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}):\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn transparency(dom: &WeakDom, name: &str) -> Option<f32> {
    dom.descendants().find(|i| i.name == name).and_then(|i| {
        match i.properties.get(&"Transparency".into()) {
            Some(Variant::Float32(t)) => Some(*t),
            _ => None,
        }
    })
}

/// base/ours/theirs that merge cleanly: Q edited by ours, R by theirs.
fn clean_trio() -> (WeakDom, WeakDom, WeakDom) {
    let dom = |q: f32, r: f32| {
        WeakDom::new(
            folder("root")
                .with_child(part_with("Q", q))
                .with_child(part_with("R", r)),
        )
    };
    (dom(0.0, 0.0), dom(0.1, 0.0), dom(0.0, 0.9))
}

/// The exact shape git presents: three extensionless temp files in the
/// repo root, the result expected back in the second one (%A).
fn git_style_inputs(dir: &Path, write: fn(&Path, &WeakDom)) -> (PathBuf, PathBuf, PathBuf) {
    let (base, ours, theirs) = clean_trio();
    let o = dir.join(".merge_file_O1x9Qa");
    let a = dir.join(".merge_file_A7kL2m");
    let b = dir.join(".merge_file_Bz3pT4");
    write(&o, &base);
    write(&a, &ours);
    write(&b, &theirs);
    (o, a, b)
}

#[test]
fn git_temp_files_merge_with_real_path_hint() {
    let dir = scratch_dir("hint");
    let (o, a, b) = git_style_inputs(&dir, write_binary);

    let output = run(&[
        "merge",
        o.to_str().unwrap(),
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        "--path",
        "Workspace/map.rbxm",
        "--json",
    ]);
    assert!(output.status.success(), "{output:?}");
    let summary = stdout_json(&output);
    assert_eq!(summary["clean"], true);
    assert_eq!(summary["output"], a.to_str().unwrap(), "result lands in %A");
    assert_eq!(summary["path"], "Workspace/map.rbxm");

    // %A now holds the merged binary — what git will move into place.
    let merged = rbx_binary::from_reader(std::fs::File::open(&a).unwrap()).unwrap();
    assert_eq!(transparency(&merged, "Q"), Some(0.1));
    assert_eq!(transparency(&merged, "R"), Some(0.9));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn extensionless_binary_inputs_are_sniffed() {
    let dir = scratch_dir("sniff-binary");
    let (o, a, b) = git_style_inputs(&dir, write_binary);

    let output = run(&[
        "merge",
        o.to_str().unwrap(),
        a.to_str().unwrap(),
        b.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{output:?}");
    let merged = rbx_binary::from_reader(std::fs::File::open(&a).unwrap()).unwrap();
    assert_eq!(transparency(&merged, "Q"), Some(0.1));
    assert_eq!(transparency(&merged, "R"), Some(0.9));

    // Every other command sniffs the same way.
    let output = run(&["check", a.to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    let output = run(&["diff", o.to_str().unwrap(), a.to_str().unwrap(), "--summary-only"]);
    assert!(output.status.success(), "{output:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn extensionless_xml_inputs_are_sniffed_and_encoding_preserved() {
    let dir = scratch_dir("sniff-xml");
    let (o, a, b) = git_style_inputs(&dir, write_xml);
    assert!(
        std::fs::read_to_string(&o).unwrap().starts_with("<roblox"),
        "precondition: XML fixture"
    );

    let output = run(&[
        "merge",
        o.to_str().unwrap(),
        a.to_str().unwrap(),
        b.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{output:?}");
    // With no hint and no extension, the result keeps the base's encoding.
    let text = std::fs::read_to_string(&a).unwrap();
    assert!(text.starts_with("<roblox"), "output should stay XML: {text:.40}");
    let merged = rbx_xml::from_reader_default(text.as_bytes()).unwrap();
    assert_eq!(transparency(&merged, "Q"), Some(0.1));
    assert_eq!(transparency(&merged, "R"), Some(0.9));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mismatched_real_path_hint_fails_instead_of_misparsing() {
    // XML temp inputs but the repository file is binary: git will move %A to
    // that path, so %A must be written in the real file's encoding.
    let dir = scratch_dir("hint-encoding");
    let (o, a, b) = git_style_inputs(&dir, write_xml);

    let output = run(&[
        "merge",
        o.to_str().unwrap(),
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        "--path",
        "map.rbxl",
    ]);
    // The hint says binary; the inputs are XML. The hint's extension wins
    // for reading too, so this must fail loudly rather than misparse.
    assert!(!output.status.success(), "{output:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unrecognizable_content_fails_with_guidance() {
    let dir = scratch_dir("garbage");
    let path = dir.join(".merge_file_nope");
    std::fs::write(&path, b"definitely not a roblox file").unwrap();

    let output = run(&["check", path.to_str().unwrap()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--path"), "should point at the fix: {stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}
