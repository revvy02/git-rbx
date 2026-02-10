//! Instance matching between two DOMs.
//! Uses name matching with hash as tiebreaker.

use rbx_dom_weak::{types::Ref, WeakDom};

use crate::hash::LazyHashCache;

/// Result of matching instances between two DOMs.
#[derive(Debug)]
pub struct MatchResult {
    /// Matched pairs: (old_ref, new_ref)
    pub matched: Vec<(Ref, Ref)>,
    /// Instances only in the old DOM (removed)
    pub removed: Vec<Ref>,
    /// Instances only in the new DOM (added)
    pub added: Vec<Ref>,
}

/// Match children of two parent instances.
/// Returns matched pairs, removed (old only), and added (new only).
/// Hashes are computed lazily - only when there are multiple candidates with the same name.
pub fn match_children(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    old_parent: Ref,
    new_parent: Ref,
    old_hashes: &LazyHashCache,
    new_hashes: &LazyHashCache,
) -> MatchResult {
    let old_parent_inst = old_dom.get_by_ref(old_parent).unwrap();
    let new_parent_inst = new_dom.get_by_ref(new_parent).unwrap();

    // Build list of old children with their info (no hash computed yet)
    let mut old_children: Vec<_> = old_parent_inst
        .children()
        .iter()
        .filter_map(|&r| {
            old_dom.get_by_ref(r).map(|inst| ChildInfo {
                referent: r,
                name: inst.name.clone(),
                matched: false,
            })
        })
        .collect();

    // Build list of new children (no hash computed yet)
    let new_children: Vec<_> = new_parent_inst
        .children()
        .iter()
        .filter_map(|&r| {
            new_dom.get_by_ref(r).map(|inst| ChildInfo {
                referent: r,
                name: inst.name.clone(),
                matched: false,
            })
        })
        .collect();

    let mut matched = Vec::new();
    let mut added = Vec::new();

    // Match each new child to an old child
    for new_child in &new_children {
        // Find all candidates with matching name
        let candidates: Vec<usize> = old_children
            .iter()
            .enumerate()
            .filter(|(_, old)| {
                !old.matched && old.name == new_child.name
            })
            .map(|(i, _)| i)
            .collect();

        if candidates.is_empty() {
            // No match found - this is a new instance
            added.push(new_child.referent);
        } else if candidates.len() == 1 {
            // Exactly one match - no hash needed!
            let idx = candidates[0];
            old_children[idx].matched = true;
            matched.push((old_children[idx].referent, new_child.referent));
        } else {
            // Multiple candidates - use hash to find best match (lazy compute)
            let new_hash = new_hashes.get(new_child.referent);
            let new_hash_bytes = *new_hash.as_bytes();

            // Try to find exact hash match
            let exact_match = candidates.iter().find(|&&idx| {
                let old_hash = old_hashes.get(old_children[idx].referent);
                *old_hash.as_bytes() == new_hash_bytes
            });

            if let Some(&idx) = exact_match {
                old_children[idx].matched = true;
                matched.push((old_children[idx].referent, new_child.referent));
            } else {
                // No exact hash match - use first available candidate
                let idx = candidates[0];
                old_children[idx].matched = true;
                matched.push((old_children[idx].referent, new_child.referent));
            }
        }
    }

    // Collect unmatched old children as removed
    let removed: Vec<Ref> = old_children
        .iter()
        .filter(|c| !c.matched)
        .map(|c| c.referent)
        .collect();

    MatchResult { matched, removed, added }
}

struct ChildInfo {
    referent: Ref,
    name: String,
    matched: bool,
}

/// Get the full path of an instance (e.g., "Workspace.Map.Building1")
pub fn get_instance_path(dom: &WeakDom, referent: Ref) -> String {
    let mut parts = Vec::new();
    let mut current = referent;

    while let Some(inst) = dom.get_by_ref(current) {
        // Skip the DataModel root
        if inst.class.as_str() != "DataModel" {
            parts.push(inst.name.to_string());
        }
        let parent = inst.parent();
        if parent.is_none() {
            break;
        }
        current = parent;
    }

    parts.reverse();
    parts.join(".")
}
