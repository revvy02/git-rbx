//! Global move detection: pairs removed and added subtree roots across parents.
//!
//! The per-parent matcher (match_instances) can only see siblings, so an instance
//! reparented elsewhere in the tree falls out as removed + added. This module
//! reconciles those pools globally, git-rename-detection style:
//!
//! - Pass A: exact deep-hash pairing — identical subtree content = pure move.
//! - Pass B: same (name, class) pairing with similarity scoring — move + edit.
//!
//! Pairs below the similarity threshold stay removed/added; a wrong move inference
//! is worse than a noisy one (it would silently relocate edits in a future merge).
//!
//! Known limitations (v1): an instance moved *out of* a subtree that was itself
//! removed is not detected (only subtree roots enter the pools), and rename+move
//! together is not detected (both hash variants include the name).

use blake3::Hasher;
use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::Variant;
use std::collections::HashMap;
use tracing::{debug, info};

use crate::hash::{get_comparable_properties, hash_variant, DeepHashCache};

/// Minimum similarity score for a Pass B pairing to count as a move.
const SIMILARITY_THRESHOLD: f32 = 0.5;

/// Cap on pairwise similarity computations per (name, class) bucket.
/// Buckets larger than this fall back to unpaired (logged).
const MAX_PAIRWISE: usize = 100_000;

/// Pair removed/added subtree roots into moves: (old_ref, new_ref).
/// Unpaired roots stay removed/added; the diff pass re-derives and reports them.
pub fn detect_moves(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    removed: Vec<Ref>,
    added: Vec<Ref>,
    old_deep: &DeepHashCache,
    new_deep: &DeepHashCache,
) -> Vec<(Ref, Ref)> {
    let removed_count = removed.len();
    let added_count = added.len();
    let mut moves: Vec<(Ref, Ref)> = Vec::new();

    // ===== Pass A: exact deep-hash pairing (pure moves) =====
    // Bucket added roots by deep hash; pop a candidate for each removed root.
    let mut added_by_hash: HashMap<[u8; 32], Vec<Ref>> = HashMap::new();
    for &a in &added {
        added_by_hash
            .entry(*new_deep.get(a).as_bytes())
            .or_default()
            .push(a);
    }

    let mut remaining_removed: Vec<Ref> = Vec::new();
    for r in removed {
        let hash = *old_deep.get(r).as_bytes();
        match added_by_hash.get_mut(&hash).and_then(|v| v.pop()) {
            Some(a) => moves.push((r, a)),
            None => remaining_removed.push(r),
        }
    }
    let remaining_added: Vec<Ref> = added_by_hash.into_values().flatten().collect();
    let pass_a_count = moves.len();

    // ===== Pass B: same (name, class) with similarity scoring (move + edit) =====
    let mut removed_by_key: HashMap<(String, String), Vec<Ref>> = HashMap::new();
    for &r in &remaining_removed {
        if let Some(inst) = old_dom.get_by_ref(r) {
            removed_by_key
                .entry((inst.name.to_string(), inst.class.to_string()))
                .or_default()
                .push(r);
        }
    }

    let mut added_by_key: HashMap<(String, String), Vec<Ref>> = HashMap::new();
    for &a in &remaining_added {
        if let Some(inst) = new_dom.get_by_ref(a) {
            added_by_key
                .entry((inst.name.to_string(), inst.class.to_string()))
                .or_default()
                .push(a);
        }
    }

    for (key, old_group) in &removed_by_key {
        let new_group = match added_by_key.get(key) {
            Some(g) => g,
            None => continue,
        };

        if old_group.len() * new_group.len() > MAX_PAIRWISE {
            debug!(
                name = %key.0,
                class = %key.1,
                old = old_group.len(),
                new = new_group.len(),
                "move detection bucket too large, skipping similarity scoring"
            );
            continue;
        }

        // Score all pairs, then pair greedily from the highest score down.
        let mut scored: Vec<(f32, Ref, Ref)> = Vec::new();
        for &o in old_group {
            for &n in new_group {
                let score = similarity(old_dom, new_dom, o, n, old_deep, new_deep);
                if score >= SIMILARITY_THRESHOLD {
                    scored.push((score, o, n));
                }
            }
        }
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        let mut used_old: Vec<Ref> = Vec::new();
        let mut used_new: Vec<Ref> = Vec::new();
        for (_, o, n) in scored {
            if used_old.contains(&o) || used_new.contains(&n) {
                continue;
            }
            used_old.push(o);
            used_new.push(n);
            moves.push((o, n));
        }
    }

    if !moves.is_empty() {
        info!(
            removed_in = removed_count,
            added_in = added_count,
            moves = moves.len(),
            exact = pass_a_count,
            similarity = moves.len() - pass_a_count,
            "move detection"
        );
    }

    moves
}

/// Similarity score in [0, 1] between a removed and an added instance of the
/// same name and class. Blends own-property equality with child-content overlap;
/// leaf instances are scored on properties alone.
fn similarity(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    old_ref: Ref,
    new_ref: Ref,
    old_deep: &DeepHashCache,
    new_deep: &DeepHashCache,
) -> f32 {
    let old_inst = match old_dom.get_by_ref(old_ref) {
        Some(i) => i,
        None => return 0.0,
    };
    let new_inst = match new_dom.get_by_ref(new_ref) {
        Some(i) => i,
        None => return 0.0,
    };

    let prop_score = property_similarity(old_dom, new_dom, old_inst, new_inst);

    let old_children = old_inst.children();
    let new_children = new_inst.children();
    let child_score = if old_children.is_empty() && new_children.is_empty() {
        None
    } else {
        // Two-tier child overlap: identical content (deep hash) scores full
        // credit, same name+class (an edited version of the child) scores half.
        struct OldChild {
            hash: [u8; 32],
            name: String,
            class: String,
            consumed: bool,
        }
        let mut old_infos: Vec<OldChild> = old_children
            .iter()
            .filter_map(|&c| {
                old_dom.get_by_ref(c).map(|inst| OldChild {
                    hash: *old_deep.get(c).as_bytes(),
                    name: inst.name.to_string(),
                    class: inst.class.to_string(),
                    consumed: false,
                })
            })
            .collect();

        let mut credit = 0.0f32;
        let mut identity_pending: Vec<(String, String)> = Vec::new();
        for &c in new_children {
            let hash = *new_deep.get(c).as_bytes();
            match old_infos.iter_mut().find(|o| !o.consumed && o.hash == hash) {
                Some(o) => {
                    o.consumed = true;
                    credit += 1.0;
                }
                None => {
                    if let Some(inst) = new_dom.get_by_ref(c) {
                        identity_pending.push((inst.name.to_string(), inst.class.to_string()));
                    }
                }
            }
        }
        for (name, class) in identity_pending {
            if let Some(o) = old_infos
                .iter_mut()
                .find(|o| !o.consumed && o.name == name && o.class == class)
            {
                o.consumed = true;
                credit += 0.5;
            }
        }

        Some(credit / old_children.len().max(new_children.len()) as f32)
    };

    // Blend whichever signals exist; no signal at all means no evidence to pair
    // (identical no-content instances were already paired by exact hash in Pass A).
    match (prop_score, child_score) {
        (Some(p), Some(c)) => 0.5 * p + 0.5 * c,
        (Some(p), None) => p,
        (None, Some(c)) => c,
        (None, None) => 0.0,
    }
}

/// Fraction of comparable properties with equal values (by variant hash).
/// Returns None when neither side has comparable properties (no signal).
fn property_similarity(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    old_inst: &rbx_dom_weak::Instance,
    new_inst: &rbx_dom_weak::Instance,
) -> Option<f32> {
    let comparable = get_comparable_properties(old_inst.class.as_str());

    let mut total = 0usize;
    let mut equal = 0usize;

    for (name, old_value) in &old_inst.properties {
        if !comparable.contains(name.as_str()) {
            continue;
        }
        total += 1;
        if let Some(new_value) = new_inst.properties.get(name) {
            if variant_hash_eq(old_dom, old_value, new_dom, new_value) {
                equal += 1;
            }
        }
    }
    // Properties only on the new side count against similarity
    for (name, _) in &new_inst.properties {
        if comparable.contains(name.as_str()) && !old_inst.properties.contains_key(name) {
            total += 1;
        }
    }

    if total == 0 {
        None
    } else {
        Some(equal as f32 / total as f32)
    }
}

fn variant_hash_eq(old_dom: &WeakDom, a: &Variant, new_dom: &WeakDom, b: &Variant) -> bool {
    let mut ha = Hasher::new();
    hash_variant(old_dom, &mut ha, a);
    let mut hb = Hasher::new();
    hash_variant(new_dom, &mut hb, b);
    ha.finalize() == hb.finalize()
}
