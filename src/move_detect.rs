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
//! Passes C/D extend the pools to *descendants*: an instance moved into a
//! newly-added subtree (e.g. grouping existing content under a fresh Model)
//! or out of a removed one still pairs. A node inside an already-paired
//! subtree is excluded — it moved as part of its ancestor.
//!
//! Known limitation: rename+move together is not detected (both hash
//! variants include the name).

use blake3::Hasher;
use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::Variant;
use std::collections::{HashMap, HashSet};
use tracing::{debug, info};

use crate::hash::{hash_variant, DeepHashCache};
use crate::property_semantics::{get_authored_properties, stable_content_identity};

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
    use std::cell::RefCell;

    let removed_count = removed.len();
    let added_count = added.len();
    let mut moves: Vec<(Ref, Ref)> = Vec::new();
    let claims_old = RefCell::new(Claims::default());
    let claims_new = RefCell::new(Claims::default());

    {
        let can_old = |r: Ref| !claims_old.borrow().conflicts(old_dom, r);
        let can_new = |n: Ref| !claims_new.borrow().conflicts(new_dom, n);
        let mut on_pair = |o: Ref, n: Ref| {
            claims_old.borrow_mut().claim(old_dom, o);
            claims_new.borrow_mut().claim(new_dom, n);
            moves.push((o, n));
        };

        // Pass A: exact deep-hash over roots (pure moves)
        pair_by_exact_hash(
            &removed,
            &added,
            old_deep,
            new_deep,
            &can_old,
            &can_new,
            &mut on_pair,
        );
    }
    let pass_a_count = moves.len();

    {
        let can_old = |r: Ref| !claims_old.borrow().conflicts(old_dom, r);
        let can_new = |n: Ref| !claims_new.borrow().conflicts(new_dom, n);
        let mut on_pair = |o: Ref, n: Ref| {
            claims_old.borrow_mut().claim(old_dom, o);
            claims_new.borrow_mut().claim(new_dom, n);
            moves.push((o, n));
        };

        // Pass B: same (name, class) similarity over roots (move + edit)
        pair_by_similarity(
            old_dom,
            new_dom,
            &removed,
            &added,
            old_deep,
            new_deep,
            &can_old,
            &can_new,
            &mut on_pair,
        );
    }
    let pass_b_count = moves.len() - pass_a_count;

    // Passes C/D: pair an unmatched boundary root with a node inside the other
    // side's unmatched tree. This detects content moved into a newly-added
    // group or out of a deleted folder. We deliberately never pair two proper
    // descendants here: identical generic Parts inside unrelated replacement
    // containers are copies, not evidence that one was moved into the other.
    // Claimed roots are not expanded: their contents moved with them.
    let leftover_removed: Vec<Ref> = removed
        .iter()
        .copied()
        .filter(|r| !claims_old.borrow().nodes.contains(r))
        .collect();
    let leftover_added: Vec<Ref> = added
        .iter()
        .copied()
        .filter(|a| !claims_new.borrow().nodes.contains(a))
        .collect();
    let old_pool = expand_with_descendants(old_dom, &leftover_removed);
    let new_pool = expand_with_descendants(new_dom, &leftover_added);

    {
        let can_old = |r: Ref| !claims_old.borrow().conflicts(old_dom, r);
        let can_new = |n: Ref| !claims_new.borrow().conflicts(new_dom, n);
        let mut on_pair = |o: Ref, n: Ref| {
            claims_old.borrow_mut().claim(old_dom, o);
            claims_new.borrow_mut().claim(new_dom, n);
            moves.push((o, n));
        };
        pair_by_exact_hash(
            &leftover_removed,
            &new_pool,
            old_deep,
            new_deep,
            &can_old,
            &can_new,
            &mut on_pair,
        );
        pair_by_exact_hash(
            &old_pool,
            &leftover_added,
            old_deep,
            new_deep,
            &can_old,
            &can_new,
            &mut on_pair,
        );
    }
    let pass_c_count = moves.len() - pass_a_count - pass_b_count;

    {
        let can_old = |r: Ref| !claims_old.borrow().conflicts(old_dom, r);
        let can_new = |n: Ref| !claims_new.borrow().conflicts(new_dom, n);
        let mut on_pair = |o: Ref, n: Ref| {
            claims_old.borrow_mut().claim(old_dom, o);
            claims_new.borrow_mut().claim(new_dom, n);
            moves.push((o, n));
        };
        pair_by_similarity(
            old_dom,
            new_dom,
            &leftover_removed,
            &new_pool,
            old_deep,
            new_deep,
            &can_old,
            &can_new,
            &mut on_pair,
        );
        pair_by_similarity(
            old_dom,
            new_dom,
            &old_pool,
            &leftover_added,
            old_deep,
            new_deep,
            &can_old,
            &can_new,
            &mut on_pair,
        );
    }
    let pass_d_count = moves.len() - pass_a_count - pass_b_count - pass_c_count;

    if !moves.is_empty() {
        info!(
            removed_in = removed_count,
            added_in = added_count,
            moves = moves.len(),
            exact = pass_a_count,
            similarity = pass_b_count,
            descendant_exact = pass_c_count,
            descendant_similarity = pass_d_count,
            "move detection"
        );
    }

    moves
}

/// Claimed pairing targets for one DOM side. Tracks the claimed nodes and,
/// incrementally, every ancestor of a claimed node — so "does this node
/// contain a claim" is a set lookup instead of a walk over all claims.
#[derive(Default)]
struct Claims {
    nodes: HashSet<Ref>,
    ancestors: HashSet<Ref>,
}

impl Claims {
    fn claim(&mut self, dom: &WeakDom, node: Ref) {
        self.nodes.insert(node);
        let mut current = dom
            .get_by_ref(node)
            .map(|i| i.parent())
            .unwrap_or_else(Ref::none);
        while let Some(inst) = dom.get_by_ref(current) {
            self.ancestors.insert(current);
            current = inst.parent();
        }
    }

    /// A node can't pair if a claimed pair already covers it (an ancestor
    /// moved, taking it along) or it covers a claimed pair (its descendant
    /// moved away — this subtree is no longer a coherent unit).
    fn conflicts(&self, dom: &WeakDom, node: Ref) -> bool {
        if self.ancestors.contains(&node) {
            return true;
        }
        let mut current = node;
        while let Some(inst) = dom.get_by_ref(current) {
            if self.nodes.contains(&current) {
                return true;
            }
            current = inst.parent();
        }
        false
    }
}

/// Exact deep-hash pairing: bucket the new-side pool, then claim one
/// candidate per old-side node. Passes A and C differ only in their pools
/// and claim predicates.
fn pair_by_exact_hash(
    old_pool: &[Ref],
    new_pool: &[Ref],
    old_deep: &DeepHashCache,
    new_deep: &DeepHashCache,
    can_claim_old: impl Fn(Ref) -> bool,
    can_claim_new: impl Fn(Ref) -> bool,
    mut on_pair: impl FnMut(Ref, Ref),
) {
    let mut new_by_hash: HashMap<[u8; 32], Vec<Ref>> = HashMap::new();
    for &n in new_pool {
        new_by_hash
            .entry(*new_deep.get(n).as_bytes())
            .or_default()
            .push(n);
    }
    for &o in old_pool {
        if !can_claim_old(o) {
            continue;
        }
        let hash = *old_deep.get(o).as_bytes();
        let Some(bucket) = new_by_hash.get_mut(&hash) else {
            continue;
        };
        let Some(pos) = bucket.iter().position(|&n| can_claim_new(n)) else {
            continue;
        };
        let n = bucket.swap_remove(pos);
        on_pair(o, n);
    }
}

/// Similarity pairing: bucket both pools by (name, class), score all pairs in
/// a bucket, then claim greedily from the highest score down. Passes B and D
/// differ only in their pools and claim predicates.
fn pair_by_similarity(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    old_pool: &[Ref],
    new_pool: &[Ref],
    old_deep: &DeepHashCache,
    new_deep: &DeepHashCache,
    can_claim_old: impl Fn(Ref) -> bool,
    can_claim_new: impl Fn(Ref) -> bool,
    mut on_pair: impl FnMut(Ref, Ref),
) {
    let mut old_by_key: HashMap<(String, String), Vec<Ref>> = HashMap::new();
    for &o in old_pool {
        if !can_claim_old(o) {
            continue;
        }
        if let Some(inst) = old_dom.get_by_ref(o) {
            old_by_key
                .entry((inst.name.to_string(), inst.class.to_string()))
                .or_default()
                .push(o);
        }
    }
    let mut new_by_key: HashMap<(String, String), Vec<Ref>> = HashMap::new();
    for &n in new_pool {
        if !can_claim_new(n) {
            continue;
        }
        if let Some(inst) = new_dom.get_by_ref(n) {
            new_by_key
                .entry((inst.name.to_string(), inst.class.to_string()))
                .or_default()
                .push(n);
        }
    }

    // Deterministic bucket order: HashMap iteration varies per process, and
    // equal-score ties would otherwise pair differently run to run.
    let mut keys: Vec<&(String, String)> = old_by_key.keys().collect();
    keys.sort();
    for key in keys {
        let old_group = &old_by_key[key];
        let Some(new_group) = new_by_key.get(key) else {
            continue;
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

        let mut scored: Vec<(f32, Ref, Ref)> = Vec::new();
        for &o in old_group {
            for &n in new_group {
                let old_instance = old_dom.get_by_ref(o).unwrap();
                let new_instance = new_dom.get_by_ref(n).unwrap();
                // Similarity is useful evidence only after strong identity
                // agrees. Otherwise many generic MeshPart properties can
                // outvote a different MeshContent and invent a destructive
                // move between unrelated pieces of geometry.
                if stable_content_identity(old_instance) != stable_content_identity(new_instance) {
                    continue;
                }
                let score = similarity(old_dom, new_dom, o, n, old_deep, new_deep);
                if score >= SIMILARITY_THRESHOLD {
                    scored.push((score, o, n));
                }
            }
        }
        // Score descending, then pool order (deterministic tree-walk order,
        // roots before descendants) so equal scores always pair the same way.
        let old_index: HashMap<Ref, usize> =
            old_group.iter().enumerate().map(|(i, &r)| (r, i)).collect();
        let new_index: HashMap<Ref, usize> =
            new_group.iter().enumerate().map(|(i, &r)| (r, i)).collect();
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap()
                .then_with(|| old_index[&a.1].cmp(&old_index[&b.1]))
                .then_with(|| new_index[&a.2].cmp(&new_index[&b.2]))
        });

        for (_, o, n) in scored {
            if !can_claim_old(o) || !can_claim_new(n) {
                continue;
            }
            on_pair(o, n);
        }
    }
}

/// Roots plus every node inside them, roots first (roots are preferred
/// pairing targets — a coherent subtree beats a fragment).
fn expand_with_descendants(dom: &WeakDom, roots: &[Ref]) -> Vec<Ref> {
    let mut pool: Vec<Ref> = roots.to_vec();
    for &root in roots {
        let mut stack: Vec<Ref> = dom
            .get_by_ref(root)
            .map(|i| i.children().to_vec())
            .unwrap_or_default();
        while let Some(node) = stack.pop() {
            pool.push(node);
            if let Some(inst) = dom.get_by_ref(node) {
                stack.extend_from_slice(inst.children());
            }
        }
    }
    pool
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

/// Fraction of authored properties with equal values (by variant hash).
/// Returns None when neither side has authored properties (no signal).
fn property_similarity(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    old_inst: &rbx_dom_weak::Instance,
    new_inst: &rbx_dom_weak::Instance,
) -> Option<f32> {
    let authored = get_authored_properties(old_inst.class.as_str());

    let mut total = 0usize;
    let mut equal = 0usize;

    for (name, old_value) in &old_inst.properties {
        if !authored.contains(name.as_str()) {
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
        if authored.contains(name.as_str()) && !old_inst.properties.contains_key(name) {
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
