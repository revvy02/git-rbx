//! Instance matching between two DOMs.
//!
//! Matching strategy:
//! 1. Single-candidate name match (unique name = instant match)
//! 2. Multi-candidate name groups with hash tiebreaking:
//!    - Pass 1: Full property hash (exact match)
//!    - Pass 2: No-refs hash (matches when only Ref properties changed)
//!    - Pass 3: Positional fallback (pair remaining by sibling order)
//! 3. Class-based fallback for remaining unmatched (catches renames)

use rbx_dom_weak::{types::Ref, WeakDom};
use std::collections::HashMap;
use tracing::{debug, info};

use crate::hash::LazyHashCache;

/// Result of matching instances between two DOMs.
#[derive(Debug, Clone)]
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
    let old_children: Vec<_> = old_parent_inst
        .children()
        .iter()
        .filter_map(|&r| {
            old_dom.get_by_ref(r).map(|inst| ChildInfo {
                referent: r,
                name: inst.name.clone(),
                class: inst.class.to_string(),
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
                class: inst.class.to_string(),
            })
        })
        .collect();

    let mut matched = Vec::new();
    let mut added = Vec::new();
    let mut hash_tiebreaks = 0usize;
    let mut unique_matches = 0usize;

    let old_count = old_children.len();
    let new_count = new_children.len();

    let mut old_matched = vec![false; old_count];
    let mut new_matched = vec![false; new_count];

    // Build name → indices map for O(1) lookup instead of O(n) scan
    let mut name_index: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, child) in old_children.iter().enumerate() {
        name_index.entry(child.name.as_str()).or_default().push(i);
    }

    // ===== Single-candidate matching (no ambiguity) =====
    for (new_idx, new_child) in new_children.iter().enumerate() {
        let candidates: Vec<usize> = name_index
            .get(new_child.name.as_str())
            .map(|indices| {
                indices.iter().copied()
                    .filter(|&i| !old_matched[i])
                    .collect()
            })
            .unwrap_or_default();

        if candidates.len() == 1 {
            unique_matches += 1;
            let idx = candidates[0];
            old_matched[idx] = true;
            new_matched[new_idx] = true;
            matched.push((old_children[idx].referent, new_child.referent));
        }
    }

    // ===== Multi-candidate matching (multi-pass tiebreaking) =====
    // Group unmatched new children by name for batch processing
    let mut name_groups: HashMap<&str, Vec<usize>> = HashMap::new();
    for (new_idx, new_child) in new_children.iter().enumerate() {
        if !new_matched[new_idx] {
            let has_candidates = name_index
                .get(new_child.name.as_str())
                .map(|indices| indices.iter().any(|&i| !old_matched[i]))
                .unwrap_or(false);
            if has_candidates {
                name_groups.entry(new_child.name.as_str()).or_default().push(new_idx);
            }
        }
    }

    for (name, new_indices) in &name_groups {
        hash_tiebreaks += new_indices.len();

        // Collect unmatched old candidates for this name group
        let old_candidates: Vec<usize> = name_index
            .get(name)
            .map(|indices| {
                indices.iter().copied()
                    .filter(|&i| !old_matched[i])
                    .collect()
            })
            .unwrap_or_default();

        let mut pass1_count = 0usize;
        let mut pass2_count = 0usize;
        let mut pass3_count = 0usize;

        // Pass 1: Full hash match (all properties including Refs)
        let mut remaining_new: Vec<usize> = Vec::new();
        for &new_idx in new_indices {
            let new_hash = new_hashes.get(new_children[new_idx].referent);
            let new_hash_bytes = *new_hash.as_bytes();

            let exact = old_candidates.iter().find(|&&oi| {
                !old_matched[oi] && {
                    let old_hash = old_hashes.get(old_children[oi].referent);
                    *old_hash.as_bytes() == new_hash_bytes
                }
            });

            if let Some(&oi) = exact {
                old_matched[oi] = true;
                new_matched[new_idx] = true;
                matched.push((old_children[oi].referent, new_children[new_idx].referent));
                pass1_count += 1;
            } else {
                remaining_new.push(new_idx);
            }
        }

        // Pass 2: No-refs hash match (stable when only Ref properties changed)
        let mut still_remaining: Vec<usize> = Vec::new();
        for new_idx in remaining_new {
            let new_hash_nr = new_hashes.get_no_refs(new_children[new_idx].referent);
            let new_hash_nr_bytes = *new_hash_nr.as_bytes();

            let nr_match = old_candidates.iter().find(|&&oi| {
                !old_matched[oi] && {
                    let old_hash_nr = old_hashes.get_no_refs(old_children[oi].referent);
                    *old_hash_nr.as_bytes() == new_hash_nr_bytes
                }
            });

            if let Some(&oi) = nr_match {
                old_matched[oi] = true;
                new_matched[new_idx] = true;
                matched.push((old_children[oi].referent, new_children[new_idx].referent));
                pass2_count += 1;
            } else {
                still_remaining.push(new_idx);
            }
        }

        // Pass 3: Positional fallback — pair remaining by order
        let mut remaining_old: Vec<usize> = old_candidates.iter()
            .copied()
            .filter(|&oi| !old_matched[oi])
            .collect();

        for new_idx in still_remaining {
            if let Some(oi) = remaining_old.first().copied() {
                remaining_old.remove(0);
                old_matched[oi] = true;
                new_matched[new_idx] = true;
                matched.push((old_children[oi].referent, new_children[new_idx].referent));
                pass3_count += 1;
            }
        }

        if pass2_count > 0 || pass3_count > 0 {
            let parent_name = old_dom.get_by_ref(old_parent).map(|i| i.name.as_str()).unwrap_or("?");
            debug!(
                parent = parent_name,
                name = *name,
                total = new_indices.len(),
                pass1_full_hash = pass1_count,
                pass2_no_refs = pass2_count,
                pass3_positional = pass3_count,
                "multi-pass tiebreak"
            );
        }
    }

    // ===== Class-based fallback (catches renames like "Signs" → "Sign") =====
    // Group remaining unmatched by class and try hash-based matching
    let mut class_groups_old: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, child) in old_children.iter().enumerate() {
        if !old_matched[i] {
            class_groups_old.entry(child.class.as_str()).or_default().push(i);
        }
    }

    let mut class_groups_new: HashMap<&str, Vec<usize>> = HashMap::new();
    for (new_idx, new_child) in new_children.iter().enumerate() {
        if !new_matched[new_idx] {
            class_groups_new.entry(new_child.class.as_str()).or_default().push(new_idx);
        }
    }

    let mut class_fallback_count = 0usize;

    for (class_name, new_indices) in &class_groups_new {
        let old_candidates = match class_groups_old.get(class_name) {
            Some(indices) => indices.clone(),
            None => continue,
        };

        if old_candidates.is_empty() {
            continue;
        }

        // Same 3-pass tiebreaking as name groups
        // Pass 1: Full hash
        let mut remaining_new: Vec<usize> = Vec::new();
        for &new_idx in new_indices {
            let new_hash = new_hashes.get(new_children[new_idx].referent);
            let new_hash_bytes = *new_hash.as_bytes();

            let exact = old_candidates.iter().find(|&&oi| {
                !old_matched[oi] && {
                    let old_hash = old_hashes.get(old_children[oi].referent);
                    *old_hash.as_bytes() == new_hash_bytes
                }
            });

            if let Some(&oi) = exact {
                old_matched[oi] = true;
                new_matched[new_idx] = true;
                matched.push((old_children[oi].referent, new_children[new_idx].referent));
                class_fallback_count += 1;
            } else {
                remaining_new.push(new_idx);
            }
        }

        // Pass 2: No-refs hash
        let mut still_remaining: Vec<usize> = Vec::new();
        for new_idx in remaining_new {
            let new_hash_nr = new_hashes.get_no_refs(new_children[new_idx].referent);
            let new_hash_nr_bytes = *new_hash_nr.as_bytes();

            let nr_match = old_candidates.iter().find(|&&oi| {
                !old_matched[oi] && {
                    let old_hash_nr = old_hashes.get_no_refs(old_children[oi].referent);
                    *old_hash_nr.as_bytes() == new_hash_nr_bytes
                }
            });

            if let Some(&oi) = nr_match {
                old_matched[oi] = true;
                new_matched[new_idx] = true;
                matched.push((old_children[oi].referent, new_children[new_idx].referent));
                class_fallback_count += 1;
            } else {
                still_remaining.push(new_idx);
            }
        }

        // Pass 3: Positional fallback
        let mut remaining_old: Vec<usize> = old_candidates.iter()
            .copied()
            .filter(|&oi| !old_matched[oi])
            .collect();

        for new_idx in still_remaining {
            if let Some(oi) = remaining_old.first().copied() {
                remaining_old.remove(0);
                old_matched[oi] = true;
                new_matched[new_idx] = true;
                matched.push((old_children[oi].referent, new_children[new_idx].referent));
                class_fallback_count += 1;
            }
        }
    }

    if class_fallback_count > 0 {
        let parent_name = old_dom.get_by_ref(old_parent).map(|i| i.name.as_str()).unwrap_or("?");
        debug!(
            parent = parent_name,
            class_fallback = class_fallback_count,
            "class-based fallback matching"
        );
    }

    // Collect unmatched new children as added
    for (new_idx, new_child) in new_children.iter().enumerate() {
        if !new_matched[new_idx] {
            added.push(new_child.referent);
        }
    }

    // Collect unmatched old children as removed
    let removed: Vec<Ref> = old_children
        .iter()
        .enumerate()
        .filter(|(i, _)| !old_matched[*i])
        .map(|(_, c)| c.referent)
        .collect();

    // Log matching stats for non-trivial levels
    if old_count + new_count > 10 {
        let parent_name = old_dom.get_by_ref(old_parent).map(|i| i.name.as_str()).unwrap_or("?");
        info!(
            parent = parent_name,
            old_children = old_count,
            new_children = new_count,
            matched = matched.len(),
            unique = unique_matches,
            hash_tiebreaks = hash_tiebreaks,
            added = added.len(),
            removed = removed.len(),
            "match_children"
        );
    } else if hash_tiebreaks > 0 {
        let parent_name = old_dom.get_by_ref(old_parent).map(|i| i.name.as_str()).unwrap_or("?");
        debug!(
            parent = parent_name,
            old_children = old_count,
            new_children = new_count,
            hash_tiebreaks = hash_tiebreaks,
            "match_children (hash tiebreak)"
        );
    }

    MatchResult { matched, removed, added }
}

struct ChildInfo {
    referent: Ref,
    name: String,
    class: String,
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
