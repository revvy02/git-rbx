//! Primitive hierarchical placement operations and their ordered application.
//!
//! A nested operation stores only the motion relative to its nearest
//! participating ancestor. This makes independent parent and child choices
//! composable: materialization reconstructs each effective transform from the
//! selected local operation and the already-resolved ancestor transform.

use rbx_dom_weak::{types::Ref, WeakDom};
use rbx_types::{CFrame, Variant};
use std::collections::HashMap;

use crate::diff_dom::{DiffDom, DomViewMut};
use crate::dom_utils::class_is_a;
use crate::rigid_groups::Rigid;

/// One inferred rigid placement of a hierarchy boundary.
///
/// Ordinary edits are applied in canonical coordinates; pivots are then
/// materialized in parent-before-child order.
#[derive(Debug, Clone)]
pub struct PivotOp {
    /// Boundary in the old/base DOM.
    pub target_ref: Ref,
    /// Corresponding boundary in the new/branch DOM.
    pub side_ref: Ref,
    /// Stable preorder among all hierarchy boundaries.
    pub order: usize,
    /// Nearest ancestor placement participating in this plan.
    pub parent_order: Option<usize>,
    /// Parent-relative rigid transform, or world-relative for a root.
    pub delta: CFrame,
}

/// A pivot whose target has been resolved into the DOM being materialized.
#[derive(Debug, Clone)]
pub struct PivotApplication {
    pub target_ref: Ref,
    pub path: String,
    pub order: usize,
    pub parent_order: Option<usize>,
    pub delta: CFrame,
}

pub(crate) fn transform_world_refs(dom: &mut dyn DomViewMut, refs: Vec<(Ref, Rigid)>) {
    for (referent, alignment) in refs {
        // Identity f64 round-trips can still perturb serialized f32 values.
        if Rigid::close(&alignment, &Rigid::identity()) {
            continue;
        }
        let replacement = {
            let Some(instance) = dom.get_by_ref(referent) else {
                continue;
            };
            if class_is_a(instance.class(), "BasePart") {
                match instance.property("CFrame") {
                    Some(Variant::CFrame(cframe)) => Some((
                        "CFrame",
                        Variant::CFrame(alignment.mul(Rigid::from_cframe(cframe)).to_cframe()),
                    )),
                    _ => None,
                }
            } else if class_is_a(instance.class(), "Model")
                && !class_is_a(instance.class(), "WorldRoot")
            {
                match instance.property("WorldPivotData") {
                    Some(Variant::OptionalCFrame(Some(cframe))) => Some((
                        "WorldPivotData",
                        Variant::OptionalCFrame(Some(
                            alignment.mul(Rigid::from_cframe(cframe)).to_cframe(),
                        )),
                    )),
                    _ => None,
                }
            } else {
                None
            }
        };
        if let Some((name, value)) = replacement {
            let updated = dom.set_existing_property(referent, name, value);
            debug_assert!(updated);
        }
    }
}

pub(crate) fn transform_world_properties_below(
    dom: &mut dyn DomViewMut,
    root: Ref,
    alignment: Rigid,
) {
    let mut pending = vec![root];
    let mut refs = Vec::new();
    while let Some(referent) = pending.pop() {
        let Some(instance) = dom.get_by_ref(referent) else {
            continue;
        };
        pending.extend(instance.children());
        refs.push((referent, alignment));
    }
    transform_world_refs(dom, refs);
}

pub fn apply_pivot_ops(dom: &mut WeakDom, pivots: &[PivotOp]) {
    let applications: Vec<_> = pivots
        .iter()
        .map(|pivot| PivotApplication {
            target_ref: pivot.target_ref,
            path: String::new(),
            order: pivot.order,
            parent_order: pivot.parent_order,
            delta: pivot.delta,
        })
        .collect();
    apply_pivot_plan(dom, &applications);
}

pub fn apply_pivot_ops_to_compact_branch(
    dom: &mut DiffDom,
    pivots: &[PivotOp],
    matched: &HashMap<Ref, Ref>,
) {
    let applications: Vec<_> = pivots
        .iter()
        .filter_map(|pivot| {
            let target_ref = matched.get(&pivot.target_ref).copied()?;
            Some(PivotApplication {
                target_ref,
                path: String::new(),
                order: pivot.order,
                parent_order: pivot.parent_order,
                delta: pivot.delta,
            })
        })
        .collect();
    apply_pivot_plan_view(dom, &applications);
}

pub fn apply_pivot_plan(dom: &mut WeakDom, pivots: &[PivotApplication]) {
    apply_pivot_plan_view(dom, pivots);
}

pub(crate) fn apply_pivot_plan_view(dom: &mut dyn DomViewMut, pivots: &[PivotApplication]) {
    let mut ordered: Vec<_> = pivots.iter().collect();
    ordered.sort_unstable_by_key(|pivot| pivot.order);
    let mut effective_by_order: HashMap<usize, Rigid> = HashMap::new();
    let mut effective_by_target = HashMap::new();
    for pivot in ordered {
        let parent = pivot
            .parent_order
            .and_then(|order| effective_by_order.get(&order).copied())
            .unwrap_or_else(Rigid::identity);
        let effective = Rigid::from_cframe(&pivot.delta).mul(parent);
        effective_by_order.insert(pivot.order, effective);
        effective_by_target.insert(pivot.target_ref, effective);
    }

    let mut pending: Vec<(Ref, Option<Rigid>)> = dom
        .get_by_ref(dom.root_ref())
        .into_iter()
        .flat_map(|root| root.children())
        .map(|referent| (referent, None))
        .collect();
    let mut refs = Vec::new();
    while let Some((referent, inherited)) = pending.pop() {
        let active = effective_by_target.get(&referent).copied().or(inherited);
        let Some(instance) = dom.get_by_ref(referent) else {
            continue;
        };
        pending.extend(instance.children().map(|child| (child, active)));
        if let Some(effective) = active {
            refs.push((referent, effective));
        }
    }
    transform_world_refs(dom, refs);
}
