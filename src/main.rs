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
use rbx_diff::{diff_doms_with_config, merge_doms, ConflictKind, DiffConfig};

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

    // Nonzero exit tells git the merge needs manual resolution
    std::process::exit(1);
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
