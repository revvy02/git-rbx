//! Output formatting for diff results.

use crate::diff::{CFrameValue, DiffEntry, PropertyChange, PropertyValue};
use colored::Colorize;
use rbx_dom_weak::types::Ref;
use std::collections::HashMap;

/// Output format options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    /// Human-readable colored output
    Pretty,
    /// Summary only (counts)
    Summary,
    /// JSON output for machine processing
    Json,
    /// GitHub-flavored markdown (tables in collapsible sections) for CI
    /// step summaries and pull-request comments
    Markdown,
}

/// Rows shown per markdown section before "… and N more"; keeps a
/// per-file section comfortably inside GitHub's 65,536-character comment
/// limit for typical changes.
pub const DEFAULT_MARKDOWN_ROWS: usize = 50;

/// Print diff results to stdout.
pub fn print_diff(diffs: &[DiffEntry], format: OutputFormat) {
    match format {
        OutputFormat::Pretty => print_pretty(diffs),
        OutputFormat::Summary => print_summary(diffs),
        OutputFormat::Json => print_json(diffs),
        OutputFormat::Markdown => print!("{}", render_markdown(diffs, DEFAULT_MARKDOWN_ROWS)),
    }
}

fn print_pretty(diffs: &[DiffEntry]) {
    if diffs.is_empty() {
        println!("{}", "No differences found.".green());
        return;
    }

    let entries: Vec<&DiffEntry> = diffs.iter().collect();
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();
    let mut reparented = Vec::new();
    let mut pivoted = Vec::new();

    for diff in diffs {
        match diff {
            DiffEntry::Added { .. } => added.push(diff),
            DiffEntry::Removed { .. } => removed.push(diff),
            DiffEntry::Modified { .. } => modified.push(diff),
            DiffEntry::Reparented { .. } => reparented.push(diff),
            DiffEntry::Pivoted { .. } => pivoted.push(diff),
        }
    }

    print_path_tree(&entries, print_instance_changes);

    println!();
    print_summary_line(&added, &removed, &modified, &reparented, &pivoted);
}

fn print_instance_changes(entries: &[&DiffEntry], label: &str, depth: usize) {
    let markers = entries
        .iter()
        .map(|entry| match entry {
            DiffEntry::Added { .. } => "+".green(),
            DiffEntry::Removed { .. } => "-".red(),
            DiffEntry::Modified { .. } => "~".yellow(),
            DiffEntry::Reparented { .. } => ">".cyan(),
            DiffEntry::Pivoted { .. } => "↻".cyan(),
        })
        .map(|marker| marker.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let class = diff_class(entries[0]);
    let label = match entries[0] {
        DiffEntry::Added { .. } => label.green().bold(),
        DiffEntry::Removed { .. } => label.red().bold(),
        DiffEntry::Modified { .. } | DiffEntry::Reparented { .. } | DiffEntry::Pivoted { .. } => {
            label.bold()
        }
    };
    println!(
        "{}{} {} {}",
        tree_indent(depth),
        markers,
        label,
        format!("[{}]", class).dimmed()
    );

    for entry in entries {
        match entry {
            DiffEntry::Pivoted { delta, .. } => {
                println!("{}{}", detail_indent(depth), format_delta(delta).cyan());
            }
            DiffEntry::Reparented { old_path, .. } => {
                println!(
                    "{}{} {}",
                    detail_indent(depth),
                    "from".dimmed(),
                    old_path.cyan()
                );
            }
            DiffEntry::Modified {
                property_changes, ..
            } => print_property_changes(property_changes, depth),
            DiffEntry::Added { .. } | DiffEntry::Removed { .. } => {}
        }
    }
}

fn diff_class(diff: &DiffEntry) -> &str {
    match diff {
        DiffEntry::Added { class, .. }
        | DiffEntry::Removed { class, .. }
        | DiffEntry::Modified { class, .. }
        | DiffEntry::Reparented { class, .. }
        | DiffEntry::Pivoted { class, .. } => class,
    }
}

struct PathTreeNode<'a> {
    segment: String,
    entries: Vec<&'a DiffEntry>,
    children: Vec<PathTreeNode<'a>>,
    child_indices: HashMap<Ref, usize>,
}

impl<'a> PathTreeNode<'a> {
    fn root() -> Self {
        Self {
            segment: String::new(),
            entries: Vec::new(),
            children: Vec::new(),
            child_indices: HashMap::new(),
        }
    }

    fn new(segment: String) -> Self {
        Self {
            segment,
            entries: Vec::new(),
            children: Vec::new(),
            child_indices: HashMap::new(),
        }
    }

    fn insert(&mut self, segments: &[(Ref, String)], entry: &'a DiffEntry) {
        let Some(((referent, segment), remaining)) = segments.split_first() else {
            self.entries.push(entry);
            return;
        };
        let child_index = match self.child_indices.get(referent) {
            Some(index) => *index,
            None => {
                let index = self.children.len();
                self.children.push(Self::new(segment.clone()));
                self.child_indices.insert(*referent, index);
                index
            }
        };
        self.children[child_index].insert(remaining, entry);
    }
}

fn print_path_tree<'a>(
    entries: &[&'a DiffEntry],
    mut print_entries: impl FnMut(&[&DiffEntry], &str, usize),
) {
    let mut root = PathTreeNode::root();
    for entry in entries {
        root.insert(path_segments(entry), entry);
    }

    for group in group_entries(&root.entries) {
        print_entries(&group, "<root>", 0);
    }
    for child in &root.children {
        print_path_node(child, 0, &mut print_entries);
    }
}

fn print_path_node(
    node: &PathTreeNode<'_>,
    depth: usize,
    print_entries: &mut impl FnMut(&[&DiffEntry], &str, usize),
) {
    if node.entries.is_empty() {
        println!("{}{}", tree_indent(depth), node.segment.dimmed());
    } else {
        for group in group_entries(&node.entries) {
            print_entries(&group, &node.segment, depth);
        }
    }

    for child in &node.children {
        print_path_node(child, depth + 1, print_entries);
    }
}

fn group_entries<'a>(entries: &[&'a DiffEntry]) -> Vec<Vec<&'a DiffEntry>> {
    let mut groups: Vec<Vec<&DiffEntry>> = Vec::new();
    let mut group_indices = HashMap::new();
    for entry in entries {
        let identity = diff_identity(entry);
        let index = match group_indices.get(&identity) {
            Some(index) => *index,
            None => {
                let index = groups.len();
                groups.push(Vec::new());
                group_indices.insert(identity, index);
                index
            }
        };
        groups[index].push(entry);
    }
    groups
}

fn diff_identity(diff: &DiffEntry) -> (bool, &str) {
    match diff {
        DiffEntry::Removed { old_ref, .. } => (false, old_ref),
        DiffEntry::Added { new_ref, .. }
        | DiffEntry::Modified { new_ref, .. }
        | DiffEntry::Reparented { new_ref, .. }
        | DiffEntry::Pivoted { new_ref, .. } => (true, new_ref),
    }
}

fn path_segments(diff: &DiffEntry) -> &[(Ref, String)] {
    match diff {
        DiffEntry::Added { path_segments, .. }
        | DiffEntry::Removed { path_segments, .. }
        | DiffEntry::Modified { path_segments, .. }
        | DiffEntry::Reparented { path_segments, .. }
        | DiffEntry::Pivoted { path_segments, .. } => path_segments,
    }
}

fn tree_indent(depth: usize) -> String {
    "  ".repeat(depth)
}

fn detail_indent(depth: usize) -> String {
    "  ".repeat(depth + 2)
}

enum PropertyTreeItem<'a> {
    Change(&'a PropertyChange),
    Container {
        name: &'static str,
        changes: Vec<(&'a str, &'a PropertyChange)>,
    },
}

fn print_property_changes(property_changes: &[PropertyChange], tree_depth: usize) {
    let mut items = Vec::new();
    for change in property_changes {
        let container_change =
            change
                .name
                .split_once('.')
                .and_then(|(container, key)| match container {
                    "Attributes" => Some(("Attributes", key)),
                    "Tags" => Some(("Tags", key)),
                    _ => None,
                });
        let Some((container, key)) = container_change else {
            items.push(PropertyTreeItem::Change(change));
            continue;
        };

        let existing = items.iter_mut().find(|item| {
            matches!(
                item,
                PropertyTreeItem::Container { name, .. } if *name == container
            )
        });
        if let Some(PropertyTreeItem::Container { changes, .. }) = existing {
            changes.push((key, change));
        } else {
            items.push(PropertyTreeItem::Container {
                name: container,
                changes: vec![(key, change)],
            });
        }
    }

    for item in items {
        match item {
            PropertyTreeItem::Change(change) => {
                print_property_change(change, &change.name, tree_depth + 2);
            }
            PropertyTreeItem::Container { name, changes } => {
                println!("{}{}", "  ".repeat(tree_depth + 2), name.dimmed());
                for (key, change) in changes {
                    print_property_change(change, key, tree_depth + 3);
                }
            }
        }
    }
}

fn print_property_change(change: &PropertyChange, name: &str, indent_level: usize) {
    println!("{}{}:", "  ".repeat(indent_level), name.yellow());
    let value_indent = "  ".repeat(indent_level + 1);
    if let Some(old) = &change.old_value {
        println!(
            "{}{} {}",
            value_indent,
            "-".red(),
            format_property_value(old).red()
        );
    }
    if let Some(new) = &change.new_value {
        println!(
            "{}{} {}",
            value_indent,
            "+".green(),
            format_property_value(new).green()
        );
    }
}

/// Format a PropertyValue for human-readable display.
fn format_property_value(v: &PropertyValue) -> String {
    match v {
        PropertyValue::Nil => "nil".to_string(),
        PropertyValue::Bool { value } => value.to_string(),
        PropertyValue::Int32 { value } => value.to_string(),
        PropertyValue::Int64 { value } => value.to_string(),
        PropertyValue::Float32 { value } => value.to_string(),
        PropertyValue::Float64 { value } => value.to_string(),
        PropertyValue::String { value } => {
            if value.len() > 50 {
                format!("\"{}...\"", &value[..47])
            } else {
                format!("\"{}\"", value)
            }
        }
        PropertyValue::BinaryString { len } => format!("<binary {} bytes>", len),
        PropertyValue::Ref { value } => format!("Ref({})", &value[..8.min(value.len())]),
        PropertyValue::Vector2 { x, y } => format!("({}, {})", x, y),
        PropertyValue::Vector3 { x, y, z } => format!("({}, {}, {})", x, y, z),
        PropertyValue::CFrame(value) => format_cframe_value(value),
        PropertyValue::Color3 { r, g, b } => format!("Color3({}, {}, {})", r, g, b),
        PropertyValue::BrickColor { value } => format!("BrickColor({})", value),
        PropertyValue::Enum { value } => format!("Enum({})", value),
        PropertyValue::UDim { scale, offset } => format!("UDim({}, {})", scale, offset),
        PropertyValue::UDim2 {
            x_scale,
            x_offset,
            y_scale,
            y_offset,
        } => {
            format!(
                "UDim2({{{}, {}}}, {{{}, {}}})",
                x_scale, x_offset, y_scale, y_offset
            )
        }
        PropertyValue::NumberRange { min, max } => format!("NumberRange({}, {})", min, max),
        PropertyValue::NumberSequence { keypoints } => {
            format!("NumberSequence({} keypoints)", keypoints.len())
        }
        PropertyValue::ColorSequence { keypoints } => {
            format!("ColorSequence({} keypoints)", keypoints.len())
        }
        PropertyValue::Rect {
            min_x,
            min_y,
            max_x,
            max_y,
        } => {
            format!("Rect({}, {}, {}, {})", min_x, min_y, max_x, max_y)
        }
        PropertyValue::Other { type_name } => format!("<{}>", type_name),
    }
}

/// One CFrame component, shortest round-trip. Rust's `{}` already emits the
/// shortest exact representation (no trailing zeros); on top of that we snap
/// components within 1e-6 of an integer so a rotation matrix's float dust
/// (`5e-13`, `0.9999999`) reads as the clean `0`/`1`/`-1` it represents.
/// Real values — a `-0.99984974` translation, `0.7071068`, `-512.8176` —
/// are nowhere near an integer boundary and print exactly.
fn fmt_component(v: f32) -> String {
    let rounded = v.round();
    let v = if (v - rounded).abs() < 1e-6 { rounded } else { v };
    if v == 0.0 {
        "0".to_string()
    } else {
        format!("{v}")
    }
}

/// Matches `fmt_component`'s snap tolerance, so the collapse decision and the
/// per-component rounding never disagree: a rotation block collapses only when
/// every component would itself snap to the identity it represents. A genuine
/// sub-degree rotation (e.g. a `0.00001` off-diagonal) stays and prints full.
const ROTATION_EPSILON: f32 = 1e-6;

/// True when the rotation block (components 3..12, row-major) is the identity
/// within tolerance — i.e. a pure translation, no meaningful turn.
fn rotation_is_identity(c: &[f32; 12]) -> bool {
    (c[3] - 1.0).abs() < ROTATION_EPSILON
        && (c[7] - 1.0).abs() < ROTATION_EPSILON
        && (c[11] - 1.0).abs() < ROTATION_EPSILON
        && [c[4], c[5], c[6], c[8], c[9], c[10]]
            .iter()
            .all(|n| n.abs() < ROTATION_EPSILON)
}

/// Trimmed CFrame: `-0` normalized, and the identity rotation matrix dropped
/// so a pure translation reads as `CFrame(x, y, z)`. Used for every CFrame the
/// diff prints — property values and rigid deltas alike.
fn format_cframe_value(value: &CFrameValue) -> String {
    let c = &value.components;
    if rotation_is_identity(c) {
        format!(
            "CFrame({}, {}, {})",
            fmt_component(c[0]),
            fmt_component(c[1]),
            fmt_component(c[2])
        )
    } else {
        let parts: Vec<String> = c.iter().map(|&n| fmt_component(n)).collect();
        format!("CFrame({})", parts.join(", "))
    }
}

/// A rigid delta for the pivoted diff rows and the merge/normalization
/// summaries: the same trimmed CFrame with a Δ marker.
pub fn format_delta(value: &CFrameValue) -> String {
    format!("\u{394} {}", format_cframe_value(value))
}

fn print_summary(diffs: &[DiffEntry]) {
    let mut added = 0;
    let mut removed = 0;
    let mut modified = 0;
    let mut reparented = 0;
    let mut pivoted = 0;

    for diff in diffs {
        match diff {
            DiffEntry::Added { .. } => added += 1,
            DiffEntry::Removed { .. } => removed += 1,
            DiffEntry::Modified { .. } => modified += 1,
            DiffEntry::Reparented { .. } => reparented += 1,
            DiffEntry::Pivoted { .. } => pivoted += 1,
        }
    }

    if added == 0 && removed == 0 && modified == 0 && reparented == 0 && pivoted == 0 {
        println!("No differences found.");
    } else {
        println!(
            "{} added, {} removed, {} modified, {} reparented, {} pivoted",
            added.to_string().green(),
            removed.to_string().red(),
            modified.to_string().yellow(),
            reparented.to_string().cyan(),
            pivoted.to_string().cyan(),
        );
    }
}

fn print_summary_line(
    added: &[&DiffEntry],
    removed: &[&DiffEntry],
    modified: &[&DiffEntry],
    reparented: &[&DiffEntry],
    pivoted: &[&DiffEntry],
) {
    println!(
        "{}: {} added, {} removed, {} modified, {} reparented, {} pivoted",
        "Summary".bold(),
        added.len().to_string().green(),
        removed.len().to_string().red(),
        modified.len().to_string().yellow(),
        reparented.len().to_string().cyan(),
        pivoted.len().to_string().cyan(),
    );
}

/// Counts per change kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiffCounts {
    pub added: usize,
    pub removed: usize,
    pub modified: usize,
    pub reparented: usize,
    pub pivoted: usize,
}

impl DiffCounts {
    pub fn of(diffs: &[DiffEntry]) -> Self {
        let mut counts = Self::default();
        for diff in diffs {
            match diff {
                DiffEntry::Added { .. } => counts.added += 1,
                DiffEntry::Removed { .. } => counts.removed += 1,
                DiffEntry::Modified { .. } => counts.modified += 1,
                DiffEntry::Reparented { .. } => counts.reparented += 1,
                DiffEntry::Pivoted { .. } => counts.pivoted += 1,
            }
        }
        counts
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Markdown table cell: paths and values may contain `|`, backticks, or
/// newlines, any of which would break the table.
fn md_cell(text: &str) -> String {
    let escaped: String = text
        .chars()
        .map(|c| match c {
            '|' => "\\|".to_string(),
            '\n' | '\r' => " ".to_string(),
            '`' => "\u{2019}".to_string(),
            other => other.to_string(),
        })
        .collect();
    escaped
}

fn md_code(text: &str) -> String {
    format!("`{}`", md_cell(text))
}

/// One collapsible table section. Small sections start open so a reader
/// sees them without a click; large ones collapse.
fn md_section(out: &mut String, title: &str, header: &[&str], rows: &[Vec<String>], max_rows: usize) {
    if rows.is_empty() {
        return;
    }
    let open = if rows.len() <= 10 { " open" } else { "" };
    out.push_str(&format!(
        "<details{open}><summary><b>{title}</b> ({})</summary>\n\n",
        rows.len()
    ));
    out.push_str(&format!("| {} |\n", header.join(" | ")));
    out.push_str(&format!("|{}|\n", header.iter().map(|_| "---").collect::<Vec<_>>().join("|")));
    for row in rows.iter().take(max_rows) {
        out.push_str(&format!("| {} |\n", row.join(" | ")));
    }
    if rows.len() > max_rows {
        out.push_str(&format!("\n_… and {} more_\n", rows.len() - max_rows));
    }
    out.push_str("\n</details>\n\n");
}

/// Render a diff as GitHub-flavored markdown: a bold count line, then one
/// collapsible table per change kind. No heading — callers that render
/// several files add their own.
pub fn render_markdown(diffs: &[DiffEntry], max_rows: usize) -> String {
    let counts = DiffCounts::of(diffs);
    let mut out = String::new();
    if counts.is_empty() {
        out.push_str("_No semantic differences._\n\n");
        return out;
    }
    out.push_str(&format!(
        "**{} added · {} removed · {} modified · {} reparented · {} pivoted**\n\n",
        counts.added, counts.removed, counts.modified, counts.reparented, counts.pivoted
    ));

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();
    let mut reparented = Vec::new();
    let mut pivoted = Vec::new();
    for diff in diffs {
        match diff {
            DiffEntry::Added { path, class, .. } => {
                added.push(vec![md_code(path), md_cell(class)]);
            }
            DiffEntry::Removed { path, class, .. } => {
                removed.push(vec![md_code(path), md_cell(class)]);
            }
            DiffEntry::Modified {
                path,
                property_changes,
                ..
            } => {
                for change in property_changes {
                    let before = change
                        .old_value
                        .as_ref()
                        .map(format_property_value)
                        .unwrap_or_else(|| "—".to_string());
                    let after = change
                        .new_value
                        .as_ref()
                        .map(format_property_value)
                        .unwrap_or_else(|| "—".to_string());
                    modified.push(vec![
                        md_code(path),
                        md_cell(&change.name),
                        md_code(&before),
                        md_code(&after),
                    ]);
                }
            }
            DiffEntry::Reparented {
                old_path,
                path,
                class,
                ..
            } => {
                reparented.push(vec![md_code(old_path), md_code(path), md_cell(class)]);
            }
            DiffEntry::Pivoted {
                path, class, delta, ..
            } => {
                pivoted.push(vec![
                    md_code(path),
                    md_cell(class),
                    md_code(&format_delta(delta)),
                ]);
            }
        }
    }
    md_section(&mut out, "Pivoted", &["Model", "Class", "Delta"], &pivoted, max_rows);
    md_section(&mut out, "Added", &["Instance", "Class"], &added, max_rows);
    md_section(&mut out, "Removed", &["Instance", "Class"], &removed, max_rows);
    md_section(&mut out, "Reparented", &["From", "To", "Class"], &reparented, max_rows);
    md_section(
        &mut out,
        "Modified",
        &["Instance", "Property", "Before", "After"],
        &modified,
        max_rows,
    );
    out
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
        reparented: usize,
        pivoted: usize,
    }

    let added = diffs
        .iter()
        .filter(|d| matches!(d, DiffEntry::Added { .. }))
        .count();
    let removed = diffs
        .iter()
        .filter(|d| matches!(d, DiffEntry::Removed { .. }))
        .count();
    let modified = diffs
        .iter()
        .filter(|d| matches!(d, DiffEntry::Modified { .. }))
        .count();
    let reparented = diffs
        .iter()
        .filter(|d| matches!(d, DiffEntry::Reparented { .. }))
        .count();
    let pivoted = diffs
        .iter()
        .filter(|d| matches!(d, DiffEntry::Pivoted { .. }))
        .count();

    let output = Output {
        changes: diffs,
        summary: Summary {
            added,
            removed,
            modified,
            reparented,
            pivoted,
        },
    };

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_output_preserves_sub_millistud_positions() {
        let value = PropertyValue::CFrame(CFrameValue {
            components: [
                1.00001, 2.0, 3.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0,
            ],
        });

        assert!(format_property_value(&value).contains("1.00001"));
    }

    #[test]
    fn pretty_output_uses_raw_cframe_components() {
        let value = PropertyValue::CFrame(CFrameValue {
            components: [
                1.0, 2.0, 3.0, 1.0, 0.00001, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0,
            ],
        });

        assert_eq!(
            format_property_value(&value),
            "CFrame(1, 2, 3, 1, 0.00001, 0, 0, 1, 0, 0, 0, 1)"
        );
    }

    #[test]
    fn json_output_uses_flat_cframe_components() {
        let value = PropertyValue::CFrame(CFrameValue {
            components: [1.0, 2.0, 3.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        });

        let json = serde_json::to_value(value).unwrap();
        assert_eq!(json["type"], "c_frame");
        assert_eq!(json["value"].as_array().unwrap().len(), 12);
    }

}
