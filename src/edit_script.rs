//! Semantic change sets: storage-independent differences between two DOMs,
//! plus materialization helpers that apply them to a mutable result.
//!
//! This is the layer the merge combiner consumes. Ordinary basis ops are
//! AddSubtree / RemoveSubtree / SetName / SetProperty; Move is the derived
//! identity op (a paired remove+add). Hierarchical [`PivotOp`]s are another
//! primitive, kept in an ordered placement phase because they transform the
//! coordinate system in which ordinary edits were planned. Ops address
//! existing instances by their ref in the OLD dom and subtree payloads by
//! their ref in the NEW dom.
//!
//! Merge planning walks the complete identity tree so Ref-valued properties
//! can always be remapped at materialization time. Compact display diffing
//! produces the same semantic records through a dense changed-subtree pass.

use rbx_dom_weak::{types::Ref, InstanceBuilder, WeakDom};
use rbx_types::Variant;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::info;

use crate::diff::{is_studio_artifact, raw_property_changes, DiffConfig};
use crate::diff_dom::DomView;
use crate::hash::{DeepHashCache, LazyHashCache};
use crate::match_instances::Matcher;
use crate::move_detect::detect_moves;
use crate::placement::{apply_pivot_ops, PivotOp};
use crate::reference_value::{direct_reference, with_direct_reference_target};

/// Where a parent lives when an op needs one: an instance that exists in the
/// old DOM, or one that an AddSubtree op creates (addressed by its new-DOM ref).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    Old(Ref),
    Added(Ref),
}

#[derive(Debug, Clone)]
pub enum EditOp {
    /// Copy the subtree rooted at `new_ref` (in the new DOM) under `parent`.
    AddSubtree { parent: Anchor, new_ref: Ref },
    /// Delete the subtree rooted at this old-DOM instance.
    RemoveSubtree { old_ref: Ref },
    /// Reparent an old-DOM instance (derived identity op; lowers to
    /// RemoveSubtree + AddSubtree).
    Move { old_ref: Ref, new_parent: Anchor },
    /// Rename a matched instance (Name lives outside the property map).
    SetName { old_ref: Ref, name: String },
    /// Set (`Some`) or remove (`None`) a property on a matched instance.
    /// `value` variants are expressed in new-DOM terms; Ref values are
    /// remapped through the identity mapping at apply time.
    SetProperty {
        old_ref: Ref,
        name: String,
        /// Previous semantic value in old-DOM terms. Retained so display
        /// diffs and conflict impacts project from the same change record.
        old_value: Option<Variant>,
        value: Option<Variant>,
    },
}

/// Storage-independent semantic changes between two DOMs.
///
/// Planning only depends on [`DomView`]. A mutable [`WeakDom`] is required
/// later, when the changes are materialized. Keeping those phases separate
/// lets the two-way diff and three-way merge planners consume compact DOMs
/// without pulling editing concerns into matching and comparison.
pub struct SemanticChangeSet {
    /// Structural and property operations in canonical coordinates.
    pub ops: Vec<EditOp>,
    /// Parent-relative placements, applied after `ops` in top-down order.
    pub pivots: Vec<PivotOp>,
    pub identity: InstanceIdentity,
}

/// Complete bidirectional identity between two DOMs.
///
/// Every derived view is built once and shared by diff projection, merge
/// planning, materialization, and conflict stamping. Keeping forward and
/// reverse mappings together prevents each consumer from rebuilding the same
/// place-wide hash table.
#[derive(Debug, Clone)]
pub struct InstanceIdentity {
    /// Identity mapping (old_ref → new_ref) for every matched instance,
    /// including moved pairs and instances inside unchanged subtrees.
    pub matched: Arc<HashMap<Ref, Ref>>,
    /// Reverse identity mapping (new_ref → old_ref).
    pub reverse_matched: Arc<HashMap<Ref, Ref>>,
    /// Paired moved roots in identity-detection order.
    pub moves: Arc<Vec<(Ref, Ref)>>,
    /// Old-DOM roots supplied by Move operations.
    pub moved_old: Arc<HashSet<Ref>>,
    /// New-DOM refs that are Move destinations. A destination can sit inside
    /// an added subtree (moved into a new group); cloning that subtree must
    /// skip these positions — the Move op supplies the real instance.
    pub moved_new: Arc<HashSet<Ref>>,
}

/// Backwards-compatible name for the applicable semantic change set.
pub type EditScript = SemanticChangeSet;

impl InstanceIdentity {
    fn new(matched: HashMap<Ref, Ref>, moves: Vec<(Ref, Ref)>) -> Self {
        let reverse_matched = matched.iter().map(|(old, new)| (*new, *old)).collect();
        let moved_old = moves.iter().map(|(old, _)| *old).collect();
        let moved_new = moves.iter().map(|(_, new)| *new).collect();
        Self {
            matched: Arc::new(matched),
            reverse_matched: Arc::new(reverse_matched),
            moves: Arc::new(moves),
            moved_old: Arc::new(moved_old),
            moved_new: Arc::new(moved_new),
        }
    }
}

/// Compute the edit script transforming `old_dom` into `new_dom`.
pub fn compute_edit_script(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    config: &DiffConfig,
) -> EditScript {
    compute_semantic_changes(old_dom, new_dom, config)
}

/// Compute the storage-independent changes from `old_dom` to `new_dom`.
pub fn compute_semantic_changes(
    old_dom: &WeakDom,
    new_dom: &WeakDom,
    config: &DiffConfig,
) -> SemanticChangeSet {
    compute_semantic_changes_with_identity(old_dom, new_dom, config, None)
}

/// Establish complete cross-DOM identity without constructing property ops.
/// Frame normalization needs this mapping before it rewrites representation;
/// avoiding the edit-emission pass matters for large place-file diffs.
pub(crate) fn compute_instance_identity(
    old_dom: &dyn DomView,
    new_dom: &dyn DomView,
    config: &DiffConfig,
) -> InstanceIdentity {
    let old_hashes = LazyHashCache::new_view(old_dom);
    let new_hashes = LazyHashCache::new_view(new_dom);
    let old_deep = DeepHashCache::new(old_dom, &config.ignore_properties);
    let new_deep = DeepHashCache::new(new_dom, &config.ignore_properties);
    let matcher = Matcher::new(
        old_dom,
        new_dom,
        &old_hashes,
        &new_hashes,
        &old_deep,
        &new_deep,
    );
    discover_identity_once(&matcher, old_dom, new_dom, &old_deep, &new_deep)
}

fn discover_identity_once(
    matcher: &Matcher<'_>,
    old_dom: &dyn DomView,
    new_dom: &dyn DomView,
    old_deep: &DeepHashCache,
    new_deep: &DeepHashCache,
) -> InstanceIdentity {
    let mut matched = HashMap::new();
    let mut removed_roots = Vec::new();
    let mut added_roots = Vec::new();
    build_full_mapping_once(
        matcher,
        old_dom.root_ref(),
        new_dom.root_ref(),
        &mut matched,
        &mut removed_roots,
        &mut added_roots,
    );

    let moves = detect_moves(
        old_dom,
        new_dom,
        removed_roots,
        added_roots,
        old_deep,
        new_deep,
    );
    for (old_root, new_root) in &moves {
        matched.insert(*old_root, *new_root);
        build_full_mapping_once(
            matcher,
            *old_root,
            *new_root,
            &mut matched,
            &mut Vec::new(),
            &mut Vec::new(),
        );
    }
    InstanceIdentity::new(matched, moves)
}

/// Per-DOM lazy hash caches shared across planning passes.
///
/// A three-way merge plans two scripts against the same base; sharing one
/// set of base caches means base subtrees are hashed once instead of once
/// per branch. Purely a memoization handle — safe to build cheaply and drop.
pub(crate) struct DomCaches<'a> {
    pub(crate) shallow: LazyHashCache<'a>,
    pub(crate) deep: DeepHashCache<'a>,
}

impl<'a> DomCaches<'a> {
    pub(crate) fn new(
        dom: &'a dyn DomView,
        ignore_properties: &'a rustc_hash::FxHashSet<String>,
    ) -> Self {
        Self {
            shallow: LazyHashCache::new_view(dom),
            deep: DeepHashCache::new(dom, ignore_properties),
        }
    }
}

/// Compute an edit script while preserving identity established before a
/// representation-only DOM canonicalization.
pub(crate) fn compute_semantic_changes_with_identity(
    old_dom: &dyn DomView,
    new_dom: &dyn DomView,
    config: &DiffConfig,
    pinned: Option<&InstanceIdentity>,
) -> SemanticChangeSet {
    let old_caches = DomCaches::new(old_dom, &config.ignore_properties);
    let new_caches = DomCaches::new(new_dom, &config.ignore_properties);
    compute_semantic_changes_with_caches(old_dom, new_dom, config, pinned, &old_caches, &new_caches)
}

/// Cache-sharing variant: the caller owns the per-DOM caches so several
/// planning passes over the same DOM reuse one memoization.
pub(crate) fn compute_semantic_changes_with_caches(
    old_dom: &dyn DomView,
    new_dom: &dyn DomView,
    config: &DiffConfig,
    pinned: Option<&InstanceIdentity>,
    old_caches: &DomCaches<'_>,
    new_caches: &DomCaches<'_>,
) -> SemanticChangeSet {
    let old_deep = &old_caches.deep;
    let new_deep = &new_caches.deep;

    let identity = if let Some(pinned) = pinned {
        pinned.clone()
    } else {
        let matcher = Matcher::new(
            old_dom,
            new_dom,
            &old_caches.shallow,
            &new_caches.shallow,
            old_deep,
            new_deep,
        );
        discover_identity_once(&matcher, old_dom, new_dom, old_deep, new_deep)
    };

    let mut ops = Vec::new();

    // Move ops first in new-side depth order (parents settle before children),
    // so applying in script order can never transfer into an unsettled spot.
    let mut moves_by_depth: Vec<(usize, &(Ref, Ref))> = identity
        .moves
        .iter()
        .map(|pair| (new_side_depth(new_dom, pair.1), pair))
        .collect();
    moves_by_depth.sort_by_key(|(depth, _)| *depth);
    for (_, (old_root, new_root)) in moves_by_depth {
        let new_parent = new_dom
            .get_by_ref(*new_root)
            .map(|inst| inst.parent())
            .unwrap_or_else(Ref::none);
        ops.push(EditOp::Move {
            old_ref: *old_root,
            new_parent: anchor_for(new_parent, &identity.reverse_matched),
        });
    }

    // Structural + property ops from the matched walk
    let ctx = BuildCtx {
        old_dom,
        new_dom,
        config,
        identity: &identity,
        old_deep,
        new_deep,
    };
    emit_ops(&ctx, old_dom.root_ref(), new_dom.root_ref(), &mut ops);
    for (old_root, new_root) in identity.moves.iter() {
        emit_instance_edits(&ctx, *old_root, *new_root, &mut ops);
        emit_ops(&ctx, *old_root, *new_root, &mut ops);
    }

    info!(
        ops = ops.len(),
        matched = identity.matched.len(),
        "edit script built"
    );
    SemanticChangeSet {
        ops,
        pivots: Vec::new(),
        identity,
    }
}

struct BuildCtx<'a> {
    old_dom: &'a dyn DomView,
    new_dom: &'a dyn DomView,
    config: &'a DiffConfig,
    identity: &'a InstanceIdentity,
    old_deep: &'a DeepHashCache<'a>,
    new_deep: &'a DeepHashCache<'a>,
}

fn build_full_mapping_once(
    matcher: &Matcher<'_>,
    old_ref: Ref,
    new_ref: Ref,
    mapping: &mut HashMap<Ref, Ref>,
    removed_roots: &mut Vec<Ref>,
    added_roots: &mut Vec<Ref>,
) {
    let result = matcher.match_children_once(old_ref, new_ref);
    removed_roots.extend_from_slice(&result.removed);
    added_roots.extend_from_slice(&result.added);
    for (old_child, new_child) in result.matched {
        mapping.insert(old_child, new_child);
        build_full_mapping_once(
            matcher,
            old_child,
            new_child,
            mapping,
            removed_roots,
            added_roots,
        );
    }
}

fn new_side_depth(new_dom: &dyn DomView, mut referent: Ref) -> usize {
    let mut depth = 0;
    while let Some(inst) = new_dom.get_by_ref(referent) {
        referent = inst.parent();
        depth += 1;
    }
    depth
}

/// Address a new-DOM instance as an apply-time parent: through the identity
/// mapping when it matched, otherwise it must be part of an added subtree.
fn anchor_for(new_ref: Ref, reverse_matched: &HashMap<Ref, Ref>) -> Anchor {
    reverse_matched
        .get(&new_ref)
        .copied()
        .map(Anchor::Old)
        .unwrap_or(Anchor::Added(new_ref))
}

fn emit_ops(ctx: &BuildCtx, old_ref: Ref, new_ref: Ref, ops: &mut Vec<EditOp>) {
    let old_parent = ctx.old_dom.get_by_ref(old_ref).unwrap();
    let new_parent = ctx.new_dom.get_by_ref(new_ref).unwrap();

    for old_child in old_parent.children() {
        let local_match = ctx
            .identity
            .matched
            .get(&old_child)
            .copied()
            .filter(|new_child| {
                ctx.new_dom
                    .get_by_ref(*new_child)
                    .is_some_and(|instance| instance.parent() == new_ref)
            });
        if local_match.is_some() || ctx.identity.moved_old.contains(&old_child) {
            continue;
        }
        if let Some(inst) = ctx.old_dom.get_by_ref(old_child) {
            if is_studio_artifact(ctx.old_dom, old_ref, inst) {
                continue;
            }
        }
        ops.push(EditOp::RemoveSubtree { old_ref: old_child });
    }

    for new_child in new_parent.children() {
        let local_match = ctx
            .identity
            .reverse_matched
            .get(&new_child)
            .copied()
            .filter(|old_child| {
                ctx.old_dom
                    .get_by_ref(*old_child)
                    .is_some_and(|instance| instance.parent() == old_ref)
            });
        if local_match.is_some() || ctx.identity.moved_new.contains(&new_child) {
            continue;
        }
        if let Some(inst) = ctx.new_dom.get_by_ref(new_child) {
            if is_studio_artifact(ctx.new_dom, new_ref, inst) {
                continue;
            }
        }
        ops.push(EditOp::AddSubtree {
            parent: Anchor::Old(old_ref),
            new_ref: new_child,
        });
    }

    for old_child in old_parent.children() {
        let Some(new_child) = ctx
            .identity
            .matched
            .get(&old_child)
            .copied()
            .filter(|new_child| {
                ctx.new_dom
                    .get_by_ref(*new_child)
                    .is_some_and(|instance| instance.parent() == new_ref)
            })
        else {
            continue;
        };
        // Pruning is safe here: mapping is already complete, and identical
        // subtrees need no ops.
        if ctx.old_deep.get(old_child) == ctx.new_deep.get(new_child) {
            continue;
        }
        emit_instance_edits(ctx, old_child, new_child, ops);
        emit_ops(ctx, old_child, new_child, ops);
    }
}

fn emit_instance_edits(ctx: &BuildCtx, old_ref: Ref, new_ref: Ref, ops: &mut Vec<EditOp>) {
    let old_inst = ctx.old_dom.get_by_ref(old_ref).unwrap();
    let new_inst = ctx.new_dom.get_by_ref(new_ref).unwrap();

    if old_inst.name() != new_inst.name() {
        ops.push(EditOp::SetName {
            old_ref,
            name: new_inst.name().to_string(),
        });
    }

    for change in raw_property_changes(
        ctx.old_dom,
        ctx.new_dom,
        old_ref,
        new_ref,
        ctx.config,
        &ctx.identity.matched,
    ) {
        ops.push(EditOp::SetProperty {
            old_ref,
            name: change.name,
            old_value: change.old,
            value: change.new,
        });
    }
}

// ============================================================================
// Apply
// ============================================================================

/// Apply an edit script to the DOM it was computed against, transforming it
/// into (a DOM diff-equal to) the new DOM. `new_dom` supplies subtree payloads
/// for AddSubtree ops.
pub fn apply_edit_script(target: &mut WeakDom, new_dom: &WeakDom, script: &EditScript) {
    apply_ops(target, new_dom, &script.ops, &script.identity);
    apply_pivot_ops(target, &script.pivots);
}

/// Apply a subset of ops (all computed against `target`'s original state, with
/// payloads/anchors in `source_dom` terms). Used directly by the merge
/// combiner to apply each branch's surviving ops from its own source DOM.
pub(crate) fn apply_ops(
    target: &mut WeakDom,
    source_dom: &dyn DomView,
    ops: &[EditOp],
    identity: &InstanceIdentity,
) -> HashMap<Ref, Ref> {
    apply_ops_where(target, source_dom, ops, identity, |_| true)
}

/// Apply only operations whose dense exclusion slot is false.
pub(crate) fn apply_ops_filtered(
    target: &mut WeakDom,
    source_dom: &dyn DomView,
    ops: &[EditOp],
    identity: &InstanceIdentity,
    excluded: &[bool],
) -> HashMap<Ref, Ref> {
    assert_eq!(ops.len(), excluded.len());
    apply_ops_where(target, source_dom, ops, identity, |index| !excluded[index])
}

fn apply_ops_where(
    target: &mut WeakDom,
    source_dom: &dyn DomView,
    ops: &[EditOp],
    identity: &InstanceIdentity,
    include: impl Fn(usize) -> bool + Copy,
) -> HashMap<Ref, Ref> {
    let new_dom = source_dom;
    // new_ref → target ref, for every instance apply creates
    let mut created: HashMap<Ref, Ref> = HashMap::new();
    // 1. Adds — clone subtrees out of the new DOM
    for (index, op) in ops.iter().enumerate() {
        if !include(index) {
            continue;
        }
        if let EditOp::AddSubtree { parent, new_ref } = op {
            let parent_ref = resolve_anchor(*parent, &created);
            let builder = build_subtree(new_dom, *new_ref, &identity.moved_new);
            let created_root = target.insert(parent_ref, builder);
            record_created(
                new_dom,
                *new_ref,
                target,
                created_root,
                &identity.moved_new,
                &mut created,
            );
        }
    }

    // 2. Moves — emitted in new-side depth order by the builder
    for (index, op) in ops.iter().enumerate() {
        if !include(index) {
            continue;
        }
        if let EditOp::Move {
            old_ref,
            new_parent,
        } = op
        {
            target.transfer_within(*old_ref, resolve_anchor(*new_parent, &created));
        }
    }

    // 3. Removes
    for (index, op) in ops.iter().enumerate() {
        if !include(index) {
            continue;
        }
        if let EditOp::RemoveSubtree { old_ref } = op {
            target.destroy(*old_ref);
        }
    }

    // 4. Names and properties
    for (index, op) in ops.iter().enumerate() {
        if !include(index) {
            continue;
        }
        match op {
            EditOp::SetName { old_ref, name } => {
                if let Some(inst) = target.get_by_ref_mut(*old_ref) {
                    inst.name = name.clone();
                }
            }
            EditOp::SetProperty {
                old_ref,
                name,
                old_value: _,
                value,
            } => {
                if let Some(inst) = target.get_by_ref_mut(*old_ref) {
                    // Granular container changes (Attributes.<key> / Tags.<tag>)
                    if set_sub_property(inst, name, value.as_ref()) {
                        continue;
                    }
                    match value {
                        None => {
                            inst.properties.remove(&name.as_str().into());
                        }
                        Some(v) => {
                            let v =
                                remap_ref_value(v.clone(), &identity.reverse_matched, &created);
                            inst.properties.insert(name.as_str().into(), v);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // 5. Cloned subtrees copied their properties verbatim — their Ref values
    // still point into the new DOM. Remap them into the target.
    let created_targets: Vec<Ref> = created.values().copied().collect();
    for target_ref in created_targets {
        let Some(inst) = target.get_by_ref(target_ref) else {
            continue;
        };
        let ref_props: Vec<(String, Variant)> = inst
            .properties
            .iter()
            .filter_map(|(name, value)| {
                direct_reference(value)
                    .filter(|(_, target)| !target.is_none())
                    .map(|_| (name.to_string(), value.clone()))
            })
            .collect();
        for (name, value) in ref_props {
            let remapped = remap_ref_value(value, &identity.reverse_matched, &created);
            if let Some(inst) = target.get_by_ref_mut(target_ref) {
                inst.properties.insert(name.as_str().into(), remapped);
            }
        }
    }

    created
}

fn resolve_anchor(anchor: Anchor, created: &HashMap<Ref, Ref>) -> Ref {
    match anchor {
        Anchor::Old(r) => r,
        Anchor::Added(new_ref) => created.get(&new_ref).copied().unwrap_or_else(Ref::none),
    }
}

fn remap_ref(new_ref: Ref, reverse: &HashMap<Ref, Ref>, created: &HashMap<Ref, Ref>) -> Ref {
    reverse
        .get(&new_ref)
        .or_else(|| created.get(&new_ref))
        .copied()
        .unwrap_or_else(Ref::none)
}

fn remap_ref_value(
    value: Variant,
    reverse: &HashMap<Ref, Ref>,
    created: &HashMap<Ref, Ref>,
) -> Variant {
    if let Some((_, target)) = direct_reference(&value) {
        if !target.is_none() {
            return with_direct_reference_target(value, remap_ref(target, reverse, created));
        }
    }
    value
}

/// Recursively clone a new-DOM subtree into an InstanceBuilder (full fidelity —
/// every property verbatim, no comparability filtering; this is a copy).
/// Move destinations are skipped: their content is an existing instance the
/// Move op relocates here, not new content to duplicate.
fn build_subtree(
    new_dom: &dyn DomView,
    referent: Ref,
    moved_destinations: &HashSet<Ref>,
) -> InstanceBuilder {
    let inst = new_dom.get_by_ref(referent).unwrap();
    let mut builder = InstanceBuilder::new(inst.class()).with_name(inst.name());
    for (name, value) in inst.properties() {
        builder = builder.with_property(name, value.clone());
    }
    let children: Vec<InstanceBuilder> = inst
        .children()
        .filter(|child| !moved_destinations.contains(child))
        .map(|child| build_subtree(new_dom, child, moved_destinations))
        .collect();
    builder.with_children(children)
}

/// Walk the source subtree and the freshly created target subtree in parallel,
/// recording new_ref → created_ref for every instance. Children were built in
/// source order minus skipped move destinations, so pairing filters the same
/// refs to stay positional.
fn record_created(
    new_dom: &dyn DomView,
    new_ref: Ref,
    target: &WeakDom,
    created_ref: Ref,
    moved_destinations: &HashSet<Ref>,
    created: &mut HashMap<Ref, Ref>,
) {
    created.insert(new_ref, created_ref);
    let source_children = new_dom
        .get_by_ref(new_ref)
        .unwrap()
        .children()
        .filter(|c| !moved_destinations.contains(c));
    let target_children = target
        .get_by_ref(created_ref)
        .unwrap()
        .children()
        .iter()
        .copied();
    for (source_child, target_child) in source_children.zip(target_children) {
        record_created(
            new_dom,
            source_child,
            target,
            target_child,
            moved_destinations,
            created,
        );
    }
}

// ============================================================================
// Granular container properties (Attributes.<key> / Tags.<tag>)
// ============================================================================

/// Write a namespaced sub-property (read-modify-write on the container).
/// Returns false when `name` isn't namespaced, leaving it to the caller.
/// Removing the last key removes the container property itself, matching the
/// differ's "empty container == absent" semantics.
pub(crate) fn set_sub_property(
    inst: &mut rbx_dom_weak::Instance,
    name: &str,
    value: Option<&Variant>,
) -> bool {
    if let Some(key) = name.strip_prefix("Attributes.") {
        let mut attrs = match inst.properties.get(&"Attributes".into()) {
            Some(Variant::Attributes(a)) => a.clone(),
            _ => rbx_types::Attributes::new(),
        };
        match value {
            Some(v) => {
                attrs.insert(key.to_string(), v.clone());
            }
            None => {
                attrs.remove(key);
            }
        }
        if attrs.is_empty() {
            inst.properties.remove(&"Attributes".into());
        } else {
            inst.properties
                .insert("Attributes".into(), Variant::Attributes(attrs));
        }
        return true;
    }

    if let Some(tag) = name.strip_prefix("Tags.") {
        let existing: Vec<String> = match inst.properties.get(&"Tags".into()) {
            Some(Variant::Tags(t)) => t.iter().map(|t| t.to_string()).collect(),
            _ => Vec::new(),
        };
        let mut tags: Vec<String> = existing.into_iter().filter(|t| t != tag).collect();
        if value.is_some() {
            tags.push(tag.to_string());
        }
        if tags.is_empty() {
            inst.properties.remove(&"Tags".into());
        } else {
            let mut t = rbx_types::Tags::new();
            for tag in &tags {
                t.push(tag);
            }
            inst.properties.insert("Tags".into(), Variant::Tags(t));
        }
        return true;
    }

    false
}

/// Read a namespaced sub-property from an instance. Returns None both for
/// "not namespaced" (callers check with is_sub_property first when it
/// matters) and "key absent".
pub(crate) fn get_sub_property(inst: &rbx_dom_weak::Instance, name: &str) -> Option<Variant> {
    if let Some(key) = name.strip_prefix("Attributes.") {
        return match inst.properties.get(&"Attributes".into()) {
            Some(Variant::Attributes(a)) => a.get(key).cloned(),
            _ => None,
        };
    }
    if let Some(tag) = name.strip_prefix("Tags.") {
        return match inst.properties.get(&"Tags".into()) {
            Some(Variant::Tags(t)) if t.iter().any(|t| t == tag) => {
                Some(Variant::String(tag.to_string()))
            }
            _ => None,
        };
    }
    None
}

/// Whether a property name addresses a granular container key.
pub(crate) fn is_sub_property(name: &str) -> bool {
    name.starts_with("Attributes.") || name.starts_with("Tags.")
}
