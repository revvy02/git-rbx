//! Candidate-scoped structural refinement of instance references.
//!
//! Ordinary hashes resolve most sibling groups. This module is invoked only
//! for the ambiguous remainder and builds a graph from those candidate
//! subtrees instead of eagerly refining every reference in the place.

use blake3::Hash;
use rbx_dom_weak::types::Ref;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use crate::diff_dom::DomView;
use crate::hash::LazyHashCache;
use crate::reference_value::{visit_reference_edges, ReferenceEdge, ReferenceLocation};

pub(crate) struct ReferenceGraphMatcher<'a> {
    old_dom: &'a dyn DomView,
    new_dom: &'a dyn DomView,
    old_hashes: &'a LazyHashCache<'a>,
    new_hashes: &'a LazyHashCache<'a>,
}

impl<'a> ReferenceGraphMatcher<'a> {
    pub(crate) fn new(
        old_dom: &'a dyn DomView,
        new_dom: &'a dyn DomView,
        old_hashes: &'a LazyHashCache<'a>,
        new_hashes: &'a LazyHashCache<'a>,
    ) -> Self {
        Self {
            old_dom,
            new_dom,
            old_hashes,
            new_hashes,
        }
    }

    /// Return only mutual-unique exact graph matches for one ambiguous sibling
    /// group. Edited or symmetric candidates deliberately fall through to the
    /// matcher's remaining evidence passes.
    pub(crate) fn unique_matches(&self, old_roots: &[Ref], new_roots: &[Ref]) -> Vec<(Ref, Ref)> {
        if old_roots.is_empty() || new_roots.is_empty() {
            return Vec::new();
        }

        if !roots_may_contain_references(self.old_dom, old_roots)
            || !roots_may_contain_references(self.new_dom, new_roots)
        {
            return Vec::new();
        }
        let old = GraphSide::new(self.old_dom, self.old_hashes, old_roots);
        let new = GraphSide::new(self.new_dom, self.new_hashes, new_roots);
        if old.reference_edges == 0 || new.reference_edges == 0 {
            return Vec::new();
        }
        let old_len = old.referents.len();

        let mut base = old.base;
        base.extend(new.base);
        let mut reverse_edges = vec![Vec::new(); base.len()];
        add_reverse_edges(&mut reverse_edges, old.edges, 0);
        add_reverse_edges(&mut reverse_edges, new.edges, old_len);
        let colors = stable_partition(&base, &reverse_edges);

        let old_colors: HashMap<_, _> = old
            .referents
            .iter()
            .copied()
            .zip(colors[..old_len].iter().copied())
            .collect();
        let new_colors: HashMap<_, _> = new
            .referents
            .iter()
            .copied()
            .zip(colors[old_len..].iter().copied())
            .collect();

        let mut old_by_color: HashMap<u32, Vec<Ref>> = HashMap::new();
        let mut new_by_color: HashMap<u32, Vec<Ref>> = HashMap::new();
        for &root in old_roots {
            if let Some(&color) = old_colors.get(&root) {
                old_by_color.entry(color).or_default().push(root);
            }
        }
        for &root in new_roots {
            if let Some(&color) = new_colors.get(&root) {
                new_by_color.entry(color).or_default().push(root);
            }
        }

        let mut matches = Vec::new();
        for (color, old) in old_by_color {
            let Some(new) = new_by_color.get(&color) else {
                continue;
            };
            if old.len() == 1 && new.len() == 1 {
                matches.push((old[0], new[0]));
            }
        }
        matches
    }
}

fn roots_may_contain_references(dom: &dyn DomView, roots: &[Ref]) -> bool {
    roots.iter().copied().any(|root| {
        dom.subtree_reference_count(root)
            .map_or(true, |count| count != 0)
    })
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum GraphEdgeLabel {
    Child,
    ReferenceOut(ReferenceLocation),
    ReferenceIn(ReferenceLocation),
}

struct GraphSide {
    referents: Vec<Ref>,
    base: Vec<Hash>,
    edges: Vec<(usize, usize, GraphEdgeLabel)>,
    reference_edges: usize,
}

impl GraphSide {
    fn new(dom: &dyn DomView, hashes: &LazyHashCache<'_>, roots: &[Ref]) -> Self {
        // Candidate roots are siblings, so their subtrees are disjoint. Keep
        // traversal order stable for reproducible color numbering.
        let mut owned = Vec::new();
        let mut seen = HashSet::new();
        let mut pending: Vec<_> = roots.iter().rev().copied().collect();
        while let Some(referent) = pending.pop() {
            if !seen.insert(referent) {
                continue;
            }
            let Some(instance) = dom.get_by_ref(referent) else {
                continue;
            };
            owned.push(referent);
            pending.extend(instance.children().rev());
        }

        let mut referents = owned.clone();
        let mut indices: HashMap<_, _> = referents
            .iter()
            .enumerate()
            .map(|(index, &referent)| (referent, index))
            .collect();

        // Collect topology once. Targets outside the candidate subtrees enter
        // as shallow context nodes; their descendants cannot affect candidate
        // identity unless the target itself is one of the candidate contents.
        let mut raw_references: Vec<(Ref, Ref, ReferenceLocation)> = Vec::new();
        let mut context = Vec::new();
        for &referent in &owned {
            let instance = dom
                .get_by_ref(referent)
                .expect("candidate graph descendant disappeared");
            let mut record = |edge: &ReferenceEdge| {
                if let Some(target) = edge
                    .target
                    .filter(|target| dom.get_by_ref(*target).is_some())
                {
                    raw_references.push((referent, target, edge.location.clone()));
                    if !indices.contains_key(&target) && seen.insert(target) {
                        context.push(target);
                    }
                }
            };
            visit_reference_edges(instance.authored_properties(), |edge| record(&edge));
        }
        for target in context {
            indices.insert(target, referents.len());
            referents.push(target);
        }

        let base = referents
            .iter()
            .map(|&referent| {
                hashes.get_instance_no_refs(
                    dom.get_by_ref(referent)
                        .expect("active reference graph node disappeared"),
                )
            })
            .collect();
        let mut edges = Vec::new();
        for &parent in &owned {
            let parent_index = indices[&parent];
            let instance = dom
                .get_by_ref(parent)
                .expect("candidate graph parent disappeared");
            for child in instance.children() {
                if let Some(&child_index) = indices.get(&child) {
                    edges.push((parent_index, child_index, GraphEdgeLabel::Child));
                }
            }
        }
        let reference_edges = raw_references.len();
        for (owner, target, location) in raw_references {
            let owner = indices[&owner];
            let target = indices[&target];
            edges.push((
                owner,
                target,
                GraphEdgeLabel::ReferenceOut(location.clone()),
            ));
            edges.push((target, owner, GraphEdgeLabel::ReferenceIn(location)));
        }

        Self {
            referents,
            base,
            edges,
            reference_edges,
        }
    }
}

fn add_reverse_edges(
    reverse_edges: &mut [Vec<(GraphEdgeLabel, usize)>],
    edges: Vec<(usize, usize, GraphEdgeLabel)>,
    offset: usize,
) {
    for (source, target, label) in edges {
        reverse_edges[target + offset].push((label, source + offset));
    }
}

/// Compute the coarsest stable partition of the combined old/new graph.
///
/// This is worklist-based partition refinement rather than synchronous color
/// propagation. A long chain of constraints is therefore near-linear instead
/// of requiring one whole-graph pass per edge of graph diameter.
fn stable_partition(base: &[Hash], reverse_edges: &[Vec<(GraphEdgeLabel, usize)>]) -> Vec<u32> {
    let mut by_base: BTreeMap<[u8; 32], Vec<usize>> = BTreeMap::new();
    for (node, color) in base.iter().enumerate() {
        by_base.entry(*color.as_bytes()).or_default().push(node);
    }

    let mut cells: Vec<Vec<usize>> = by_base.into_values().collect();
    let mut color = vec![0usize; base.len()];
    for (cell, members) in cells.iter().enumerate() {
        for &node in members {
            color[node] = cell;
        }
    }

    let mut queue: VecDeque<usize> = (0..cells.len()).collect();
    let mut queued: HashSet<usize> = (0..cells.len()).collect();
    while let Some(splitter) = queue.pop_front() {
        queued.remove(&splitter);
        let splitter_members = cells[splitter].clone();

        let mut counts_by_label: BTreeMap<GraphEdgeLabel, HashMap<usize, usize>> = BTreeMap::new();
        for target in splitter_members {
            for (label, source) in &reverse_edges[target] {
                *counts_by_label
                    .entry(label.clone())
                    .or_default()
                    .entry(*source)
                    .or_default() += 1;
            }
        }

        for counts in counts_by_label.into_values() {
            let affected_cells: BTreeSet<_> = counts.keys().map(|node| color[*node]).collect();
            for affected_cell in affected_cells {
                let members = cells[affected_cell].clone();
                let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
                for node in members {
                    groups
                        .entry(counts.get(&node).copied().unwrap_or(0))
                        .or_default()
                        .push(node);
                }
                if groups.len() <= 1 {
                    continue;
                }

                let original_was_queued = queued.contains(&affected_cell);
                let mut parts: Vec<Vec<usize>> = groups.into_values().collect();
                let keep = parts
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, members)| members.len())
                    .map(|(index, _)| index)
                    .unwrap();
                let kept = parts.swap_remove(keep);
                cells[affected_cell] = kept;
                for &node in &cells[affected_cell] {
                    color[node] = affected_cell;
                }

                let mut new_cells = Vec::new();
                for part in parts {
                    let new_cell = cells.len();
                    for &node in &part {
                        color[node] = new_cell;
                    }
                    cells.push(part);
                    new_cells.push(new_cell);
                }

                if original_was_queued {
                    for new_cell in new_cells {
                        if queued.insert(new_cell) {
                            queue.push_back(new_cell);
                        }
                    }
                } else {
                    if queued.insert(affected_cell) {
                        queue.push_back(affected_cell);
                    }
                    for new_cell in new_cells {
                        if queued.insert(new_cell) {
                            queue.push_back(new_cell);
                        }
                    }
                }
            }
        }
    }

    color.into_iter().map(|value| value as u32).collect()
}
