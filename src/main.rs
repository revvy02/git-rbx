//! rbx-diff: A fast diff and merge tool for Roblox place/model files.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::time::Instant;
use tracing::info_span;
use tracing_subscriber::{fmt, EnvFilter};

use rbx_diff::output::{print_diff, OutputFormat};
use rbx_diff::{
    diff_doms_with_config, finalize, find_container, list_entries, mark_entry, mark_entry_custom,
    merge_doms, stamp_conflicts, ConflictKind, DiffConfig, CONTAINER_NAME,
};

#[derive(Parser)]
#[command(name = "rbx-diff")]
#[command(about = "Diff and merge Roblox rbxm/rbxmx/rbxl/rbxlx files")]
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

        /// Only show summary counts
        #[arg(long)]
        summary_only: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Properties to ignore (comma-separated)
        #[arg(long, value_delimiter = ',')]
        ignore_property: Vec<String>,

        /// Show timing information
        #[arg(long, short = 't')]
        timing: bool,
    },
    /// Three-way merge (git merge driver: rbx-diff merge %O %A %B).
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

        /// Properties to ignore (comma-separated)
        #[arg(long, value_delimiter = ',')]
        ignore_property: Vec<String>,
    },
    /// Inspect and resolve conflicts stored in a merged file
    Resolve {
        /// The conflicted file written by `rbx-diff merge`
        file: String,

        /// List conflicts and their resolution state
        #[arg(long)]
        list: bool,

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
        #[arg(long, conflicts_with_all = ["list", "take", "value", "path", "entry", "all", "finalize"])]
        studio: bool,

        /// Debug: auto-stage every conflict to this side and complete
        #[arg(long, hide = true, requires = "studio", value_name = "SIDE")]
        studio_auto: Option<String>,
    },
    /// Exit nonzero if the file contains unresolved merge conflict state
    Check {
        /// File to check
        file: String,
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
        Command::Diff { old_file, new_file, summary_only, json, ignore_property, timing } => {
            cmd_diff(&old_file, &new_file, summary_only, json, &ignore_property, timing)
        }
        Command::Merge { base, ours, theirs, output, ignore_property } => {
            cmd_merge(&base, &ours, &theirs, output.as_deref(), &ignore_property)
        }
        Command::Resolve { file, list, take, value, path, entry, all, finalize, studio, studio_auto } => {
            if studio {
                cmd_resolve_studio(&file, studio_auto.as_deref())
            } else {
                cmd_resolve(&file, list, take.as_deref(), value.as_deref(), path.as_deref(), entry.as_deref(), all, finalize)
            }
        }
        Command::Check { file } => cmd_check(&file),
    }
}

fn cmd_diff(
    old_file: &str,
    new_file: &str,
    summary_only: bool,
    json: bool,
    ignore_property: &[String],
    timing: bool,
) -> Result<()> {
    let total_start = Instant::now();

    let load_start = Instant::now();
    eprintln!("Loading {}...", old_file);
    let old_dom = {
        let _span = info_span!("load_old_file", file = %old_file).entered();
        load_file(old_file)?
    };
    let old_load_time = load_start.elapsed();

    let load_start = Instant::now();
    eprintln!("Loading {}...", new_file);
    let new_dom = {
        let _span = info_span!("load_new_file", file = %new_file).entered();
        load_file(new_file)?
    };
    let new_load_time = load_start.elapsed();

    let config = build_config(ignore_property);

    let diff_start = Instant::now();
    eprintln!("Computing differences...");
    let diffs = diff_doms_with_config(&old_dom, &new_dom, &config);
    let diff_time = diff_start.elapsed();

    let total_time = total_start.elapsed();

    let format = if json {
        OutputFormat::Json
    } else if summary_only {
        OutputFormat::Summary
    } else {
        OutputFormat::Pretty
    };

    eprintln!();
    print_diff(&diffs, format);

    if timing {
        eprintln!();
        eprintln!("Timing:");
        eprintln!("  Load old file: {:?}", old_load_time);
        eprintln!("  Load new file: {:?}", new_load_time);
        eprintln!("  Diff computation (includes lazy hashing): {:?}", diff_time);
        eprintln!("  Total: {:?}", total_time);
    }

    Ok(())
}

fn cmd_merge(
    base_path: &str,
    ours_path: &str,
    theirs_path: &str,
    output: Option<&str>,
    ignore_property: &[String],
) -> Result<()> {
    eprintln!("Loading base {}...", base_path);
    let mut base = load_file(base_path)?;
    eprintln!("Loading ours {}...", ours_path);
    let ours = load_file(ours_path)?;
    eprintln!("Loading theirs {}...", theirs_path);
    let theirs = load_file(theirs_path)?;

    let config = build_config(ignore_property);

    eprintln!("Merging...");
    let start = Instant::now();
    let result = merge_doms(&mut base, &ours, &theirs, &config);
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
    // discovery. `rbx-diff resolve` (or Studio) consumes it.
    if !result.conflicts.is_empty() {
        stamp_conflicts(&mut base, &ours, &theirs, &result);
    }

    let out_path = output.unwrap_or(ours_path);
    save_file(out_path, &base)?;
    eprintln!("Wrote merged result to {}", out_path);

    if result.conflicts.is_empty() {
        return Ok(());
    }

    eprintln!();
    eprintln!("CONFLICTS ({}):", result.conflicts.len());
    for conflict in &result.conflicts {
        let kind = match &conflict.kind {
            ConflictKind::Property { name } => format!("property '{}'", name),
            ConflictKind::DeleteVsEdit => "delete vs edit".to_string(),
            ConflictKind::MoveTarget => "conflicting move destinations".to_string(),
        };
        eprintln!("  ! {} — {} (base content kept)", conflict.path, kind);
    }

    eprintln!();
    eprintln!("Conflict state is stored in the file ({CONTAINER_NAME}); resolve with:");
    eprintln!("  rbx-diff resolve {} --list", out_path);

    // Nonzero exit tells git the merge needs manual resolution
    std::process::exit(1);
}

fn cmd_resolve(
    file: &str,
    list: bool,
    take: Option<&str>,
    value: Option<&str>,
    path: Option<&str>,
    entry_name: Option<&str>,
    all: bool,
    do_finalize: bool,
) -> Result<()> {
    let mut dom = load_file(file)?;
    let Some(container) = find_container(&dom) else {
        bail!("{file} has no conflict container — nothing to resolve");
    };

    if list {
        for entry in list_entries(&dom, container) {
            let state = entry.resolved.as_deref().unwrap_or("UNRESOLVED");
            let detail = entry
                .property
                .as_deref()
                .map(|p| format!(" ({p})"))
                .unwrap_or_default();
            println!("[{state}] {} {} — {}{}", entry.name, entry.path, entry.kind, detail);
        }
        return Ok(());
    }

    if take == Some("custom") {
        let entry_name = entry_name
            .ok_or_else(|| anyhow::anyhow!("--take custom requires --entry <name>"))?;
        let value = value
            .ok_or_else(|| anyhow::anyhow!("--take custom requires --value <json>"))?;
        let parsed: serde_json::Value =
            serde_json::from_str(value).with_context(|| format!("parsing --value {value}"))?;
        let entry = list_entries(&dom, container)
            .into_iter()
            .find(|e| e.name == entry_name)
            .ok_or_else(|| anyhow::anyhow!("no conflict entry named {entry_name}"))?;
        mark_entry_custom(&mut dom, entry.entry_ref, &parsed)?;
        save_file(file, &dom)?;
        eprintln!("Marked {entry_name} as custom");
        return Ok(());
    }

    if let Some(side) = take {
        let entries = list_entries(&dom, container);
        let targets: Vec<_> = entries
            .iter()
            .filter(|e| match (entry_name, path) {
                (Some(name), _) => e.name == name,
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
        save_file(file, &dom)?;
        eprintln!("Marked {count} conflict(s) as '{side}'");

        let remaining = list_entries(&dom, container)
            .iter()
            .filter(|e| e.resolved.is_none())
            .count();
        if remaining == 0 {
            eprintln!("All conflicts resolved — run: rbx-diff resolve {file} --finalize");
        } else {
            eprintln!("{remaining} conflict(s) still unresolved");
        }
        return Ok(());
    }

    if do_finalize {
        let count = finalize(&mut dom)?;
        save_file(file, &dom)?;
        eprintln!("Applied {count} resolution(s); conflict state stripped from {file}");
        return Ok(());
    }

    bail!("specify --list, --take <ours|theirs> (--path/--all), --finalize, or --studio");
}

/// Entry point of the Studio resolver, resolved against this crate's checkout
/// at build time. `resolve --studio` therefore needs the checkout present at
/// its build location — fine while the tool is iterated and run from source;
/// a self-contained binary (pre-bundled script embedded at build) is the
/// eventual shape once the resolver stabilizes.
const RESOLVER_ENTRY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/studio-resolver/src/init.luau");

/// Launch the visual resolver in Roblox Studio via rodeo. The session stages
/// decisions in-Studio and calls back into this binary (`resolve --take`,
/// `--finalize`) when the user hits Complete — the file on disk is the only
/// truth, so the verdict afterwards is simply whether conflict state remains.
fn cmd_resolve_studio(file: &str, auto: Option<&str>) -> Result<()> {
    let dom = load_file(file)?;
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
    let is_place = matches!(extension(file)?.as_str(), "rbxl" | "rbxlx");

    if !Path::new(RESOLVER_ENTRY).exists() {
        bail!(
            "resolver source not found at {RESOLVER_ENTRY} — `resolve --studio` \
             currently runs from the rbx-diff checkout it was built in; rebuild \
             on this machine or resolve from the CLI (rbx-diff resolve {file} --list)"
        );
    }

    let mut cmd = std::process::Command::new("rodeo");
    cmd.arg("run").arg("--place");
    if is_place {
        cmd.arg(&abs_file);
    }
    cmd.arg("--focus")
        .arg(RESOLVER_ENTRY)
        .arg("--")
        .arg(&abs_file)
        .arg("--rbx-diff")
        .arg(std::env::current_exe()?);
    if let Some(side) = auto {
        cmd.args(["--auto", side]);
    }

    eprintln!("Opening the Studio resolver for {file} ({unresolved} unresolved conflict(s))...");
    let status = cmd.status().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => anyhow::anyhow!(
            "`rodeo` not found on PATH — the Studio resolver runs through it \
             (https://github.com/revvy02/rodeo). Resolve from the CLI instead: \
             rbx-diff resolve {file} --list"
        ),
        _ => anyhow::Error::from(e).context("launching rodeo"),
    })?;

    // rodeo's exit code only says how the SESSION ended (completed, killed,
    // Studio closed mid-way); what the merge is at now is in the file.
    if find_container(&load_file(file)?).is_none() {
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

fn cmd_check(file: &str) -> Result<()> {
    let dom = load_file(file)?;
    match find_container(&dom) {
        Some(container) => {
            let unresolved = list_entries(&dom, container)
                .iter()
                .filter(|e| e.resolved.is_none())
                .count();
            eprintln!(
                "{file}: contains merge conflict state ({unresolved} unresolved)"
            );
            std::process::exit(1);
        }
        None => {
            eprintln!("{file}: clean");
            Ok(())
        }
    }
}

fn build_config(ignore_property: &[String]) -> DiffConfig {
    let mut config = DiffConfig::default();
    for prop in ignore_property {
        config.ignore_properties.insert(prop.clone());
    }
    config
}

/// Load a Roblox file (binary or XML, model or place) based on extension.
fn load_file(path: &str) -> Result<rbx_dom_weak::WeakDom> {
    let ext = extension(path)?;
    let file = BufReader::new(File::open(path).with_context(|| format!("opening {path}"))?);
    match ext.as_str() {
        "rbxm" | "rbxl" => Ok(rbx_binary::from_reader(file)?),
        "rbxmx" | "rbxlx" => Ok(rbx_xml::from_reader_default(file)?),
        _ => bail!("Unknown file extension: {ext}. Expected .rbxm, .rbxmx, .rbxl, or .rbxlx"),
    }
}

/// Save a DOM in the format implied by the path's extension.
fn save_file(path: &str, dom: &rbx_dom_weak::WeakDom) -> Result<()> {
    let ext = extension(path)?;
    let file = BufWriter::new(File::create(path).with_context(|| format!("creating {path}"))?);
    let roots = dom.root().children();
    match ext.as_str() {
        "rbxm" | "rbxl" => rbx_binary::to_writer(file, dom, roots)?,
        "rbxmx" | "rbxlx" => rbx_xml::to_writer_default(file, dom, roots)?,
        _ => bail!("Unknown file extension: {ext}. Expected .rbxm, .rbxmx, .rbxl, or .rbxlx"),
    }
    Ok(())
}

fn extension(path: &str) -> Result<String> {
    Ok(Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase())
}
