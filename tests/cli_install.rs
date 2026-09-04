//! `git rbx install` and the git integration it enables, end to end against
//! a real git: driver config, managed .gitattributes block, idempotency,
//! `--check` drift detection, the pre-commit hook, and actual `git merge`
//! invocations that reach the driver through git's extensionless temp
//! files (`%O %A %B`) plus the `--path %P` hint.

mod common;
use common::*;

use std::process::Command;

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
    assert_eq!(
        repo.git(&["config", "--local", "--get", "difftool.rbx.cmd"]),
        format!("{BIN} diff \"$LOCAL\" \"$REMOTE\" --studio")
    );
    let attributes = repo.attributes();
    assert!(attributes.starts_with("*.png binary\n"), "{attributes}");
    for glob in ["*.rbxl", "*.rbxlx", "*.rbxm", "*.rbxmx"] {
        assert!(
            attributes.lines().any(|l| l.starts_with(glob)
                && l.contains("merge=rbx")
                && l.contains("diff=rbx")
                && l.contains("-text")),
            "missing {glob} line in:\n{attributes}"
        );
    }
    assert_eq!(
        repo.git(&["config", "--local", "--get", "diff.rbx.command"]),
        format!("{BIN} git-diff")
    );

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

/// A later `git lfs track` line for the same glob would win under git's
/// last-match rule; re-running install must move the managed block back to
/// the end, and --check must notice when it isn't last.
#[test]
fn managed_block_stays_last_so_it_overrides_later_lfs_lines() {
    let repo = Repo::new("block-order");
    repo.install(&[]);
    let mut attributes = repo.attributes();
    attributes.push_str("*.rbxm filter=lfs diff=lfs merge=lfs -text\n");
    std::fs::write(repo.dir.join(".gitattributes"), &attributes).unwrap();

    let output = repo.rbx_output(&["install", "--local", "--exe", BIN, "--check"]);
    assert_eq!(output.status.code(), Some(1), "block no longer last: {output:?}");

    repo.install(&[]);
    let attributes = repo.attributes();
    let lfs_line = attributes.find("filter=lfs").unwrap();
    let block = attributes.find("# >>> git-rbx").unwrap();
    assert!(block > lfs_line, "block must follow the lfs line:\n{attributes}");
    assert_eq!(attributes.matches("# >>> git-rbx").count(), 1);
    repo.rbx(&["install", "--local", "--exe", BIN, "--check"]);
}

/// `git diff` on a modified Roblox file goes through the external-diff
/// shim, so the output is semantic instead of "Binary files differ".
#[test]
fn git_diff_shows_semantic_changes_through_the_shim() {
    let repo = Repo::new("ext-diff");
    repo.install(&[]);
    repo.git(&["add", ".gitattributes"]);
    repo.git(&["commit", "-q", "-m", "attributes"]);
    repo.commit_map(&map(0.0, 0.0), "base");

    // Worktree vs index: new side is the real file, old side a temp blob.
    write_model(&repo.dir.join("map.rbxm"), &map(0.4, 0.0));
    let diff = repo.git(&["diff"]);
    assert!(diff.contains("diff --rbx a/map.rbxm b/map.rbxm"), "{diff}");
    assert!(diff.contains("Transparency"), "{diff}");
    assert!(!diff.contains("Binary files"), "{diff}");

    // Commit-to-commit (`git log -p`/`git show` need --ext-diff).
    repo.git(&["commit", "-q", "-am", "edit Q"]);
    let shown = repo.git(&["show", "--ext-diff", "--format=", "HEAD"]);
    assert!(shown.contains("Transparency"), "{shown}");
    let summary = repo.git(&["-c", &format!("diff.rbx.command={BIN} git-diff --summary-only"), "show", "--ext-diff", "--format=", "HEAD"]);
    assert!(summary.contains("modified"), "{summary}");

    // Added and deleted files (a /dev/null side) render too.
    std::fs::remove_file(repo.dir.join("map.rbxm")).unwrap();
    let removed = repo.git(&["diff"]);
    assert!(removed.contains("deleted file"), "{removed}");
}
