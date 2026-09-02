//! Git LFS pointer handling at the CLI boundary.
//!
//! LFS is a clean/smudge filter: the repository stores small pointer files
//! and the worktree holds real content. Git does NOT run those filters for
//! merge drivers or external diff commands — they receive the pointer text
//! for each side — and whatever a merge driver writes to `%A` is stored as
//! the result blob verbatim. So a driver that ignores LFS either fails to
//! parse a pointer, or worse, writes real content into the object database
//! and silently takes the file out of LFS. Reading resolves pointers through
//! `git lfs smudge`; writing back where a pointer was found goes through
//! `git lfs clean`, which stores the object locally and returns its pointer.

use anyhow::{bail, Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

const POINTER_VERSION: &[u8] = b"version https://git-lfs.github.com/spec/v1";
const LEGACY_POINTER_VERSION: &[u8] = b"version https://hawser.github.com/spec/v1";
/// Per the LFS spec a pointer is a small text file; the leading bytes of
/// any real Roblox file are `<roblox` and can never start like a pointer.
const MAX_POINTER_BYTES: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pointer {
    pub oid: String,
    pub size: u64,
}

pub fn parse_pointer(bytes: &[u8]) -> Option<Pointer> {
    if bytes.len() > MAX_POINTER_BYTES
        || !(bytes.starts_with(POINTER_VERSION) || bytes.starts_with(LEGACY_POINTER_VERSION))
    {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    let mut oid = None;
    let mut size = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("oid sha256:") {
            oid = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("size ") {
            size = value.trim().parse().ok();
        }
    }
    Some(Pointer {
        oid: oid?,
        size: size?,
    })
}

pub fn is_pointer(bytes: &[u8]) -> bool {
    parse_pointer(bytes).is_some()
}

/// Resolve a pointer to the content it names, downloading it from the LFS
/// server when it is not in the local object store.
pub fn smudge(pointer: &[u8], path: &str) -> Result<Vec<u8>> {
    run_lfs("smudge", Some(path), pointer)
}

/// Store content in the local LFS object store and return its pointer.
///
/// Deliberately passes NO path: `git lfs clean <path>` stats that path and,
/// when the file there is smaller than its 1024-byte pointer cutoff, reads
/// only 1024 bytes of stdin and stores a truncated object with a wrong
/// size (git-lfs 3.7). Inside a merge driver the path would name the OLD
/// worktree file, whose size is unrelated to the result being stored.
pub fn clean(content: &[u8]) -> Result<Vec<u8>> {
    run_lfs("clean", None, content)
}

fn run_lfs(subcommand: &str, path: Option<&str>, input: &[u8]) -> Result<Vec<u8>> {
    let mut child = Command::new("git")
        .args(["lfs", subcommand])
        .args(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("running `git lfs` (is git-lfs installed and on PATH?)")?;
    // Feed stdin from a thread so a large payload can never deadlock
    // against the child's output pipe.
    let mut stdin = child.stdin.take().expect("stdin is piped");
    let payload = input.to_vec();
    let writer = std::thread::spawn(move || stdin.write_all(&payload));
    let output = child.wait_with_output()?;
    writer
        .join()
        .expect("stdin writer thread panicked")
        .with_context(|| format!("writing to `git lfs {subcommand}`"))?;
    if !output.status.success() {
        bail!(
            "`git lfs {subcommand} {}` failed: {}",
            path.unwrap_or(""),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    const POINTER: &str = "version https://git-lfs.github.com/spec/v1\n\
        oid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\n\
        size 12345\n";

    #[test]
    fn parses_a_pointer() {
        let pointer = parse_pointer(POINTER.as_bytes()).unwrap();
        assert_eq!(pointer.size, 12345);
        assert!(pointer.oid.starts_with("4d7a2146"));
    }

    #[test]
    fn rejects_roblox_content_and_oversized_text() {
        assert!(!is_pointer(b"<roblox!\x89\xff\r\n\x1a\n"));
        assert!(!is_pointer(b"<roblox xmlns=\"...\">"));
        assert!(!is_pointer(b"version https://git-lfs.github.com/spec/v1\n"));
        let padded = format!("{POINTER}{}", "x".repeat(MAX_POINTER_BYTES));
        assert!(!is_pointer(padded.as_bytes()));
    }
}
