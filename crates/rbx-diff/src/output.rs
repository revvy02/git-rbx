//! Output formatting for diff results.

use colored::Colorize;
use crate::diff::{DiffEntry, PropertyValue};

/// Output format options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    /// Human-readable colored output
    Pretty,
    /// Summary only (counts)
    Summary,
    /// JSON output for machine processing
    Json,
}

/// Print diff results to stdout.
pub fn print_diff(diffs: &[DiffEntry], format: OutputFormat) {
    match format {
        OutputFormat::Pretty => print_pretty(diffs),
        OutputFormat::Summary => print_summary(diffs),
        OutputFormat::Json => print_json(diffs),
    }
}

fn print_pretty(diffs: &[DiffEntry]) {
    if diffs.is_empty() {
        println!("{}", "No differences found.".green());
        return;
    }

    // Group by type for cleaner output
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();

    for diff in diffs {
        match diff {
            DiffEntry::Added { .. } => added.push(diff),
            DiffEntry::Removed { .. } => removed.push(diff),
            DiffEntry::Modified { .. } => modified.push(diff),
        }
    }

    // Print removed instances
    if !removed.is_empty() {
        println!("\n{}", "Removed:".red().bold());
        for diff in &removed {
            if let DiffEntry::Removed { path, class, .. } = diff {
                println!("  {} {}", "-".red(), format!("{} [{}]", path, class).red());
            }
        }
    }

    // Print added instances
    if !added.is_empty() {
        println!("\n{}", "Added:".green().bold());
        for diff in &added {
            if let DiffEntry::Added { path, class, .. } = diff {
                println!("  {} {}", "+".green(), format!("{} [{}]", path, class).green());
            }
        }
    }

    // Print modified instances
    if !modified.is_empty() {
        println!("\n{}", "Modified:".yellow().bold());
        for diff in &modified {
            if let DiffEntry::Modified { path, class, property_changes, .. } = diff {
                println!("  {} {} [{}]", "~".yellow(), path.yellow(), class);
                for change in property_changes {
                    match (&change.old_value, &change.new_value) {
                        (Some(old), Some(new)) => {
                            println!("      {}: {} {} {}",
                                change.name,
                                format_property_value(old).red(),
                                "→".dimmed(),
                                format_property_value(new).green()
                            );
                        }
                        (None, Some(new)) => {
                            println!("      {}: {} {}",
                                change.name,
                                "+".green(),
                                format_property_value(new).green()
                            );
                        }
                        (Some(old), None) => {
                            println!("      {}: {} {}",
                                change.name,
                                "-".red(),
                                format_property_value(old).red()
                            );
                        }
                        (None, None) => {}
                    }
                }
            }
        }
    }

    // Print summary
    println!();
    print_summary_line(&added, &removed, &modified);
}

/// Format a PropertyValue for human-readable display.
fn format_property_value(v: &PropertyValue) -> String {
    match v {
        PropertyValue::Nil => "nil".to_string(),
        PropertyValue::Bool { value } => value.to_string(),
        PropertyValue::Int32 { value } => value.to_string(),
        PropertyValue::Int64 { value } => value.to_string(),
        PropertyValue::Float32 { value } => format!("{:.2}", value),
        PropertyValue::Float64 { value } => format!("{:.2}", value),
        PropertyValue::String { value } => {
            if value.len() > 50 {
                format!("\"{}...\"", &value[..47])
            } else {
                format!("\"{}\"", value)
            }
        }
        PropertyValue::BinaryString { len } => format!("<binary {} bytes>", len),
        PropertyValue::Ref { value } => format!("Ref({})", &value[..8.min(value.len())]),
        PropertyValue::Vector2 { x, y } => format!("({:.2}, {:.2})", x, y),
        PropertyValue::Vector3 { x, y, z } => format!("({:.2}, {:.2}, {:.2})", x, y, z),
        PropertyValue::CFrame { position, .. } => {
            format!("CFrame({:.2}, {:.2}, {:.2})", position[0], position[1], position[2])
        }
        PropertyValue::Color3 { r, g, b } => format!("Color3({:.2}, {:.2}, {:.2})", r, g, b),
        PropertyValue::BrickColor { value } => format!("BrickColor({})", value),
        PropertyValue::Enum { value } => format!("Enum({})", value),
        PropertyValue::UDim { scale, offset } => format!("UDim({:.2}, {})", scale, offset),
        PropertyValue::UDim2 { x_scale, x_offset, y_scale, y_offset } => {
            format!("UDim2({{{:.2}, {}}}, {{{:.2}, {}}})", x_scale, x_offset, y_scale, y_offset)
        }
        PropertyValue::NumberRange { min, max } => format!("NumberRange({:.2}, {:.2})", min, max),
        PropertyValue::NumberSequence { keypoints } => {
            format!("NumberSequence({} keypoints)", keypoints.len())
        }
        PropertyValue::ColorSequence { keypoints } => {
            format!("ColorSequence({} keypoints)", keypoints.len())
        }
        PropertyValue::Rect { min_x, min_y, max_x, max_y } => {
            format!("Rect({:.2}, {:.2}, {:.2}, {:.2})", min_x, min_y, max_x, max_y)
        }
        PropertyValue::Other { type_name } => format!("<{}>", type_name),
    }
}

fn print_summary(diffs: &[DiffEntry]) {
    let mut added = 0;
    let mut removed = 0;
    let mut modified = 0;

    for diff in diffs {
        match diff {
            DiffEntry::Added { .. } => added += 1,
            DiffEntry::Removed { .. } => removed += 1,
            DiffEntry::Modified { .. } => modified += 1,
        }
    }

    if added == 0 && removed == 0 && modified == 0 {
        println!("No differences found.");
    } else {
        println!("{} added, {} removed, {} modified",
            added.to_string().green(),
            removed.to_string().red(),
            modified.to_string().yellow()
        );
    }
}

fn print_summary_line(
    added: &[&DiffEntry],
    removed: &[&DiffEntry],
    modified: &[&DiffEntry],
) {
    println!(
        "{}: {} added, {} removed, {} modified",
        "Summary".bold(),
        added.len().to_string().green(),
        removed.len().to_string().red(),
        modified.len().to_string().yellow()
    );
}

fn print_json(diffs: &[DiffEntry]) {
    // Use serde_json for proper JSON serialization
    #[derive(serde::Serialize)]
    struct Output<'a> {
        changes: &'a [DiffEntry],
        summary: Summary,
    }

    #[derive(serde::Serialize)]
    struct Summary {
        added: usize,
        removed: usize,
        modified: usize,
    }

    let added = diffs.iter().filter(|d| matches!(d, DiffEntry::Added { .. })).count();
    let removed = diffs.iter().filter(|d| matches!(d, DiffEntry::Removed { .. })).count();
    let modified = diffs.iter().filter(|d| matches!(d, DiffEntry::Modified { .. })).count();

    let output = Output {
        changes: diffs,
        summary: Summary { added, removed, modified },
    };

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
