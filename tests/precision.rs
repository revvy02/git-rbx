//! Precision regressions: pruning hashes are exact, while leaf comparisons
//! tolerate only representation-scale floating-point noise.

use rbx_diff::{diff_doms, merge_doms, DiffConfig, DiffEntry};
use rbx_dom_weak::{InstanceBuilder, WeakDom};
use rbx_types::{CFrame, Matrix3, Variant, Vector3};

fn folder(name: &str) -> InstanceBuilder {
    InstanceBuilder::new("Folder").with_name(name)
}

fn part_with(property: &str, value: Variant) -> InstanceBuilder {
    InstanceBuilder::new("Part")
        .with_name("P")
        .with_property("Anchored", Variant::Bool(true))
        .with_property(property, value)
}

fn dom_with(property: &str, value: Variant) -> WeakDom {
    WeakDom::new(folder("root").with_child(part_with(property, value)))
}

fn cframe_at(x: f32) -> CFrame {
    CFrame::new(
        Vector3::new(x, 0.0, 0.0),
        Matrix3::new(
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        ),
    )
}

fn assert_one_property_change(diffs: &[DiffEntry], property: &str) {
    assert_eq!(diffs.len(), 1, "{diffs:#?}");
    match &diffs[0] {
        DiffEntry::Modified { property_changes, .. } => {
            assert_eq!(property_changes.len(), 1, "{property_changes:#?}");
            assert_eq!(property_changes[0].name, property);
        }
        other => panic!("expected Modified, got {other:#?}"),
    }
}

#[test]
fn decimal_rounding_can_no_longer_prune_a_real_change() {
    // The old hash rounded both values to 0.0, despite the property comparator
    // considering them different. That made the entire subtree disappear.
    let old = dom_with("Transparency", Variant::Float32(0.01));
    let new = dom_with("Transparency", Variant::Float32(0.04));

    assert_one_property_change(&diff_doms(&old, &new), "Transparency");
}

#[test]
fn one_float32_ulp_is_treated_as_representation_noise() {
    let value = 0.5_f32;
    let adjacent = f32::from_bits(value.to_bits() + 1);
    let old = dom_with("Transparency", Variant::Float32(value));
    let new = dom_with("Transparency", Variant::Float32(adjacent));

    assert!(diff_doms(&old, &new).is_empty());
}

#[test]
fn three_float32_ulps_are_not_hidden() {
    let value = 0.5_f32;
    let three_ulps_away = f32::from_bits(value.to_bits() + 3);
    let old = dom_with("Transparency", Variant::Float32(value));
    let new = dom_with("Transparency", Variant::Float32(three_ulps_away));

    assert_one_property_change(&diff_doms(&old, &new), "Transparency");
}

#[test]
fn cframe_noise_and_visible_motion_are_distinguished() {
    let value = 0.5_f32;
    let adjacent = f32::from_bits(value.to_bits() + 1);
    let base = dom_with("CFrame", Variant::CFrame(cframe_at(value)));
    let noisy = dom_with("CFrame", Variant::CFrame(cframe_at(adjacent)));
    let moved = dom_with("CFrame", Variant::CFrame(cframe_at(value + 0.00001)));

    assert!(diff_doms(&base, &noisy).is_empty());
    assert_one_property_change(&diff_doms(&base, &moved), "CFrame");
}

#[test]
fn merge_deduplication_uses_the_same_float_policy() {
    let value = 0.5_f32;
    let adjacent = f32::from_bits(value.to_bits() + 1);
    let mut base = dom_with("Transparency", Variant::Float32(0.0));
    let ours = dom_with("Transparency", Variant::Float32(value));
    let theirs = dom_with("Transparency", Variant::Float32(adjacent));

    let result = merge_doms(&mut base, &ours, &theirs, &DiffConfig::default());

    assert!(result.conflicts.is_empty(), "{:#?}", result.conflicts);
    assert_eq!(result.stats.deduped, 1);
    assert!(diff_doms(&base, &ours).is_empty());
}
