//! rbx-diff: Compare two Roblox DOMs and report differences.

mod conflict_file;
mod diff;
mod dom_utils;
mod edit_script;
mod hash;
mod match_instances;
mod merge;
mod model_normalize;
mod move_detect;
pub mod output;
mod rigid_groups;
mod value_compare;

pub use conflict_file::{
    finalize, find_container, list_entries, mark_entry, mark_entry_custom, stamp_conflicts,
    stamp_rigid_groups, ConflictEntry, CONFLICT_TAG, CONTAINER_NAME, ENTRY_TAG,
};
pub use diff::{compute_diff, DiffConfig, DiffEntry, PropertyChange, PropertyValue};
pub use diff::{ColorKeypoint, NumberKeypoint};
pub use edit_script::{apply_edit_script, compute_edit_script, Anchor, EditOp, EditScript};
pub use merge::{merge_doms, ConflictKind, MergeConflict, MergeResult, MergeStats};
pub use model_normalize::{
    apply_model_frame, apply_model_frame_to_dom, model_frames_close, normalize_model_dom_to_base,
    normalize_model_merge_frames, ModelFrameDecision, ModelFrameMerge, ModelNormalization,
};
pub use rigid_groups::{detect_rigid_groups, RigidGroup};

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
