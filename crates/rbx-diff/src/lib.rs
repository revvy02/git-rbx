//! rbx-diff: Compare two Roblox DOMs and report differences.

mod diff;
mod hash;
mod match_instances;
pub mod output;

pub use diff::{compute_diff, DiffConfig, DiffEntry, PropertyChange, PropertyValue};
pub use diff::{ColorKeypoint, NumberKeypoint};

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
