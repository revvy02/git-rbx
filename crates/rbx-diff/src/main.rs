//! rbx-diff: A fast diff tool for comparing Roblox rbxm/rbxmx files.

use anyhow::{bail, Result};
use clap::Parser;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Instant;

use rbx_diff::{diff_doms_with_config, DiffConfig};
use rbx_diff::output::{print_diff, OutputFormat};

#[derive(Parser)]
#[command(name = "rbx-diff")]
#[command(about = "Compare two Roblox rbxm/rbxmx files and show differences")]
#[command(version)]
struct Args {
    /// First (old) rbxm or rbxmx file
    old_file: String,

    /// Second (new) rbxm or rbxmx file
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
}

fn main() -> Result<()> {
    let args = Args::parse();

    let total_start = Instant::now();

    // Load old file
    let load_start = Instant::now();
    eprintln!("Loading {}...", args.old_file);
    let old_dom = load_file(&args.old_file)?;
    let old_load_time = load_start.elapsed();

    // Load new file
    let load_start = Instant::now();
    eprintln!("Loading {}...", args.new_file);
    let new_dom = load_file(&args.new_file)?;
    let new_load_time = load_start.elapsed();

    // Build diff config
    let mut config = DiffConfig::default();
    for prop in &args.ignore_property {
        config.ignore_properties.insert(prop.clone());
    }

    // Compute diff
    let diff_start = Instant::now();
    eprintln!("Computing differences...");
    let diffs = diff_doms_with_config(&old_dom, &new_dom, &config);
    let diff_time = diff_start.elapsed();

    let total_time = total_start.elapsed();

    // Determine output format
    let format = if args.json {
        OutputFormat::Json
    } else if args.summary_only {
        OutputFormat::Summary
    } else {
        OutputFormat::Pretty
    };

    // Print results
    eprintln!(); // Blank line before output
    print_diff(&diffs, format);

    // Print timing if requested
    if args.timing {
        eprintln!();
        eprintln!("Timing:");
        eprintln!("  Load old file: {:?}", old_load_time);
        eprintln!("  Load new file: {:?}", new_load_time);
        eprintln!("  Diff computation (includes lazy hashing): {:?}", diff_time);
        eprintln!("  Total: {:?}", total_time);
    }

    Ok(())
}

/// Load a Roblox file (rbxm or rbxmx) based on extension.
fn load_file(path: &str) -> Result<rbx_dom_weak::WeakDom> {
    let path = Path::new(path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let file = BufReader::new(File::open(path)?);

    match ext.to_lowercase().as_str() {
        "rbxm" => Ok(rbx_binary::from_reader(file)?),
        "rbxmx" => Ok(rbx_xml::from_reader_default(file)?),
        _ => bail!("Unknown file extension: {}. Expected .rbxm or .rbxmx", ext),
    }
}
