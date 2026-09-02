//! Shared scaffolding for CLI tests that drive the built `git-rbx` binary
//! against a real git repository.
#![allow(dead_code)]

use rbx_dom_weak::{InstanceBuilder, WeakDom};
use rbx_types::Variant;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub const BIN: &str = env!("CARGO_BIN_EXE_git-rbx");

pub fn folder(name: &str) -> InstanceBuilder {
    InstanceBuilder::new("Folder").with_name(name)
}

pub fn part_with(name: &str, transparency: f32) -> InstanceBuilder {
    InstanceBuilder::new("Part")
        .with_name(name)
        .with_property("Anchored", Variant::Bool(true))
        .with_property("Transparency", Variant::Float32(transparency))
}

/// root { Q(q), R(r) }
pub fn map(q: f32, r: f32) -> WeakDom {
    WeakDom::new(
        folder("root")
            .with_child(part_with("Q", q))
            .with_child(part_with("R", r)),
    )
}

pub fn write_model(path: &Path, dom: &WeakDom) {
    let file = std::fs::File::create(path).unwrap();
    rbx_binary::to_writer(file, dom, dom.root().children()).unwrap();
}

pub fn transparency(path: &Path, name: &str) -> Option<f32> {
    let bytes = std::fs::read(path).unwrap();
    let dom: WeakDom = rbx_binary::from_reader(bytes.as_slice()).unwrap_or_else(|e| {
        panic!(
            "{} is not a Roblox binary ({e}); {} bytes, head: {:?}",
            path.display(),
            bytes.len(),
            String::from_utf8_lossy(&bytes[..bytes.len().min(160)])
        )
    });
    dom.descendants().find(|i| i.name == name).and_then(|i| {
        match i.properties.get(&"Transparency".into()) {
            Some(Variant::Float32(t)) => Some(*t),
            _ => None,
        }
    })
}

/// A throwaway repository, fully isolated from the developer's own git
/// configuration (global config, system config, hooks path, editor).
pub struct Repo {
    pub dir: PathBuf,
}

impl Repo {
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("git-rbx-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = Self { dir };
        repo.git(&["-c", "init.defaultBranch=main", "init", "-q"]);
        repo.git(&["config", "user.name", "Test"]);
        repo.git(&["config", "user.email", "test@example.com"]);
        repo
    }

    pub fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command
            .current_dir(&self.dir)
            .env("GIT_CONFIG_GLOBAL", self.dir.join("isolated-global.gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_EDITOR", "true")
            .env("HOME", &self.dir);
        command
    }

    pub fn git_output(&self, args: &[&str]) -> Output {
        self.command("git").args(args).output().expect("git runs")
    }

    pub fn git(&self, args: &[&str]) -> String {
        let output = self.git_output(args);
        assert!(
            output.status.success(),
            "git {} failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim_end().to_string()
    }

    pub fn rbx_output(&self, args: &[&str]) -> Output {
        self.command(BIN).args(args).output().expect("git-rbx runs")
    }

    pub fn rbx(&self, args: &[&str]) -> Output {
        let output = self.rbx_output(args);
        assert!(
            output.status.success(),
            "git-rbx {} failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    /// `git rbx install --local` with the test binary embedded.
    pub fn install(&self, extra: &[&str]) {
        let mut args = vec!["install", "--local", "--exe", BIN];
        args.extend_from_slice(extra);
        self.rbx(&args);
    }

    pub fn commit_map(&self, dom: &WeakDom, message: &str) {
        write_model(&self.dir.join("map.rbxm"), dom);
        self.git(&["add", "map.rbxm"]);
        self.git(&["commit", "-q", "-m", message]);
    }

    pub fn attributes(&self) -> String {
        std::fs::read_to_string(self.dir.join(".gitattributes")).unwrap_or_default()
    }

    /// The raw blob git stores for a path at a revision.
    pub fn blob(&self, rev_path: &str) -> Vec<u8> {
        let output = self.git_output(&["cat-file", "-p", rev_path]);
        assert!(output.status.success(), "{output:?}");
        output.stdout
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
