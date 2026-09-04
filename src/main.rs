//! git-rbx: semantic diff, three-way merge, and conflict resolution for
//! Roblox place/model files, as a git extension (`git rbx <subcommand>`).

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::Path;
use std::time::Instant;
use tracing::info_span;
use tracing_subscriber::{fmt, EnvFilter};

mod git_lfs;

use git_rbx::output::{print_diff, render_markdown, OutputFormat, DEFAULT_MARKDOWN_ROWS};
use rbx_dom_weak::{InstanceBuilder, WeakDom};
use git_rbx::{
    apply_pivot_ops, apply_pivot_ops_to_compact_branch, conflict_report, detect_rigid_groups,
    diff_model_compact_doms_document, diff_model_compact_doms_with_config, finalize, find_container, list_entries, mark_entry,
    mark_entry_custom, merge_compact_doms, merge_compact_doms_with_matches_and_pivots,
    normalize_model_merge_compact_pivots, stamp_compact_conflicts, stamp_pivot_plan,
    stamp_rigid_groups, ConflictKind, ConflictReport, DiffConfig, DiffDom, MergeStats,
    PivotApplication, CONTAINER_NAME,
};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "git-rbx", bin_name = "git rbx")]
#[command(about = "Diff, merge, and resolve Roblox rbxm/rbxmx/rbxl/rbxlx files under git")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compare two files and show differences
    Diff {
        /// First (old) file
        old_file: String,

        /// Second (new) file
        new_file: String,

        /// Only show summary counts (same as --format summary)
        #[arg(long)]
        summary_only: bool,

        /// Output as JSON (same as --format json)
        #[arg(long)]
        json: bool,

        /// Output format; takes precedence over --json/--summary-only
        #[arg(long, value_enum)]
        format: Option<Format>,

        /// Markdown: rows per table before "… and N more"
        #[arg(long, default_value_t = DEFAULT_MARKDOWN_ROWS)]
        max_rows: usize,

        /// Show timing information
        #[arg(long, short = 't')]
        timing: bool,
    },
    /// Semantic diff of every Roblox file changed between two revisions —
    /// what `git diff --stat` cannot say about binaries. Rename-aware and
    /// Git LFS-aware; one section per file. Built for CI (step summaries,
    /// pull-request comments) and for `git rbx changes HEAD~1 HEAD` locally
    Changes {
        /// Base revision, or a `<base>..<head>` range
        base: String,

        /// Head revision (omit when BASE is a `..` range)
        head: Option<String>,

        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Markdown)]
        format: Format,

        /// Markdown: rows per table before "… and N more"
        #[arg(long, default_value_t = DEFAULT_MARKDOWN_ROWS)]
        max_rows: usize,
    },
    /// Three-way merge (git merge driver: git-rbx merge %O %A %B --path %P).
    /// Writes the merged result to OURS (or --output) and exits nonzero
    /// when conflicts remain (conflicted content keeps the base version).
    Merge {
        /// Common ancestor file (%O)
        base: String,

        /// Our side (%A) — also the default output path
        ours: String,

        /// Their side (%B)
        theirs: String,

        /// Write the merged result here instead of overwriting OURS
        #[arg(short, long)]
        output: Option<String>,

        /// Real repository path of the file being merged (git merge driver:
        /// %P). Git hands drivers extensionless temp copies; this decides the
        /// output encoding, model-vs-place behavior, and the path shown in
        /// hints. Without it the encoding is sniffed from file content
        #[arg(long, value_name = "PATH")]
        path: Option<String>,

        /// Print a machine-readable summary (stats + the conflict report of
        /// the written file) to stdout. Exit codes are unchanged
        #[arg(long)]
        json: bool,
    },
    /// Inspect and resolve conflicts stored in a merged file
    Resolve {
        /// The conflicted file written by `git rbx merge`
        file: String,

        /// List conflicts and their resolution state
        #[arg(long)]
        list: bool,

        /// With --list: print the full conflict report as JSON (competing
        /// values, exact per-side patches, groups) instead of text lines
        #[arg(long, requires = "list")]
        json: bool,

        /// Resolve toward this side: ours | theirs | custom (custom requires
        /// --entry and --value)
        #[arg(long, value_name = "SIDE")]
        take: Option<String>,

        /// Custom resolution value as plain JSON (number, string, bool, or
        /// array — coerced to the conflicted property's type), with
        /// --take custom
        #[arg(long)]
        value: Option<String>,

        /// Base path of the conflict(s) to resolve (with --take).
        /// May match several entries (e.g. two properties on one instance)
        #[arg(long)]
        path: Option<String>,

        /// Entry name (e.g. Conflict_2) — the unique key, from --list
        #[arg(long)]
        entry: Option<String>,

        /// Resolve every remaining conflict (with --take)
        #[arg(long)]
        all: bool,

        /// Apply all resolutions, strip conflict state, write the clean file
        #[arg(long)]
        finalize: bool,

        /// Resolve visually in Roblox Studio: opens the file with conflict
        /// highlights and an Ours/Theirs/Custom panel (needs `rodeo` on
        /// PATH). Exits 0 only when the session leaves the file clean
        #[arg(long, conflicts_with_all = ["list", "json", "take", "value", "path", "entry", "all", "finalize"])]
        studio: bool,

        /// Debug: auto-stage every conflict to this side and complete
        #[arg(long, hide = true, requires = "studio", value_name = "SIDE")]
        studio_auto: Option<String>,
    },
    /// Exit nonzero if the file contains unresolved merge conflict state
    Check {
        /// File to check
        file: String,

        /// Print `{"clean": bool, "unresolvedCount": n}` to stdout
        #[arg(long)]
        json: bool,
    },
    /// git external-diff entry point (`diff.rbx.command`, written by
    /// `install`). Git calls it as `<path> <old-file> <old-hex> <old-mode>
    /// <new-file> <new-hex> <new-mode>` and splices stdout into `git diff`
    /// and `git log -p --ext-diff` / `git show --ext-diff`
    #[command(name = "git-diff")]
    GitDiff {
        /// Only print summary counts
        #[arg(long)]
        summary_only: bool,

        /// Arguments exactly as git passes them
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 1..)]
        args: Vec<String>,
    },
    /// Configure git to merge Roblox files through git-rbx: writes the merge
    /// driver + mergetool config, and a managed block in the repository's
    /// .gitattributes routing *.rbxl/*.rbxlx/*.rbxm/*.rbxmx through it.
    /// Idempotent; re-run to update
    Install {
        /// Write config to ~/.gitconfig (default) so every repository can use
        /// it; each teammate runs this once per machine
        #[arg(long, conflicts_with = "local")]
        global: bool,

        /// Write config to this repository only
        #[arg(long)]
        local: bool,

        /// Only manage git config; leave .gitattributes alone
        #[arg(long)]
        no_attributes: bool,

        /// Also install a pre-commit hook that refuses to commit files still
        /// carrying merge conflict state (`git rbx check`)
        #[arg(long)]
        hooks: bool,

        /// Embed this executable path in the driver commands instead of
        /// relying on `git-rbx` being on PATH
        #[arg(long, value_name = "PATH")]
        exe: Option<String>,

        /// Report what is and isn't configured, change nothing; exit 1 on
        /// any drift (attributes present but driver missing is the classic
        /// silent failure — git quietly keeps "ours")
        #[arg(long)]
        check: bool,
    },
}

fn main() -> Result<()> {
    // Init tracing subscriber (controlled via RUST_LOG env var)
    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(fmt::format::FmtSpan::CLOSE)
        .with_target(true)
        .with_writer(std::io::stderr)
        .init();

    match Cli::parse().command {
        Command::Diff {
            old_file,
            new_file,
            summary_only,
            json,
            format,
            max_rows,
            timing,
        } => {
            let format = format.unwrap_or(if json {
                Format::Json
            } else if summary_only {
                Format::Summary
            } else {
                Format::Pretty
            });
            cmd_diff(&old_file, &new_file, format, max_rows, timing)
        }
        Command::Changes {
            base,
            head,
            format,
            max_rows,
        } => {
            let (base, head) = match (head, base.split_once("..")) {
                (Some(head), _) => (base.clone(), head),
                (None, Some((range_base, range_head))) if !range_head.is_empty() => {
                    (range_base.to_string(), range_head.to_string())
                }
                (None, _) => bail!("changes needs <base> <head> or <base>..<head>"),
            };
            cmd_changes(&base, &head, format, max_rows)
        }
        Command::Merge {
            base,
            ours,
            theirs,
            output,
            path,
            json,
        } => cmd_merge(
            &base,
            &ours,
            &theirs,
            output.as_deref(),
            path.as_deref(),
            json,
        ),
        Command::Resolve {
            file,
            list,
            json,
            take,
            value,
            path,
            entry,
            all,
            finalize,
            studio,
            studio_auto,
        } => {
            if studio {
                cmd_resolve_studio(&file, studio_auto.as_deref())
            } else {
                cmd_resolve(
                    &file,
                    list,
                    json,
                    take.as_deref(),
                    value.as_deref(),
                    path.as_deref(),
                    entry.as_deref(),
                    all,
                    finalize,
                )
            }
        }
        Command::Check { file, json } => cmd_check(&file, json),
        Command::GitDiff { summary_only, args } => cmd_git_diff(summary_only, &args),
        Command::Install {
            global,
            local,
            no_attributes,
            hooks,
            exe,
            check,
        } => cmd_install(InstallOptions {
            scope: if local {
                ConfigScope::Local
            } else {
                // --global is also the default; the flag exists for
                // explicitness in docs and scripts
                let _ = global;
                ConfigScope::Global
            },
            attributes: !no_attributes,
            hooks,
            exe,
            check,
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    Pretty,
    Summary,
    Json,
    Markdown,
}

impl From<Format> for OutputFormat {
    fn from(format: Format) -> Self {
        match format {
            Format::Pretty => OutputFormat::Pretty,
            Format::Summary => OutputFormat::Summary,
            Format::Json => OutputFormat::Json,
            Format::Markdown => OutputFormat::Markdown,
        }
    }
}

fn cmd_diff(
    old_file: &str,
    new_file: &str,
    format: Format,
    max_rows: usize,
    timing: bool,
) -> Result<()> {
    let total_start = Instant::now();

    let load_start = Instant::now();
    let old_dom = {
        let _span = info_span!("load_old_file", file = %old_file).entered();
        load_diff_file(old_file, None)?.0
    };
    let old_load_time = load_start.elapsed();

    let load_start = Instant::now();
    let mut new_dom = {
        let _span = info_span!("load_new_file", file = %new_file).entered();
        load_diff_file(new_file, None)?.0
    };
    let new_load_time = load_start.elapsed();

    let diff_start = Instant::now();
    let pivot_stats = if format == Format::Markdown {
        let (diffs, pivots) =
            diff_model_compact_doms_with_config(&old_dom, &mut new_dom, &DiffConfig::default());
        print!("{}", render_markdown(&diffs, max_rows));
        pivots.as_ref().map(|p| (p.pivots.len(), p.detected))
    } else if format == Format::Json {
        let document =
            diff_model_compact_doms_document(&old_dom, &mut new_dom, &DiffConfig::default());
        println!("{}", serde_json::to_string_pretty(&document)?);
        Some((document.pivots.len(), 0))
    } else {
        diff_and_print(&old_dom, &mut new_dom, format.into())
    };
    let diff_time = diff_start.elapsed();

    let total_time = total_start.elapsed();

    if timing {
        eprintln!();
        eprintln!("Timing:");
        eprintln!("  Load old file: {:?}", old_load_time);
        eprintln!("  Load new file: {:?}", new_load_time);
        eprintln!(
            "  Diff computation (includes lazy hashing): {:?}",
            diff_time
        );
        if let Some((pivot_count, boundary_count)) = pivot_stats {
            eprintln!(
                "  Pivot factoring: {} pivot(s) from {} boundaries",
                pivot_count, boundary_count
            );
        }
        eprintln!("  Total: {:?}", total_time);
    }

    Ok(())
}

/// Diff two loaded DOMs and print in `format`; returns pivot-factoring
/// stats when any pivots were inferred.
fn diff_and_print(
    old_dom: &DiffDom,
    new_dom: &mut DiffDom,
    format: OutputFormat,
) -> Option<(usize, usize)> {
    let config = DiffConfig::default();
    // Unlike the old root-only model normalization, hierarchical framing
    // reports every inferred movement explicitly. It is therefore safe and
    // useful for place files too: authored placement remains visible as one
    // Pivoted entry instead of thousands of descendant CFrame changes.
    let (diffs, pivots) = diff_model_compact_doms_with_config(old_dom, new_dom, &config);
    print_diff(&diffs, format);
    pivots
        .as_ref()
        .map(|pivots| (pivots.pivots.len(), pivots.detected))
}

/// External diff for git. Git's temp copies are extensionless, so the real
/// path (the first argument) is the format hint; `/dev/null` stands for a
/// missing side (added or deleted file). Stdout is the whole diff git shows
/// for this path — git prints no header of its own for external diffs.
fn cmd_git_diff(summary_only: bool, args: &[String]) -> Result<()> {
    // One argument means an unmerged path.
    if args.len() == 1 {
        println!("* Unmerged path {}", args[0]);
        return Ok(());
    }
    if args.len() < 7 {
        bail!(
            "git-diff expects git's external-diff arguments \
             (<path> <old-file> <old-hex> <old-mode> <new-file> <new-hex> <new-mode>), got {}",
            args.len()
        );
    }
    let path = args[0].as_str();
    let old_file = args[1].as_str();
    let new_file = args[4].as_str();
    // Renames/copies append the new path (and a similarity note).
    let new_path = args.get(7).map(String::as_str).unwrap_or(path);

    let load_side = |file: &str| -> Result<DiffDom> {
        if file == "/dev/null" {
            return Ok(DiffDom::from_weak_dom_owned(WeakDom::new(
                InstanceBuilder::new("DataModel"),
            )));
        }
        Ok(load_diff_file(file, Some(path))?.0)
    };
    let old_dom = load_side(old_file)?;
    let mut new_dom = load_side(new_file)?;

    println!("diff --rbx a/{path} b/{new_path}");
    if old_file == "/dev/null" {
        println!("new file");
    } else if new_file == "/dev/null" {
        println!("deleted file");
    }
    let format = if summary_only {
        OutputFormat::Summary
    } else {
        OutputFormat::Pretty
    };
    diff_and_print(&old_dom, &mut new_dom, format);
    Ok(())
}

/// One Roblox file changed between two revisions.
struct ChangedFile {
    status: char,
    old_path: Option<String>,
    new_path: Option<String>,
}

impl ChangedFile {
    fn display_path(&self) -> &str {
        self.new_path
            .as_deref()
            .or(self.old_path.as_deref())
            .unwrap_or("?")
    }
}

/// `git diff --name-status -z -M` restricted to Roblox files. NUL-separated:
/// status, path, and for renames/copies a second path.
fn changed_roblox_files(base: &str, head: &str) -> Result<Vec<ChangedFile>> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-status", "-z", "-M", base, head, "--"])
        .args(ROBLOX_GLOBS)
        .output()
        .context("running git diff")?;
    if !output.status.success() {
        bail!(
            "git diff {base} {head} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut fields = text.split('\0').filter(|f| !f.is_empty());
    let mut files = Vec::new();
    while let Some(status) = fields.next() {
        let kind = status.chars().next().unwrap_or('?');
        let first = fields.next().map(str::to_string);
        let file = match kind {
            'A' => ChangedFile {
                status: kind,
                old_path: None,
                new_path: first,
            },
            'D' => ChangedFile {
                status: kind,
                old_path: first,
                new_path: None,
            },
            'R' | 'C' => ChangedFile {
                status: kind,
                old_path: first,
                new_path: fields.next().map(str::to_string),
            },
            _ => ChangedFile {
                status: kind,
                old_path: first.clone(),
                new_path: first,
            },
        };
        files.push(file);
    }
    Ok(files)
}

/// Load `<rev>:<path>` as a compact DOM; an absent side is an empty DOM.
fn load_revision_side(rev: &str, path: Option<&str>) -> Result<DiffDom> {
    let Some(path) = path else {
        return Ok(DiffDom::from_weak_dom_owned(WeakDom::new(
            InstanceBuilder::new("DataModel"),
        )));
    };
    let spec = format!("{rev}:{path}");
    let output = std::process::Command::new("git")
        .args(["cat-file", "blob", &spec])
        .output()
        .context("running git cat-file")?;
    if !output.status.success() {
        bail!(
            "git cat-file {spec} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let (bytes, source) = source_from_bytes(&spec, output.stdout, Some(path))?;
    diff_dom_from_bytes(&bytes, source.format)
}

fn cmd_changes(base: &str, head: &str, format: Format, max_rows: usize) -> Result<()> {
    let files = changed_roblox_files(base, head)?;
    let config = DiffConfig::default();

    if files.is_empty() {
        match format {
            Format::Json => println!("[]"),
            Format::Markdown => println!("_No Roblox files changed._"),
            _ => println!("No Roblox files changed between {base} and {head}."),
        }
        return Ok(());
    }

    let mut json_files = Vec::new();
    for file in &files {
        let old_dom = load_revision_side(base, file.old_path.as_deref())?;
        let mut new_dom = load_revision_side(head, file.new_path.as_deref())?;
        if format == Format::Json {
            let document = diff_model_compact_doms_document(&old_dom, &mut new_dom, &config);
            json_files.push(serde_json::json!({
                "path": file.display_path(),
                "status": file.status.to_string(),
                "oldPath": file.old_path,
                "counts": document.counts,
                "diff": document,
            }));
            continue;
        }
        let (diffs, _) = diff_model_compact_doms_with_config(&old_dom, &mut new_dom, &config);
        let status_note = match file.status {
            'A' => " (added)".to_string(),
            'D' => " (deleted)".to_string(),
            'R' => format!(" (renamed from `{}`)", file.old_path.as_deref().unwrap_or("?")),
            'C' => format!(" (copied from `{}`)", file.old_path.as_deref().unwrap_or("?")),
            _ => String::new(),
        };
        match format {
            Format::Markdown => {
                println!("### `{}`{status_note}\n", file.display_path());
                print!("{}", render_markdown(&diffs, max_rows));
            }
            Format::Json => unreachable!("handled above"),
            Format::Pretty | Format::Summary => {
                println!("== {}{} ==", file.display_path(), status_note);
                print_diff(&diffs, format.into());
                println!();
            }
        }
    }
    if format == Format::Json {
        println!("{}", serde_json::to_string_pretty(&json_files)?);
    }
    Ok(())
}

/// `merge --json` payload: merge statistics plus the conflict report of the
/// file that was just written — the same report `resolve --list --json`
/// produces, so an agent driving `git merge` sees exactly what it will
/// resolve, without a second invocation.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MergeSummary<'a> {
    /// Where the result was written (under git, the %A temp file).
    output: &'a str,
    /// The real repository path, when the caller supplied --path.
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<&'a str>,
    /// The inputs were Git LFS pointers; the result was written back as a
    /// pointer (content stored in the local LFS object store).
    lfs: bool,
    /// No conflict state was written; the file is ready to commit.
    clean: bool,
    stats: &'a MergeStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    pivots: Option<PivotSummary>,
    #[serde(flatten)]
    report: ConflictReport,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PivotSummary {
    factored: usize,
    ours_detected: usize,
    theirs_detected: usize,
}

#[allow(clippy::too_many_arguments)]
fn cmd_merge(
    base_path: &str,
    ours_path: &str,
    theirs_path: &str,
    output: Option<&str>,
    real_path: Option<&str>,
    json: bool,
) -> Result<()> {
    eprintln!("Loading base {}...", base_path);
    let (mut base, base_source) = load_file(base_path, real_path)?;
    eprintln!("Loading ours {}...", ours_path);
    let (mut ours, ours_source) = load_diff_file(ours_path, real_path)?;
    eprintln!("Loading theirs {}...", theirs_path);
    let (mut theirs, _) = load_diff_file(theirs_path, real_path)?;
    if ours_source.lfs_pointer {
        eprintln!("Inputs are Git LFS pointers; the result will be stored through LFS");
    }

    // Model vs place is a property of the real filename — git's temp copies
    // say nothing about it. Unknown falls back to place semantics (no model
    // pivot factoring), the conservative choice.
    let is_model = match real_path {
        Some(real_path) => is_model_asset_path(real_path),
        None => {
            is_model_asset_path(base_path)
                && is_model_asset_path(ours_path)
                && is_model_asset_path(theirs_path)
        }
    };
    let pivot_merge = if is_model {
        let pivots = normalize_model_merge_compact_pivots(&base, &mut ours, &mut theirs);
        if let Some(pivots) = &pivots {
            eprintln!(
                "Factored {} hierarchical pivot(s) ({} ours / {} theirs boundaries detected)",
                pivots.affected_boundaries(),
                pivots.ours_detected,
                pivots.theirs_detected,
            );
        }
        pivots
    } else {
        None
    };

    let config = DiffConfig::default();

    eprintln!("Merging...");
    let start = Instant::now();
    let result = if let Some(pivots) = &pivot_merge {
        merge_compact_doms_with_matches_and_pivots(
            &mut base,
            &ours,
            &theirs,
            &config,
            &pivots.ours_identity,
            &pivots.theirs_identity,
            pivots.ours_pivots(),
            pivots.theirs_pivots(),
        )
    } else {
        merge_compact_doms(&mut base, &ours, &theirs, &config)
    };
    let has_pivot_conflicts = result.conflicts.iter().any(|conflict| {
        matches!(conflict.kind, ConflictKind::Pivot { .. })
            || !conflict.ours.pivots.is_empty()
            || !conflict.theirs.pivots.is_empty()
    });
    // With no unresolved pivot decision, materialize the selected plan in
    // every canonical DOM before stamping. If any pivot conflicts, defer the
    // entire ordered plan to finalization/Studio preview.
    if !has_pivot_conflicts {
        apply_pivot_ops(&mut base, &result.pivots);
        apply_pivot_ops_to_compact_branch(&mut ours, &result.pivots, &result.ours_identity.matched);
        apply_pivot_ops_to_compact_branch(
            &mut theirs,
            &result.pivots,
            &result.theirs_identity.matched,
        );
    }
    eprintln!(
        "Merged in {:.2?}: {} ours + {} theirs ops applied, {} deduped, {} conflicts",
        start.elapsed(),
        result.stats.ours_applied,
        result.stats.theirs_applied,
        result.stats.deduped,
        result.stats.conflicted,
    );

    // Conflicted merges carry their conflict state in the file itself:
    // competing versions materialized as instances, targets tagged for
    // discovery. `git rbx resolve` (or Studio) consumes it.
    let mut groups = Vec::new();
    if !result.conflicts.is_empty() {
        groups = detect_rigid_groups(&base, &result.conflicts);
        stamp_compact_conflicts(&mut base, &ours, &theirs, &result);
        if has_pivot_conflicts {
            let applications: Vec<_> = result
                .pivots
                .iter()
                .map(|pivot| PivotApplication {
                    target_ref: pivot.target_ref,
                    path: pivot_merge
                        .as_ref()
                        .and_then(|pivots| pivots.path_for(pivot.target_ref))
                        .map(str::to_string)
                        .unwrap_or_else(|| pivot.target_ref.to_string()),
                    order: pivot.order,
                    parent_order: pivot.parent_order,
                    delta: pivot.delta,
                })
                .collect();
            stamp_pivot_plan(&mut base, &applications);
        }
        stamp_rigid_groups(&mut base, &groups);
    }

    let out_path = output.unwrap_or(ours_path);
    // The output encoding follows the real file when known, else the output
    // path's own extension, else whatever the base was encoded as.
    let out_format = real_path
        .and_then(format_from_extension)
        .or_else(|| format_from_extension(out_path))
        .unwrap_or(base_source.format);
    // %A is what git stores as the result blob: if ours arrived as a pointer
    // the file is LFS-tracked at this path, and the result must be too.
    let out_source = Source {
        format: out_format,
        lfs_pointer: ours_source.lfs_pointer,
    };
    save_file(out_path, &base, out_source)?;
    eprintln!("Wrote merged result to {}", out_path);
    // Under git, %A is moved to the real path once the driver exits — that is
    // the file a resolver will find.
    let display_path = real_path.unwrap_or(out_path);

    if json {
        let report = find_container(&base)
            .map(|container| conflict_report(&base, container))
            .unwrap_or_else(ConflictReport::empty);
        let summary = MergeSummary {
            output: out_path,
            path: real_path,
            lfs: out_source.lfs_pointer,
            clean: result.conflicts.is_empty(),
            stats: &result.stats,
            pivots: pivot_merge.as_ref().map(|pivots| PivotSummary {
                factored: pivots.affected_boundaries(),
                ours_detected: pivots.ours_detected,
                theirs_detected: pivots.theirs_detected,
            }),
            report,
        };
        println!("{}", serde_json::to_string_pretty(&summary)?);
    }

    if result.conflicts.is_empty() {
        return Ok(());
    }

    let grouped: std::collections::HashSet<usize> = groups
        .iter()
        .flat_map(|g| g.members.iter().copied())
        .collect();
    eprintln!();
    eprintln!("CONFLICTS ({}):", result.conflicts.len());
    for (index, group) in groups.iter().enumerate() {
        eprintln!(
            "  ! Group_{} {} — rigid move x{} (ours {} vs theirs {})",
            index + 1,
            group.path,
            group.members.len(),
            format_delta(&group.delta_ours),
            format_delta(&group.delta_theirs),
        );
    }
    for (index, conflict) in result.conflicts.iter().enumerate() {
        if grouped.contains(&index) {
            continue;
        }
        let kind = match &conflict.kind {
            ConflictKind::Property { name } => format!("property '{}'", name),
            ConflictKind::PropertyBundle { name, .. } => {
                format!("property bundle '{}'", name)
            }
            ConflictKind::DeleteVsEdit => "delete vs edit".to_string(),
            ConflictKind::ReparentTarget => "conflicting reparent destinations".to_string(),
            ConflictKind::Pivot { .. } => "pivot".to_string(),
        };
        eprintln!("  ! {} — {} (base content kept)", conflict.path, kind);
    }
    if !groups.is_empty() {
        eprintln!(
            "  ({} spatial conflicts folded into {} rigid groups)",
            grouped.len(),
            groups.len()
        );
    }

    eprintln!();
    eprintln!(
        "The conflicts are stored inside the file itself (a {CONTAINER_NAME} folder you will \
         see in Studio — leave it; resolving removes it). Resolve with:"
    );
    eprintln!("  git rbx resolve {} --list        (or --studio)", display_path);

    // Nonzero exit tells git the merge needs manual resolution
    std::process::exit(1);
}

/// Compact one-line summary of a rigid delta for merge/normalization output —
/// the same axis-angle formatting the pivoted diff rows use.
fn format_delta(cf: &rbx_types::CFrame) -> String {
    git_rbx::output::format_delta(&(*cf).into())
}

fn is_model_asset_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(extension.to_ascii_lowercase().as_str(), "rbxm" | "rbxmx")
        })
}

#[allow(clippy::too_many_arguments)]
fn cmd_resolve(
    file: &str,
    list: bool,
    json: bool,
    take: Option<&str>,
    value: Option<&str>,
    path: Option<&str>,
    entry_name: Option<&str>,
    all: bool,
    do_finalize: bool,
) -> Result<()> {
    let (mut dom, source) = load_file(file, None)?;
    let Some(container) = find_container(&dom) else {
        bail!("{file} has no conflict container — nothing to resolve");
    };

    if list {
        if json {
            let report = conflict_report(&dom, container);
            println!("{}", serde_json::to_string_pretty(&report)?);
            return Ok(());
        }
        for entry in list_entries(&dom, container) {
            let state = entry.resolved.as_deref().unwrap_or("UNRESOLVED");
            let detail = entry
                .property
                .as_deref()
                .map(|p| format!(" ({p})"))
                .unwrap_or_default();
            let group = entry
                .group
                .as_deref()
                .map(|g| format!(" [{g}]"))
                .unwrap_or_default();
            println!(
                "[{state}] {} {} — {}{}{}",
                entry.name, entry.path, entry.kind, detail, group
            );
        }
        return Ok(());
    }

    if take == Some("custom") {
        let entry_name =
            entry_name.ok_or_else(|| anyhow::anyhow!("--take custom requires --entry <name>"))?;
        let value =
            value.ok_or_else(|| anyhow::anyhow!("--take custom requires --value <json>"))?;
        let parsed: serde_json::Value =
            serde_json::from_str(value).with_context(|| format!("parsing --value {value}"))?;
        let entry = list_entries(&dom, container)
            .into_iter()
            .find(|e| e.name == entry_name)
            .ok_or_else(|| anyhow::anyhow!("no conflict entry named {entry_name}"))?;
        mark_entry_custom(&mut dom, entry.entry_ref, &parsed)?;
        save_file(file, &dom, source)?;
        eprintln!("Marked {entry_name} as custom");
        return Ok(());
    }

    if let Some(side) = take {
        let entries = list_entries(&dom, container);
        let targets: Vec<_> = entries
            .iter()
            .filter(|e| match (entry_name, path) {
                (Some(name), _) => e.name == name || e.group.as_deref() == Some(name),
                (None, Some(p)) => e.path == p,
                (None, None) => all,
            })
            .collect();
        if targets.is_empty() {
            bail!("no conflicts matched (use --entry <name>, --path <base path>, or --all)");
        }
        let count = targets.len();
        let refs: Vec<_> = targets.iter().map(|e| e.entry_ref).collect();
        for entry_ref in refs {
            mark_entry(&mut dom, entry_ref, side)?;
        }
        save_file(file, &dom, source)?;
        eprintln!("Marked {count} conflict(s) as '{side}'");

        let remaining = list_entries(&dom, container)
            .iter()
            .filter(|e| e.resolved.is_none())
            .count();
        if remaining == 0 {
            eprintln!("All conflicts resolved — run: git rbx resolve {file} --finalize");
        } else {
            eprintln!("{remaining} conflict(s) still unresolved");
        }
        return Ok(());
    }

    if do_finalize {
        let count = finalize(&mut dom)?;
        save_file(file, &dom, source)?;
        eprintln!("Applied {count} resolution(s); conflict state stripped from {file}");
        return Ok(());
    }

    bail!("specify --list, --take <ours|theirs> (--path/--all), --finalize, or --studio");
}

/// The Studio resolver checkout, resolved against this crate at build time.
/// `resolve --studio` therefore needs the checkout present at its build
/// location — fine while the tool is iterated and run from source; a
/// self-contained binary (pre-bundled script embedded at build) is the
/// eventual shape once the resolver stabilizes. The root is also passed to
/// the script (--resolver-root) so it can rojo-build roblox_packages.
const RESOLVER_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/studio-resolver");
const RESOLVER_ENTRY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/studio-resolver/src/init.luau");

/// Launch the visual resolver in Roblox Studio via rodeo. The session stages
/// decisions in-Studio and calls back into this binary (`resolve --take`,
/// `--finalize`) when the user hits Complete — the file on disk is the only
/// truth, so the verdict afterwards is simply whether conflict state remains.
fn cmd_resolve_studio(file: &str, auto: Option<&str>) -> Result<()> {
    let (dom, _) = load_file(file, None)?;
    let Some(container) = find_container(&dom) else {
        bail!("{file} has no conflict container — nothing to resolve");
    };
    let unresolved = list_entries(&dom, container)
        .iter()
        .filter(|e| e.resolved.is_none())
        .count();

    // Places open in Studio directly; models run in an empty place and the
    // resolver imports them into a preview folder.
    let abs_file = std::fs::canonicalize(file)?;
    let is_place = matches!(extension(file).as_str(), "rbxl" | "rbxlx");

    if !Path::new(RESOLVER_ENTRY).exists() {
        bail!(
            "resolver source not found at {RESOLVER_ENTRY} — `resolve --studio` \
             currently runs from the git-rbx checkout it was built in; rebuild \
             on this machine or resolve from the CLI (git rbx resolve {file} --list)"
        );
    }

    // `--place` always: with a path rodeo opens that place; with no value it
    // launches an empty one. Omitting it entirely would route the script
    // into whatever Studio session happens to be running.
    let mut cmd = std::process::Command::new("rodeo");
    cmd.arg("run");
    if is_place {
        cmd.arg("--place").arg(&abs_file);
    } else {
        cmd.arg("--place");
    }
    cmd.arg("--focus")
        .args(["--show-widgets", "none"])
        .arg(RESOLVER_ENTRY)
        .arg("--")
        .arg(&abs_file)
        .arg("--git-rbx")
        .arg(std::env::current_exe()?)
        .arg("--resolver-root")
        .arg(RESOLVER_ROOT);
    if let Some(side) = auto {
        cmd.args(["--auto", side]);
    }

    eprintln!("Opening the Studio resolver for {file} ({unresolved} unresolved conflict(s))...");
    let status = cmd.status().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => anyhow::anyhow!(
            "`rodeo` not found on PATH — the Studio resolver runs through it \
             (https://github.com/revvy02/rodeo). Resolve from the CLI instead: \
             git rbx resolve {file} --list"
        ),
        _ => anyhow::Error::from(e).context("launching rodeo"),
    })?;

    // rodeo's exit code only says how the SESSION ended (completed, killed,
    // Studio closed mid-way); what the merge is at now is in the file.
    if find_container(&load_file(file, None)?.0).is_none() {
        eprintln!("{file}: conflicts resolved, file is clean");
        Ok(())
    } else {
        if !status.success() {
            eprintln!("(resolver session ended without completing)");
        }
        eprintln!("{file}: still contains conflict state");
        std::process::exit(1);
    }
}

fn cmd_check(file: &str, json: bool) -> Result<()> {
    let (dom, _) = load_file(file, None)?;
    let unresolved = find_container(&dom).map(|container| {
        list_entries(&dom, container)
            .iter()
            .filter(|e| e.resolved.is_none())
            .count()
    });
    if json {
        println!(
            "{}",
            serde_json::json!({
                "file": file,
                "clean": unresolved.is_none(),
                "unresolvedCount": unresolved.unwrap_or(0),
            })
        );
    }
    match unresolved {
        Some(unresolved) => {
            eprintln!("{file}: contains merge conflict state ({unresolved} unresolved)");
            std::process::exit(1);
        }
        None => {
            eprintln!("{file}: clean");
            Ok(())
        }
    }
}

// ============================================================================
// install: git wiring
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConfigScope {
    Global,
    Local,
}

impl ConfigScope {
    fn flag(self) -> &'static str {
        match self {
            ConfigScope::Global => "--global",
            ConfigScope::Local => "--local",
        }
    }
}

struct InstallOptions {
    scope: ConfigScope,
    attributes: bool,
    hooks: bool,
    exe: Option<String>,
    check: bool,
}

/// The managed region of .gitattributes. Everything between the markers is
/// rewritten on every install; user content outside is preserved verbatim.
const ATTRIBUTES_BEGIN: &str = "# >>> git-rbx (managed; re-run `git rbx install` to update)";
const ATTRIBUTES_END: &str = "# <<< git-rbx";
const ROBLOX_GLOBS: [&str; 4] = ["*.rbxl", "*.rbxlx", "*.rbxm", "*.rbxmx"];
const HOOK_MARKER: &str = "# git-rbx pre-commit";

fn attributes_block() -> String {
    let mut block = String::new();
    block.push_str(ATTRIBUTES_BEGIN);
    block.push('\n');
    for glob in ROBLOX_GLOBS {
        // -text: never let git normalize line endings or attempt a text
        // merge on these; merge=rbx / diff=rbx: route three-way merges and
        // `git diff` through git-rbx.
        block.push_str(&format!("{glob:<8} merge=rbx diff=rbx -text\n"));
    }
    block.push_str(ATTRIBUTES_END);
    block.push('\n');
    block
}

/// Shell-safe reference to the executable inside git config commands.
fn exe_reference(exe: Option<&str>) -> String {
    let exe = exe.unwrap_or("git-rbx");
    if exe.chars().any(char::is_whitespace) {
        format!("\"{exe}\"")
    } else {
        exe.to_string()
    }
}

/// (key, value) pairs the driver needs. `recursive = binary` matters on
/// criss-cross merges: git synthesizes a virtual ancestor by merging, and a
/// conflict-stamped file must never become the ancestor of the real merge.
fn config_entries(exe: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            "merge.rbx.name",
            "git-rbx semantic merge for Roblox files".to_string(),
        ),
        (
            "merge.rbx.driver",
            format!("{exe} merge %O %A %B --path %P"),
        ),
        ("merge.rbx.recursive", "binary".to_string()),
        ("diff.rbx.command", format!("{exe} git-diff")),
        (
            "mergetool.rbx.cmd",
            format!("{exe} resolve \"$MERGED\" --studio"),
        ),
        ("mergetool.rbx.trustExitCode", "true".to_string()),
    ]
}

fn git_output(args: &[&str]) -> Result<Option<String>> {
    let output = std::process::Command::new("git")
        .args(args)
        .output()
        .context("running git (is it installed and on PATH?)")?;
    if output.status.success() {
        Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim_end().to_string(),
        ))
    } else {
        Ok(None)
    }
}

fn git_run(args: &[&str]) -> Result<()> {
    let output = std::process::Command::new("git")
        .args(args)
        .output()
        .context("running git (is it installed and on PATH?)")?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn repository_toplevel() -> Result<Option<std::path::PathBuf>> {
    Ok(git_output(&["rev-parse", "--show-toplevel"])?.map(std::path::PathBuf::from))
}

fn on_path(program: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let candidate = dir.join(program);
        candidate.is_file() || dir.join(format!("{program}.exe")).is_file()
    })
}

/// Rewrite the managed block at the END of the file, preserving everything
/// else. Last matching line wins per attribute in git, and `git lfs track`
/// appends `merge=lfs diff=lfs` lines for the same globs — the block must
/// come after them to take effect.
fn merge_attributes(existing: &str, block: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut inside = false;
    for line in existing.lines() {
        if line.trim() == ATTRIBUTES_BEGIN {
            inside = true;
            continue;
        }
        if inside {
            if line.trim() == ATTRIBUTES_END {
                inside = false;
            }
            continue;
        }
        kept.push(line);
    }
    while kept.last().is_some_and(|line| line.trim().is_empty()) {
        kept.pop();
    }
    let mut result = kept.join("\n");
    if !result.is_empty() {
        result.push_str("\n\n");
    }
    result.push_str(block);
    result
}

/// The block is effective only when nothing after it re-assigns the
/// attributes (e.g. a later `git lfs track`).
fn attributes_block_effective(existing: &str) -> bool {
    existing.trim_end().ends_with(attributes_block().trim_end())
}

fn hook_script(exe: &str) -> String {
    format!(
        r#"#!/bin/sh
{HOOK_MARKER}: refuse to commit Roblox files still carrying merge conflict state.
git diff --cached --name-only --diff-filter=ACM -- '*.rbxl' '*.rbxlx' '*.rbxm' '*.rbxmx' |
while IFS= read -r file; do
    if ! {exe} check "$file" >/dev/null 2>&1; then
        echo "git-rbx: $file has unresolved merge conflict state" >&2
        echo "         resolve it first: git rbx resolve \"$file\" --list" >&2
        exit 1
    fi
done
"#
    )
}

fn cmd_install(options: InstallOptions) -> Result<()> {
    let exe = exe_reference(options.exe.as_deref());
    let entries = config_entries(&exe);
    let toplevel = repository_toplevel()?;
    if options.scope == ConfigScope::Local && toplevel.is_none() {
        bail!("--local requires running inside a git repository");
    }
    let attributes_path = toplevel.as_ref().map(|top| top.join(".gitattributes"));

    if options.check {
        return install_check(options.scope, &entries, attributes_path.as_deref());
    }

    if options.exe.is_none() && !on_path("git-rbx") {
        eprintln!(
            "warning: `git-rbx` is not on PATH; the driver git runs will not be found. \
             Put it on PATH, or re-run with --exe {}",
            std::env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "<path to git-rbx>".to_string())
        );
    }

    for (key, value) in &entries {
        git_run(&["config", options.scope.flag(), key, value])?;
    }
    eprintln!(
        "Wrote {} git config: merge.rbx.* (driver), diff.rbx.* (git diff), mergetool.rbx.* (Studio resolver)",
        match options.scope {
            ConfigScope::Global => "global",
            ConfigScope::Local => "repository",
        }
    );

    if options.attributes {
        match &attributes_path {
            Some(path) => {
                let existing = std::fs::read_to_string(path).unwrap_or_default();
                let updated = merge_attributes(&existing, &attributes_block());
                if updated != existing {
                    std::fs::write(path, updated)
                        .with_context(|| format!("writing {}", path.display()))?;
                    eprintln!(
                        "Updated {} (commit it so every clone routes Roblox files through git-rbx)",
                        path.display()
                    );
                } else {
                    eprintln!("{} already up to date", path.display());
                }
            }
            None => eprintln!(
                "Not inside a git repository: skipped .gitattributes. Run `git rbx install` \
                 in each repository once to add the merge=rbx attributes"
            ),
        }
    }

    if options.hooks {
        let hooks_dir = git_output(&["rev-parse", "--git-path", "hooks"])?
            .ok_or_else(|| anyhow::anyhow!("--hooks requires running inside a git repository"))?;
        let hook_path = Path::new(&hooks_dir).join("pre-commit");
        if let Ok(existing) = std::fs::read_to_string(&hook_path) {
            if !existing.contains(HOOK_MARKER) {
                bail!(
                    "{} already exists and is not managed by git-rbx; add this to it manually:\n\n{}",
                    hook_path.display(),
                    hook_script(&exe)
                );
            }
        }
        std::fs::create_dir_all(&hooks_dir)?;
        std::fs::write(&hook_path, hook_script(&exe))
            .with_context(|| format!("writing {}", hook_path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755))?;
        }
        eprintln!("Installed pre-commit hook at {}", hook_path.display());
    }

    if options.scope == ConfigScope::Global {
        eprintln!("Done. Each teammate runs `git rbx install` once; the .gitattributes change ships with the repository.");
    }
    eprintln!("Note: `git diff` uses the semantic differ automatically; `git log -p` and `git show` need --ext-diff.");
    Ok(())
}

fn install_check(
    scope: ConfigScope,
    entries: &[(&str, String)],
    attributes_path: Option<&Path>,
) -> Result<()> {
    let mut drift = false;
    let mut report = |ok: bool, what: String| {
        eprintln!("  [{}] {what}", if ok { "ok" } else { "MISSING" });
        drift |= !ok;
    };
    for (key, expected) in entries {
        // Any scope may satisfy it at runtime, but report the requested one
        // so `--local --check` is a precise question.
        let actual = git_output(&["config", scope.flag(), "--get", key])?;
        let ok = actual.as_deref() == Some(expected.as_str());
        report(ok, format!("{key} = {expected}"));
    }
    match attributes_path {
        Some(path) => {
            let existing = std::fs::read_to_string(path).unwrap_or_default();
            let ok = attributes_block_effective(&existing);
            report(
                ok,
                format!(
                    "{}: managed git-rbx block (last, so it overrides earlier lines)",
                    path.display()
                ),
            );
        }
        None => eprintln!("  [skip] .gitattributes (not inside a git repository)"),
    }
    if drift {
        eprintln!("git-rbx is not fully installed; run: git rbx install{}", match scope {
            ConfigScope::Global => "",
            ConfigScope::Local => " --local",
        });
        std::process::exit(1);
    }
    eprintln!("git-rbx is installed");
    Ok(())
}

/// On-disk encoding of a Roblox file. Independent of whether the content is
/// a model or a place: both come in either encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileFormat {
    Binary,
    Xml,
}

/// How a file reached us and how to write it back the same way.
#[derive(Debug, Clone, Copy)]
struct Source {
    format: FileFormat,
    /// The file on disk was a Git LFS pointer (content resolved through
    /// `git lfs smudge`); writes go back through `git lfs clean`.
    lfs_pointer: bool,
}

/// Read a file's content. A Git LFS pointer is resolved first — before the
/// extension is trusted, since a `.rbxl` in a skip-smudge checkout or a
/// git temp copy is pointer text. The encoding then resolves from:
/// 1. the extension of `hint` — the real repository path (git's %P);
/// 2. the extension of `path` itself;
/// 3. the leading bytes. Git merge drivers receive extensionless temp copies
///    (`.merge_file_XXXXXX`), and the content is unambiguous: binary files
///    open with the `<roblox!` magic, XML with `<roblox`.
fn read_source(path: &str, hint: Option<&str>) -> Result<(Vec<u8>, Source)> {
    let bytes = std::fs::read(path).with_context(|| format!("opening {path}"))?;
    source_from_bytes(path, bytes, hint)
}

/// The content-resolution half of [`read_source`], for bytes that did not
/// come from a file on disk (e.g. `git cat-file`). `label` names the input
/// in messages and supplies the fallback extension.
fn source_from_bytes(
    label: &str,
    mut bytes: Vec<u8>,
    hint: Option<&str>,
) -> Result<(Vec<u8>, Source)> {
    let lfs_pointer = git_lfs::is_pointer(&bytes);
    if lfs_pointer {
        bytes = git_lfs::smudge(&bytes, hint.unwrap_or(label))
            .with_context(|| format!("{label} is a Git LFS pointer; resolving its content"))?;
    }
    let format = match hint
        .and_then(format_from_extension)
        .or_else(|| format_from_extension(label))
    {
        Some(format) => format,
        None => sniff_format(label, &bytes)?,
    };
    Ok((
        bytes,
        Source {
            format,
            lfs_pointer,
        },
    ))
}

fn diff_dom_from_bytes(bytes: &[u8], format: FileFormat) -> Result<DiffDom> {
    Ok(match format {
        FileFormat::Binary => DiffDom::from_binary_reader(bytes)?,
        FileFormat::Xml => DiffDom::from_weak_dom_owned(rbx_xml::from_reader_default(bytes)?),
    })
}

fn format_from_extension(path: &str) -> Option<FileFormat> {
    match extension(path).as_str() {
        "rbxm" | "rbxl" => Some(FileFormat::Binary),
        "rbxmx" | "rbxlx" => Some(FileFormat::Xml),
        _ => None,
    }
}

fn sniff_format(path: &str, bytes: &[u8]) -> Result<FileFormat> {
    let head = &bytes[..bytes.len().min(512)];
    if head.starts_with(b"<roblox!") {
        return Ok(FileFormat::Binary);
    }
    let text = String::from_utf8_lossy(head);
    let text = text.trim_start_matches('\u{feff}').trim_start();
    if text.starts_with("<roblox") || (text.starts_with("<?xml") && text.contains("<roblox")) {
        return Ok(FileFormat::Xml);
    }
    bail!(
        "{path}: not a recognizable Roblox file (no .rbxm/.rbxl/.rbxmx/.rbxlx extension and \
         no <roblox header). When invoked by git with temp files, pass --path <real path>"
    )
}

/// Load a Roblox file (model or place).
fn load_file(path: &str, hint: Option<&str>) -> Result<(rbx_dom_weak::WeakDom, Source)> {
    let (bytes, source) = read_source(path, hint)?;
    let dom = match source.format {
        FileFormat::Binary => rbx_binary::from_reader(bytes.as_slice())?,
        FileFormat::Xml => rbx_xml::from_reader_default(bytes.as_slice())?,
    };
    Ok((dom, source))
}

/// Load directly into the compact comparison DOM when the source format
/// supports it. XML still uses WeakDom as its parser output.
fn load_diff_file(path: &str, hint: Option<&str>) -> Result<(DiffDom, Source)> {
    let (bytes, source) = read_source(path, hint)?;
    Ok((diff_dom_from_bytes(&bytes, source.format)?, source))
}

/// Write a DOM back the way its source arrived: same encoding, and through
/// `git lfs clean` when the source was an LFS pointer, so the object lands
/// in the LFS store and git records a pointer.
fn save_file(path: &str, dom: &rbx_dom_weak::WeakDom, source: Source) -> Result<()> {
    let mut bytes = Vec::new();
    let roots = dom.root().children();
    match source.format {
        FileFormat::Binary => rbx_binary::to_writer(&mut bytes, dom, roots)?,
        FileFormat::Xml => rbx_xml::to_writer_default(&mut bytes, dom, roots)?,
    }
    if source.lfs_pointer {
        bytes = git_lfs::clean(&bytes)
            .with_context(|| format!("storing {path} through Git LFS"))?;
    }
    std::fs::write(path, bytes).with_context(|| format!("creating {path}"))?;
    Ok(())
}

fn extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}
