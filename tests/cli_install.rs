//! `git rbx install` and the git integration it enables, end to end against
//! a real git: driver config, managed .gitattributes block, idempotency,
//! `--check` drift detection, the pre-commit hook, and actual `git merge`
//! invocations that reach the driver through git's extensionless temp
//! files (`%O %A %B`) plus the `--path %P` hint.

use rbx_dom_weak::{InstanceBuilder, WeakDom};
use rbx_types::Variant;
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

fn map(q: f32, r: f32) -> WeakDom {
    WeakDom::new(
        folder("root")
            .with_child(part_with("Q", q))
            .with_child(part_with("R", r)),
    )
}

fn write_model(path: &Path, dom: &WeakDom) {
    let file = std::fs::File::create(path).unwrap();
    rbx_binary::to_writer(file, dom, dom.root().children()).unwrap();
}

fn transparency(path: &Path, name: &str) -> Option<f32> {
    let dom: WeakDom = rbx_binary::from_reader(std::fs::File::open(path).unwrap()).unwrap();
    dom.descendants().find(|i| i.name == name).and_then(|i| {
        match i.properties.get(&"Transparency".into()) {
            Some(Variant::Float32(t)) => Some(*t),
            _ => None,
        }
    })
}

/// A throwaway repository, fully isolated from the developer's own git
/// configuration (global config, system config, hooks path, editor).
struct Repo {
    dir: PathBuf,
}

impl Repo {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "git-rbx-install-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = Self { dir };
        repo.git(&["-c", "init.defaultBranch=main", "init", "-q"]);
        repo.git(&["config", "user.name", "Test"]);
        repo.git(&["config", "user.email", "test@example.com"]);
        repo
    }

    fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command
            .current_dir(&self.dir)
            .env("GIT_CONFIG_GLOBAL", self.dir.join("isolated-global.gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_EDITOR", "true")
            .env("HOME", &self.dir);
        command
    }

    fn git_output(&self, args: &[&str]) -> Output {
        self.command("git").args(args).output().expect("git runs")
    }

    fn git(&self, args: &[&str]) -> String {
        let output = self.git_output(args);
        assert!(
            output.status.success(),
            "git {} failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim_end().to_string()
    }

    fn rbx_output(&self, args: &[&str]) -> Output {
        self.command(BIN).args(args).output().expect("git-rbx runs")
    }

    fn rbx(&self, args: &[&str]) -> Output {
        let output = self.rbx_output(args);
        assert!(
            output.status.success(),
            "git-rbx {} failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn install(&self, extra: &[&str]) {
        let mut args = vec!["install", "--local", "--exe", BIN];
        args.extend_from_slice(extra);
        self.rbx(&args);
    }

    fn commit_map(&self, dom: &WeakDom, message: &str) {
        write_model(&self.dir.join("map.rbxm"), dom);
        self.git(&["add", "map.rbxm"]);
        self.git(&["commit", "-q", "-m", message]);
    }

    fn attributes(&self) -> String {
        std::fs::read_to_string(self.dir.join(".gitattributes")).unwrap_or_default()
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn install_writes_driver_config_and_managed_attributes_idempotently() {
    let repo = Repo::new("install");
    // Pre-existing user content must survive untouched.
    std::fs::write(repo.dir.join(".gitattributes"), "*.png binary\n").unwrap();

    repo.install(&[]);

    assert_eq!(
        repo.git(&["config", "--local", "--get", "merge.rbx.driver"]),
        format!("{BIN} merge %O %A %B --path %P")
    );
    assert_eq!(
        repo.git(&["config", "--local", "--get", "merge.rbx.recursive"]),
        "binary"
    );
    assert_eq!(
        repo.git(&["config", "--local", "--get", "mergetool.rbx.cmd"]),
        format!("{BIN} resolve \"$MERGED\" --studio")
    );
    let attributes = repo.attributes();
    assert!(attributes.starts_with("*.png binary\n"), "{attributes}");
    for glob in ["*.rbxl", "*.rbxlx", "*.rbxm", "*.rbxmx"] {
        assert!(
            attributes.lines().any(|l| l.starts_with(glob) && l.contains("merge=rbx") && l.contains("-text")),
            "missing {glob} line in:\n{attributes}"
        );
    }

    // Idempotent: a second run changes nothing.
    repo.install(&[]);
    assert_eq!(repo.attributes(), attributes);

    // --check agrees.
    repo.rbx(&["install", "--local", "--exe", BIN, "--check"]);
}

#[test]
fn install_check_detects_drift() {
    let repo = Repo::new("drift");
    let output = repo.rbx_output(&["install", "--local", "--exe", BIN, "--check"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("MISSING"), "{stderr}");

    // Attributes present but driver config absent is the silent-failure
    // state (git quietly keeps "ours"); --check must flag it.
    repo.install(&[]);
    repo.git(&["config", "--local", "--unset", "merge.rbx.driver"]);
    let output = repo.rbx_output(&["install", "--local", "--exe", BIN, "--check"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("merge.rbx.driver"),
        "{output:?}"
    );
}

#[test]
fn install_local_outside_a_repository_is_an_error() {
    let dir = std::env::temp_dir().join(format!("git-rbx-norepo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let output = Command::new(BIN)
        .current_dir(&dir)
        .env("GIT_CONFIG_GLOBAL", dir.join("g"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(["install", "--local", "--exe", BIN])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let _ = std::fs::remove_dir_all(&dir);
}

/// The whole point: `git merge` on a branch that edited a Roblox file
/// reaches the driver through git's temp files and composes cleanly.
/// Git's own binary handling would refuse to merge these at all.
#[test]
fn git_merge_composes_non_overlapping_edits_through_the_driver() {
    let repo = Repo::new("merge-clean");
    repo.install(&[]);
    repo.commit_map(&map(0.0, 0.0), "base");
    repo.git(&["add", ".gitattributes"]);
    repo.git(&["commit", "-q", "-m", "attributes"]);

    repo.git(&["checkout", "-q", "-b", "feature"]);
    repo.commit_map(&map(0.0, 0.9), "feature edits R");
    repo.git(&["checkout", "-q", "main"]);
    repo.commit_map(&map(0.1, 0.0), "main edits Q");

    let output = repo.git_output(&["merge", "--no-edit", "feature"]);
    assert!(
        output.status.success(),
        "git merge should compose via the driver:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let merged = repo.dir.join("map.rbxm");
    assert_eq!(transparency(&merged, "Q"), Some(0.1));
    assert_eq!(transparency(&merged, "R"), Some(0.9));
    assert_eq!(repo.git(&["status", "--porcelain"]), "", "merge committed cleanly");
    repo.rbx(&["check", "map.rbxm"]);
}

/// Conflicting edits: git reports the path unmerged, the worktree file
/// carries git-rbx's conflict state, the pre-commit hook blocks committing
/// it, and the CLI resolve loop gets the merge committed.
#[test]
fn git_merge_conflict_resolves_through_the_cli_with_hook_enforcement() {
    let repo = Repo::new("merge-conflict");
    repo.install(&["--hooks"]);
    repo.commit_map(&map(0.0, 0.0), "base");
    repo.git(&["add", ".gitattributes"]);
    repo.git(&["commit", "-q", "-m", "attributes"]);

    repo.git(&["checkout", "-q", "-b", "feature"]);
    repo.commit_map(&map(0.2, 0.0), "feature edits Q");
    repo.git(&["checkout", "-q", "main"]);
    repo.commit_map(&map(0.1, 0.0), "main edits Q differently");

    let output = repo.git_output(&["merge", "--no-edit", "feature"]);
    assert!(!output.status.success(), "conflict must fail the merge");
    assert_eq!(repo.git(&["status", "--porcelain"]), "UU map.rbxm");

    // The worktree file is the driver's %A output, moved into place by git.
    let file = repo.dir.join("map.rbxm");
    let output = repo.rbx_output(&["check", "map.rbxm", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let check: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(check["unresolvedCount"], 1);
    assert_eq!(transparency(&file, "Q"), Some(0.0), "conflicted target keeps base");

    // Hook: staging the still-conflicted file and committing is refused.
    repo.git(&["add", "map.rbxm"]);
    let output = repo.git_output(&["commit", "-q", "-m", "premature"]);
    assert!(!output.status.success(), "pre-commit hook must block");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unresolved merge conflict state"),
        "{output:?}"
    );

    // Resolve from the CLI exactly as an agent would, then commit.
    let output = repo.rbx(&["resolve", "map.rbxm", "--list", "--json"]);
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["conflicts"][0]["path"], "Q");
    let entry = report["conflicts"][0]["name"].as_str().unwrap().to_string();
    repo.rbx(&["resolve", "map.rbxm", "--take", "theirs", "--entry", &entry]);
    repo.rbx(&["resolve", "map.rbxm", "--finalize"]);
    repo.rbx(&["check", "map.rbxm"]);
    repo.git(&["add", "map.rbxm"]);
    repo.git(&["commit", "-q", "-m", "merge feature"]);

    assert_eq!(repo.git(&["status", "--porcelain"]), "");
    assert_eq!(transparency(&file, "Q"), Some(0.2), "theirs won");
    // And the merge commit really has two parents.
    let parents = repo.git(&["log", "-1", "--format=%P"]);
    assert_eq!(parents.split_whitespace().count(), 2, "{parents}");
}
