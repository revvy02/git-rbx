//! Git LFS: the driver and the diff shim receive pointer text for each
//! side and must resolve it, and a merge result must be written back as a
//! pointer so the file never silently leaves LFS. Skips when git-lfs is
//! not installed.

mod common;
use common::*;

const POINTER_PREFIX: &[u8] = b"version https://git-lfs.github.com/spec/v1";

fn lfs_available() -> bool {
    let ok = std::process::Command::new("git")
        .args(["lfs", "version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        eprintln!("git-lfs not installed; skipping");
    }
    ok
}

/// A repository with *.rbxm tracked by LFS and git-rbx installed after it,
/// so the managed block overrides the lfs merge=/diff= lines.
fn lfs_repo(name: &str) -> Repo {
    let repo = Repo::new(name);
    repo.git(&["lfs", "install", "--local"]);
    repo.git(&["lfs", "track", "*.rbxm"]);
    repo.install(&[]);
    repo.git(&["add", ".gitattributes"]);
    repo.git(&["commit", "-q", "-m", "attributes"]);
    repo
}

fn assert_pointer(blob: &[u8]) {
    assert!(
        blob.starts_with(POINTER_PREFIX),
        "blob must be an LFS pointer, got {} bytes starting {:?}",
        blob.len(),
        String::from_utf8_lossy(&blob[..blob.len().min(16)])
    );
}

/// `map` plus enough padding to exceed git-lfs's 1024-byte pointer cutoff:
/// `git lfs clean` behaves differently for content under that size, so a
/// realistic result must be larger to catch truncation.
fn big_map(q: f32, r: f32) -> rbx_dom_weak::WeakDom {
    let mut root = folder("root")
        .with_child(part_with("Q", q))
        .with_child(part_with("R", r));
    // rbx_binary LZ4-compresses chunks, so filler must be incompressible:
    // pseudo-random hex names (deterministic LCG) rather than padding.
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for index in 0..64 {
        let mut name = format!("Filler{index}_");
        for _ in 0..4 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            name.push_str(&format!("{:016x}", state));
        }
        root = root.with_child(part_with(&name, (index as f32) / 64.0));
    }
    rbx_dom_weak::WeakDom::new(root)
}

#[test]
fn lfs_tracked_files_merge_through_the_driver_and_stay_in_lfs() {
    if !lfs_available() {
        return;
    }
    let repo = lfs_repo("merge");
    repo.commit_map(&big_map(0.0, 0.0), "base");
    assert_pointer(&repo.blob("HEAD:map.rbxm"));

    repo.git(&["checkout", "-q", "-b", "feature"]);
    repo.commit_map(&big_map(0.0, 0.9), "feature edits R");
    repo.git(&["checkout", "-q", "main"]);
    repo.commit_map(&big_map(0.1, 0.0), "main edits Q");

    let output = repo.git_output(&["merge", "--no-edit", "feature"]);
    assert!(
        output.status.success(),
        "driver must resolve pointers and merge:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Worktree: real merged content (git smudged the pointer we wrote).
    let file = repo.dir.join("map.rbxm");
    assert_eq!(transparency(&file, "Q"), Some(0.1));
    assert_eq!(transparency(&file, "R"), Some(0.9));
    // Repository: still a pointer — the merge did not take the file out of LFS.
    assert_pointer(&repo.blob("HEAD:map.rbxm"));
    assert!(
        repo.git(&["lfs", "ls-files"]).contains("map.rbxm"),
        "merged file must remain LFS-tracked"
    );
    assert_eq!(repo.git(&["status", "--porcelain"]), "");
    // The stored object is the complete result, and the pointer's size
    // agrees with it (guards the `git lfs clean <path>` truncation quirk).
    let pointer = String::from_utf8(repo.blob("HEAD:map.rbxm")).unwrap();
    let size: u64 = pointer
        .lines()
        .find_map(|l| l.strip_prefix("size "))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(size, std::fs::metadata(&file).unwrap().len());
    assert!(size > 1024, "fixture must exceed the LFS pointer cutoff: {size}");
}

#[test]
fn lfs_conflict_resolves_from_the_worktree_and_recommits_as_pointer() {
    if !lfs_available() {
        return;
    }
    let repo = lfs_repo("conflict");
    repo.commit_map(&map(0.0, 0.0), "base");
    repo.git(&["checkout", "-q", "-b", "feature"]);
    repo.commit_map(&map(0.2, 0.0), "feature edits Q");
    repo.git(&["checkout", "-q", "main"]);
    repo.commit_map(&map(0.1, 0.0), "main edits Q differently");

    let output = repo.git_output(&["merge", "--no-edit", "feature"]);
    assert!(!output.status.success());
    assert_eq!(repo.git(&["status", "--porcelain"]), "UU map.rbxm");

    // The worktree holds the smudged conflict-stamped file.
    let file = repo.dir.join("map.rbxm");
    let output = repo.rbx_output(&["check", "map.rbxm"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(transparency(&file, "Q"), Some(0.0));

    repo.rbx(&["resolve", "map.rbxm", "--take", "theirs", "--all"]);
    repo.rbx(&["resolve", "map.rbxm", "--finalize"]);
    repo.rbx(&["check", "map.rbxm"]);
    repo.git(&["add", "map.rbxm"]);
    repo.git(&["commit", "-q", "-m", "merge feature"]);

    assert_eq!(transparency(&file, "Q"), Some(0.2));
    assert_pointer(&repo.blob("HEAD:map.rbxm"));
    assert_eq!(
        repo.git(&["log", "-1", "--format=%P"]).split_whitespace().count(),
        2
    );
}

#[test]
fn git_diff_resolves_pointer_sides() {
    if !lfs_available() {
        return;
    }
    let repo = lfs_repo("diff");
    repo.commit_map(&map(0.0, 0.0), "base");
    write_model(&repo.dir.join("map.rbxm"), &map(0.4, 0.0));
    // Old side arrives as the index blob (a pointer); new side is the file.
    let diff = repo.git(&["diff"]);
    assert!(diff.contains("Transparency"), "{diff}");
    repo.git(&["commit", "-q", "-am", "edit"]);
    // Both sides pointers.
    let shown = repo.git(&["show", "--ext-diff", "--format=", "HEAD"]);
    assert!(shown.contains("Transparency"), "{shown}");
}

/// A skip-smudge checkout (GIT_LFS_SKIP_SMUDGE=1) leaves pointer text in a
/// file that still has its .rbxm extension. Pointer detection must come
/// before the extension is trusted.
#[test]
fn pointer_text_in_a_named_file_is_resolved() {
    if !lfs_available() {
        return;
    }
    let repo = lfs_repo("skip-smudge");
    repo.commit_map(&map(0.3, 0.0), "base");
    let pointer = repo.blob("HEAD:map.rbxm");
    let file = repo.dir.join("map.rbxm");
    std::fs::write(&file, &pointer).unwrap();

    repo.rbx(&["check", "map.rbxm"]);
    let output = repo.rbx(&["diff", "map.rbxm", "map.rbxm", "--summary-only"]);
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("No differences"),
        "{output:?}"
    );
}

/// `changes` reads blobs through `git cat-file`, which yields pointers for
/// LFS-tracked files; both sides must be resolved.
#[test]
fn changes_resolves_lfs_pointer_blobs() {
    if !lfs_available() {
        return;
    }
    let repo = lfs_repo("changes");
    repo.commit_map(&map(0.0, 0.0), "base");
    repo.commit_map(&map(0.6, 0.0), "edit");
    assert_pointer(&repo.blob("HEAD:map.rbxm"));
    let output = repo.rbx(&["changes", "HEAD~1", "HEAD"]);
    let md = String::from_utf8(output.stdout).unwrap();
    assert!(md.contains("| `Q` | Transparency | `0` | `0.6` |"), "{md}");
}
