//! rbx-diff: Compare two Roblox DOMs and report differences.

mod compact_diff;
mod conflict_file;
mod diff;
mod diff_dom;
mod dom_utils;
mod edit_script;
mod explorer_tree;
mod hash;
mod match_instances;
mod merge;
mod model_normalize;
mod move_detect;
pub mod output;
mod placement;
mod property_semantics;
mod rigid_groups;
mod semantic_verify;
mod value_compare;

pub use conflict_file::{
    finalize, find_container, list_entries, mark_entry, mark_entry_custom, stamp_compact_conflicts,
    stamp_conflicts, stamp_model_frame_plan, stamp_rigid_groups, ConflictEntry, CONFLICT_TAG,
    CONTAINER_NAME, ENTRY_TAG, VIRTUAL_TREES_NAME,
};
pub use diff::{compute_diff, CFrameValue, DiffConfig, DiffEntry, PropertyChange, PropertyValue};
pub use diff::{ColorKeypoint, NumberKeypoint};
pub use diff_dom::DiffDom;
pub use edit_script::{
    apply_edit_script, compute_edit_script, compute_semantic_changes, Anchor, EditOp, EditScript,
    InstanceIdentity, SemanticChangeSet,
};
pub use merge::{
    merge_compact_doms, merge_compact_doms_with_matches,
    merge_compact_doms_with_matches_and_pivots, merge_doms, merge_doms_with_matches, ConflictKind,
    ConflictSide, MergeConflict, MergeResult, MergeStats,
};
pub use model_normalize::{
    apply_model_frame, apply_model_frame_plan, apply_model_frame_to_dom, model_frames_close,
    normalize_model_diff_frames, normalize_model_dom_to_base, normalize_model_merge_compact_frames,
    normalize_model_merge_frames, ModelFrameApplication, ModelFrameDiff, ModelFrameMerge,
    ModelNormalization,
};
pub use placement::{apply_pivot_ops, apply_pivot_ops_to_compact_branch, PivotOp};
pub use rigid_groups::{detect_rigid_groups, RigidGroup};
pub use semantic_verify::{verify_mesh_geometry, SemanticMismatch};

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
    let changes =
        edit_script::compute_semantic_changes_with_identity(old_dom, new_dom, config, None);
    diff::semantic_changes_to_diff(old_dom, new_dom, &changes)
}

/// Diff two model-asset DOMs using hierarchical frame factorization.
///
/// The new DOM is canonicalized in place. Inferred rigid movement is returned
/// as one [`DiffEntry::ModelFrame`] per affected boundary; ordinary entries
/// contain only residual authored changes.
pub fn diff_model_doms_with_config(
    old_dom: &WeakDom,
    new_dom: &mut WeakDom,
    config: &DiffConfig,
) -> (Vec<DiffEntry>, Option<ModelFrameDiff>) {
    diff_model_views_with_config(old_dom, new_dom, config)
}

/// Diff against a compact old-side snapshot while retaining a mutable new
/// WeakDom for model-frame canonicalization.
pub fn diff_model_compact_old_with_config(
    old_dom: &DiffDom,
    new_dom: &mut WeakDom,
    config: &DiffConfig,
) -> (Vec<DiffEntry>, Option<ModelFrameDiff>) {
    diff_model_views_with_config(old_dom, new_dom, config)
}

/// Diff two compact snapshots, mutating only existing world-space properties
/// on the new side while hierarchical model frames are factored.
pub fn diff_model_compact_doms_with_config(
    old_dom: &DiffDom,
    new_dom: &mut DiffDom,
    config: &DiffConfig,
) -> (Vec<DiffEntry>, Option<ModelFrameDiff>) {
    let normalization = model_normalize::prepare_model_diff_frames_view(old_dom, new_dom);
    let diffs = compact_diff::compute_compact_diff_with_identity(
        old_dom,
        new_dom,
        &normalization.identity,
        normalization.pivot_ops(),
        config,
    );
    finish_model_diff(diffs, normalization)
}

fn diff_model_views_with_config(
    old_dom: &dyn diff_dom::DomView,
    new_dom: &mut dyn diff_dom::DomViewMut,
    config: &DiffConfig,
) -> (Vec<DiffEntry>, Option<ModelFrameDiff>) {
    let normalization = model_normalize::prepare_model_diff_frames_view(old_dom, new_dom);
    let mut changes = edit_script::compute_semantic_changes_with_identity(
        old_dom,
        new_dom.as_view(),
        config,
        Some(&normalization.identity),
    );
    changes.pivots = normalization.pivots.clone();
    let diffs = diff::semantic_changes_to_diff(old_dom, new_dom.as_view(), &changes);
    finish_model_diff(diffs, normalization)
}

fn finish_model_diff(
    diffs: Vec<DiffEntry>,
    normalization: ModelFrameDiff,
) -> (Vec<DiffEntry>, Option<ModelFrameDiff>) {
    if normalization.pivots.is_empty() {
        return (diffs, None);
    }
    (diffs, Some(normalization))
}
