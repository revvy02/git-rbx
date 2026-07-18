//! rbx-diff: Compare two Roblox DOMs and report differences.

mod conflict_file;
mod diff;
mod edit_script;
mod hash;
mod match_instances;
mod merge;
mod move_detect;
pub mod output;

pub use diff::{compute_diff, DiffConfig, DiffEntry, PropertyChange, PropertyValue};
pub use diff::{ColorKeypoint, NumberKeypoint};
pub use edit_script::{apply_edit_script, compute_edit_script, Anchor, EditOp, EditScript};
pub use conflict_file::{
    finalize, find_container, list_entries, mark_entry, stamp_conflicts, ConflictEntry,
    CONFLICT_TAG, CONTAINER_NAME, ENTRY_TAG,
};
pub use merge::{merge_doms, ConflictKind, MergeConflict, MergeResult, MergeStats};

use rbx_dom_weak::WeakDom;

/// Diff two WeakDoms and return the list of differences.
/// This is the main entry point for library usage.
pub fn diff_doms(old_dom: &WeakDom, new_dom: &WeakDom) -> Vec<DiffEntry> {
    diff_doms_with_config(old_dom, new_dom, &DiffConfig::default())
}

/// Diff two WeakDoms with custom configuration.
pub fn diff_doms_with_config(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    config: &DiffConfig,
) -> Vec<DiffEntry> {
    let old_hashes = hash::LazyHashCache::new(old_dom);
    let new_hashes = hash::LazyHashCache::new(new_dom);
    compute_diff(old_dom, new_dom, &old_hashes, &new_hashes, config)
}
