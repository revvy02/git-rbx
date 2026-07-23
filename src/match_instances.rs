//! Instance matching between two DOMs.
//!
//! Matching strategy:
//! 1. Single-candidate (name, class) match (unique pair = instant match)
//! 2. Multi-candidate name groups with hash tiebreaking:
//!    - Pass 1: Full property hash (exact match)
//!    - Pass 2: No-refs hash (matches when only Ref properties changed)
//!    - Pass 3: Stable content identity (for example MeshPart.MeshContent)
//!    - Pass 4: Tolerance-aware mutual-unique property match
//!    - Pass 5: Positional fallback only within the same identity class
//! 3. Content-preserving class fallback for remaining unmatched renames

use rbx_dom_weak::types::Ref;
use std::collections::HashMap;
use tracing::{debug, info};

use crate::diff_dom::{DomView, InstanceView};
use crate::hash::{DeepHashCache, LazyHashCache};
use crate::property_semantics::{pairing_compatible, strong_content_key, PairingBasis};
use crate::value_compare::non_ref_variants_equal;

const MAX_TOLERANT_PAIRWISE: usize = 100_000;

fn tolerant_non_ref_properties_equal(old: InstanceView<'_>, new: InstanceView<'_>) -> bool {
    for (name, old_value) in old.authored_properties() {
        if matches!(old_value, rbx_types::Variant::Ref(_)) {
            continue;
        }
        let Some(new_value) = new.property(name) else {
            return false;
        };
        if matches!(new_value, rbx_types::Variant::Ref(_))
            || !non_ref_variants_equal(old_value, new_value)
        {
            return false;
        }
    }
    for (name, new_value) in new.authored_properties() {
        if matches!(new_value, rbx_types::Variant::Ref(_)) {
            continue;
        }
        if old.property(name).is_none() {
            return false;
        }
    }
    true
}

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

/// Shared immutable matching context for one DOM pair.
pub(crate) struct Matcher<'a> {
    old_dom: &'a dyn DomView,
    new_dom: &'a dyn DomView,
    old_hashes: &'a LazyHashCache<'a>,
    new_hashes: &'a LazyHashCache<'a>,
    old_deep: &'a DeepHashCache<'a>,
    new_deep: &'a DeepHashCache<'a>,
}

impl<'a> Matcher<'a> {
    pub(crate) fn new(
        old_dom: &'a dyn DomView,
        new_dom: &'a dyn DomView,
        old_hashes: &'a LazyHashCache<'a>,
        new_hashes: &'a LazyHashCache<'a>,
        old_deep: &'a DeepHashCache<'a>,
        new_deep: &'a DeepHashCache<'a>,
    ) -> Self {
        Self {
            old_dom,
            new_dom,
            old_hashes,
            new_hashes,
            old_deep,
            new_deep,
        }
    }

    /// Complete identity discovery visits every matched parent exactly once,
    /// so parent-pair results never need to outlive this call.
    pub(crate) fn match_children_once(&self, old_parent: Ref, new_parent: Ref) -> MatchResult {
        compute_child_matches(self, old_parent, new_parent)
    }
}

/// Match children of two parent instances. Hashes are computed lazily only
/// when multiple candidates share a name.
fn compute_child_matches(matcher: &Matcher<'_>, old_parent: Ref, new_parent: Ref) -> MatchResult {
    let old_dom = matcher.old_dom;
    let new_dom = matcher.new_dom;
    let old_hashes = matcher.old_hashes;
    let new_hashes = matcher.new_hashes;
    let old_deep = matcher.old_deep;
    let new_deep = matcher.new_deep;

    let old_parent_inst = old_dom.get_by_ref(old_parent).unwrap();
    let new_parent_inst = new_dom.get_by_ref(new_parent).unwrap();

    // Build list of old children with their info (no hash computed yet)
    let old_children: Vec<_> = old_parent_inst
        .children()
        .filter_map(|r| {
            old_dom.get_by_ref(r).map(|inst| ChildInfo {
                referent: r,
                instance: inst,
            })
        })
        .collect();

    // Build list of new children (no hash computed yet)
    let new_children: Vec<_> = new_parent_inst
        .children()
        .filter_map(|r| {
            new_dom.get_by_ref(r).map(|inst| ChildInfo {
                referent: r,
                instance: inst,
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
        name_index.entry(child.name()).or_default().push(i);
    }
    let mut new_name_class_counts: HashMap<(&str, &str), usize> = HashMap::new();
    for child in &new_children {
        *new_name_class_counts
            .entry((child.name(), child.class()))
            .or_default() += 1;
    }

    // ===== Single-candidate matching (no ambiguity) =====
    //
    // Names alone are not identities: a Model may be replaced by a Part with
    // the same name. Pairing those would emit the Part's properties onto the
    // Model (for example, Color3uint8 into Model.Color) and make an invalid
    // Roblox file. A direct name match must preserve the class and be unique
    // on both sides. Old-side uniqueness alone is insufficient: if two new
    // siblings converge on one name, whichever happens to occur first could
    // steal the sole old sibling before content-aware matching runs.
    for (new_idx, new_child) in new_children.iter().enumerate() {
        if new_name_class_counts
            .get(&(new_child.name(), new_child.class()))
            .copied()
            != Some(1)
        {
            continue;
        }
        let candidates: Vec<usize> = name_index
            .get(new_child.name())
            .map(|indices| {
                indices
                    .iter()
                    .copied()
                    .filter(|&i| {
                        !old_matched[i]
                            && pairing_compatible(
                                &old_children[i].instance,
                                &new_child.instance,
                                PairingBasis::AnchoredName,
                            )
                    })
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
    let mut name_groups: HashMap<(&str, &str), Vec<usize>> = HashMap::new();
    for (new_idx, new_child) in new_children.iter().enumerate() {
        if !new_matched[new_idx] {
            let has_candidates = name_index
                .get(new_child.name())
                .map(|indices| {
                    indices
                        .iter()
                        .any(|&i| !old_matched[i] && old_children[i].class() == new_child.class())
                })
                .unwrap_or(false);
            if has_candidates {
                name_groups
                    .entry((new_child.name(), new_child.class()))
                    .or_default()
                    .push(new_idx);
            }
        }
    }

    for ((name, class), new_indices) in &name_groups {
        hash_tiebreaks += new_indices.len();

        // Collect unmatched old candidates for this (name, class) group.
        let old_candidates: Vec<usize> = name_index
            .get(name)
            .map(|indices| {
                indices
                    .iter()
                    .copied()
                    .filter(|&i| !old_matched[i] && old_children[i].class() == *class)
                    .collect()
            })
            .unwrap_or_default();

        let mut pass1_count = 0usize;
        let mut pass2_count = 0usize;
        let mut identity_count = 0usize;
        let mut tolerant_count = 0usize;
        let mut positional_count = 0usize;

        // Pass 1: Full hash match (all properties including Refs)
        let mut remaining_new: Vec<usize> = Vec::new();
        for &new_idx in new_indices {
            let new_hash = new_hashes.get_instance(new_children[new_idx].instance);
            let new_hash_bytes = *new_hash.as_bytes();

            let exact = old_candidates.iter().find(|&&oi| {
                !old_matched[oi] && {
                    let old_hash = old_hashes.get_instance(old_children[oi].instance);
                    *old_hash.as_bytes() == new_hash_bytes
                        && pairing_compatible(
                            &old_children[oi].instance,
                            &new_children[new_idx].instance,
                            PairingBasis::ExactContent,
                        )
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
            let new_hash_nr = new_hashes.get_instance_no_refs(new_children[new_idx].instance);
            let new_hash_nr_bytes = *new_hash_nr.as_bytes();

            let nr_match = old_candidates.iter().find(|&&oi| {
                !old_matched[oi] && {
                    let old_hash_nr = old_hashes.get_instance_no_refs(old_children[oi].instance);
                    *old_hash_nr.as_bytes() == new_hash_nr_bytes
                        && pairing_compatible(
                            &old_children[oi].instance,
                            &new_children[new_idx].instance,
                            PairingBasis::ExactContent,
                        )
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

        // Pass 3: placement-independent authored identity. A reordered set of
        // same-named MeshParts may move every CFrame while retaining MeshContent;
        // matching those siblings by position would detach geometry from its
        // intended transform.
        let old_identities: HashMap<usize, String> = old_candidates
            .iter()
            .filter(|&&old_idx| !old_matched[old_idx])
            .filter_map(|&old_idx| {
                strong_content_key(&old_children[old_idx].instance)
                    .map(|identity| (old_idx, identity))
            })
            .collect();
        let new_identities: HashMap<usize, String> = still_remaining
            .iter()
            .filter_map(|&new_idx| {
                strong_content_key(&new_children[new_idx].instance)
                    .map(|identity| (new_idx, identity))
            })
            .collect();
        let mut old_by_identity: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut new_by_identity: HashMap<&str, Vec<usize>> = HashMap::new();
        for (&index, identity) in &old_identities {
            old_by_identity
                .entry(identity.as_str())
                .or_default()
                .push(index);
        }
        for (&index, identity) in &new_identities {
            new_by_identity
                .entry(identity.as_str())
                .or_default()
                .push(index);
        }
        for &new_idx in &still_remaining {
            let Some(identity) = new_identities.get(&new_idx) else {
                continue;
            };
            let (Some(old_indices), Some(new_indices)) = (
                old_by_identity.get(identity.as_str()),
                new_by_identity.get(identity.as_str()),
            ) else {
                continue;
            };
            if old_indices.len() != 1 || new_indices.len() != 1 {
                continue;
            }
            let old_idx = old_indices[0];
            if !pairing_compatible(
                &old_children[old_idx].instance,
                &new_children[new_idx].instance,
                PairingBasis::Inferred,
            ) {
                continue;
            }
            old_matched[old_idx] = true;
            new_matched[new_idx] = true;
            matched.push((
                old_children[old_idx].referent,
                new_children[new_idx].referent,
            ));
            identity_count += 1;
        }
        still_remaining.retain(|&new_idx| !new_matched[new_idx]);

        // Pass 4: Tolerance-aware content matching. Exact hashes intentionally
        // retain every finite bit, but pivot normalization can introduce
        // harmless float noise. Resolve only mutual-unique pairs here; truly
        // identical twins remain for the positional fallback.
        if old_candidates
            .len()
            .checked_mul(still_remaining.len())
            .is_some_and(|pairs| pairs <= MAX_TOLERANT_PAIRWISE)
        {
            loop {
                let remaining_old: Vec<usize> = old_candidates
                    .iter()
                    .copied()
                    .filter(|&oi| !old_matched[oi])
                    .collect();
                let mut edges: Vec<(usize, usize)> = Vec::new();
                let mut old_edges = vec![0usize; old_count];
                let mut new_edges = vec![0usize; new_count];
                for &old_idx in &remaining_old {
                    for &new_idx in &still_remaining {
                        let old_inst = old_children[old_idx].instance;
                        let new_inst = new_children[new_idx].instance;
                        if pairing_compatible(&old_inst, &new_inst, PairingBasis::Inferred)
                            && tolerant_non_ref_properties_equal(old_inst, new_inst)
                        {
                            edges.push((old_idx, new_idx));
                            old_edges[old_idx] += 1;
                            new_edges[new_idx] += 1;
                        }
                    }
                }
                let unique_pairs: Vec<(usize, usize)> = edges
                    .iter()
                    .copied()
                    .filter(|(old_idx, new_idx)| {
                        old_edges[*old_idx] == 1 && new_edges[*new_idx] == 1
                    })
                    .collect();
                if unique_pairs.is_empty() {
                    break;
                }
                for (old_idx, new_idx) in unique_pairs {
                    old_matched[old_idx] = true;
                    new_matched[new_idx] = true;
                    matched.push((
                        old_children[old_idx].referent,
                        new_children[new_idx].referent,
                    ));
                    tolerant_count += 1;
                }
                still_remaining.retain(|&new_idx| !new_matched[new_idx]);
            }
        }

        // Pass 5: positional fallback. Strongly identified instances may only
        // pair with the same identity. MeshParts with different content IDs
        // must not be arbitrarily paired just because they share a sibling
        // position. Instances without a stable key retain the legacy behavior.
        let mut remaining_old: Vec<usize> = old_candidates
            .iter()
            .copied()
            .filter(|&oi| !old_matched[oi])
            .collect();

        for new_idx in still_remaining {
            let old_position = remaining_old.iter().position(|old_idx| {
                pairing_compatible(
                    &old_children[*old_idx].instance,
                    &new_children[new_idx].instance,
                    PairingBasis::Inferred,
                )
            });
            if let Some(old_position) = old_position {
                let oi = remaining_old.remove(old_position);
                old_matched[oi] = true;
                new_matched[new_idx] = true;
                matched.push((old_children[oi].referent, new_children[new_idx].referent));
                positional_count += 1;
            }
        }

        if pass2_count > 0 || identity_count > 0 || tolerant_count > 0 || positional_count > 0 {
            let parent_name = old_dom
                .get_by_ref(old_parent)
                .map(|i| i.name())
                .unwrap_or("?");
            debug!(
                parent = parent_name,
                name = *name,
                total = new_indices.len(),
                pass1_full_hash = pass1_count,
                pass2_no_refs = pass2_count,
                pass3_identity = identity_count,
                pass4_tolerant = tolerant_count,
                pass5_positional = positional_count,
                "multi-pass tiebreak"
            );
        }
    }

    // ===== Class-based fallback (content-preserving renames) =====
    // Different names are not enough evidence of shared identity. Pair only a
    // unique old/new subtree whose deep content is identical when the root
    // name is omitted. This catches real renames without consuming unrelated
    // same-class additions by sibling position.
    let mut class_groups_old: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, child) in old_children.iter().enumerate() {
        if !old_matched[i] {
            class_groups_old.entry(child.class()).or_default().push(i);
        }
    }

    let mut class_groups_new: HashMap<&str, Vec<usize>> = HashMap::new();
    for (new_idx, new_child) in new_children.iter().enumerate() {
        if !new_matched[new_idx] {
            class_groups_new
                .entry(new_child.class())
                .or_default()
                .push(new_idx);
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

        // Pass 1: full deep content, ignoring only the candidate root's name.
        let mut old_by_hash: HashMap<[u8; 32], Vec<usize>> = HashMap::new();
        let mut new_by_hash: HashMap<[u8; 32], Vec<usize>> = HashMap::new();
        for &old_idx in &old_candidates {
            old_by_hash
                .entry(
                    *old_deep
                        .get_instance_without_name(old_children[old_idx].instance)
                        .as_bytes(),
                )
                .or_default()
                .push(old_idx);
        }
        for &new_idx in new_indices {
            new_by_hash
                .entry(
                    *new_deep
                        .get_instance_without_name(new_children[new_idx].instance)
                        .as_bytes(),
                )
                .or_default()
                .push(new_idx);
        }
        for (hash, old_indices) in &old_by_hash {
            let Some(new_indices) = new_by_hash.get(hash) else {
                continue;
            };
            if old_indices.len() != 1 || new_indices.len() != 1 {
                continue;
            }
            let old_idx = old_indices[0];
            let new_idx = new_indices[0];
            if !pairing_compatible(
                &old_children[old_idx].instance,
                &new_children[new_idx].instance,
                PairingBasis::ContentPreservingRename,
            ) {
                continue;
            }
            old_matched[old_idx] = true;
            new_matched[new_idx] = true;
            matched.push((
                old_children[old_idx].referent,
                new_children[new_idx].referent,
            ));
            class_fallback_count += 1;
        }

        // Pass 2: the same strong match while allowing Ref retargeting.
        let mut old_by_hash: HashMap<[u8; 32], Vec<usize>> = HashMap::new();
        let mut new_by_hash: HashMap<[u8; 32], Vec<usize>> = HashMap::new();
        for &old_idx in &old_candidates {
            if old_matched[old_idx] {
                continue;
            }
            old_by_hash
                .entry(
                    *old_deep
                        .get_instance_without_name_no_refs(old_children[old_idx].instance)
                        .as_bytes(),
                )
                .or_default()
                .push(old_idx);
        }
        for &new_idx in new_indices {
            if new_matched[new_idx] {
                continue;
            }
            new_by_hash
                .entry(
                    *new_deep
                        .get_instance_without_name_no_refs(new_children[new_idx].instance)
                        .as_bytes(),
                )
                .or_default()
                .push(new_idx);
        }
        for (hash, old_indices) in &old_by_hash {
            let Some(new_indices) = new_by_hash.get(hash) else {
                continue;
            };
            if old_indices.len() != 1 || new_indices.len() != 1 {
                continue;
            }
            let old_idx = old_indices[0];
            let new_idx = new_indices[0];
            if !pairing_compatible(
                &old_children[old_idx].instance,
                &new_children[new_idx].instance,
                PairingBasis::ContentPreservingRename,
            ) {
                continue;
            }
            old_matched[old_idx] = true;
            new_matched[new_idx] = true;
            matched.push((
                old_children[old_idx].referent,
                new_children[new_idx].referent,
            ));
            class_fallback_count += 1;
        }
    }

    if class_fallback_count > 0 {
        let parent_name = old_dom
            .get_by_ref(old_parent)
            .map(|i| i.name())
            .unwrap_or("?");
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
        let parent_name = old_dom
            .get_by_ref(old_parent)
            .map(|i| i.name())
            .unwrap_or("?");
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
        let parent_name = old_dom
            .get_by_ref(old_parent)
            .map(|i| i.name())
            .unwrap_or("?");
        debug!(
            parent = parent_name,
            old_children = old_count,
            new_children = new_count,
            hash_tiebreaks = hash_tiebreaks,
            "match_children (hash tiebreak)"
        );
    }

    MatchResult {
        matched,
        removed,
        added,
    }
}

struct ChildInfo<'a> {
    referent: Ref,
    instance: InstanceView<'a>,
}

impl ChildInfo<'_> {
    fn name(&self) -> &str {
        self.instance.name()
    }

    fn class(&self) -> &str {
        self.instance.class()
    }
}

/// Get the full path of an instance (e.g., "Workspace.Map.Building1")
pub(crate) fn get_instance_path(dom: &dyn DomView, referent: Ref) -> String {
    let segments = get_instance_path_segments(dom, referent);
    join_instance_path(&segments)
}

/// Get the individual instance names that form a full path.
///
/// Presentation code must retain these boundaries rather than splitting the
/// dot-joined display path, because Roblox instance names may contain dots.
pub(crate) fn get_instance_path_segments(dom: &dyn DomView, referent: Ref) -> Vec<(Ref, String)> {
    let mut parts = Vec::new();
    let mut current = referent;

    while let Some(inst) = dom.get_by_ref(current) {
        // Skip the DataModel root
        if inst.class() != "DataModel" {
            parts.push((current, inst.name().to_string()));
        }
        let parent = inst.parent();
        if parent.is_none() {
            break;
        }
        current = parent;
    }

    parts.reverse();
    parts
}

pub(crate) fn join_instance_path(segments: &[(Ref, String)]) -> String {
    let capacity = segments.iter().map(|(_, name)| name.len()).sum::<usize>()
        + segments.len().saturating_sub(1);
    let mut path = String::with_capacity(capacity);
    for (index, (_, name)) in segments.iter().enumerate() {
        if index > 0 {
            path.push('.');
        }
        path.push_str(name);
    }
    path
}
