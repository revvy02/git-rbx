//! Shared semantic equality for Roblox property values.
//!
//! Hashes stay exact so they remain conservative pruning accelerators. This
//! module owns the small tolerances used by diffing, matching, and merge
//! deduplication so those layers cannot silently disagree about equality.

use rbx_types::Variant;

const F32_ABS_TOLERANCE: f32 = 1.0e-7;
const F64_ABS_TOLERANCE: f64 = 1.0e-12;
const MAX_FLOAT_ULPS: u32 = 2;
// Studio re-saves CFrames after normalizing their rotation matrices. The
// resulting component drift can be several dozen ULPs while representing the
// same authored placement. Sub-millistud/sub-hundredth-degree placement
// changes are outside rbx-diff's useful fidelity, so CFrames get a deliberately
// wider policy without weakening comparison for unrelated float properties.
const CFRAME_POSITION_ABS_TOLERANCE: f32 = 1.0e-4;
const CFRAME_ROTATION_ABS_TOLERANCE: f32 = 1.0e-4;

fn ordered_f32_bits(value: f32) -> u32 {
    let bits = value.to_bits();
    if bits & 0x8000_0000 != 0 {
        !bits
    } else {
        bits | 0x8000_0000
    }
}

fn ordered_f64_bits(value: f64) -> u64 {
    let bits = value.to_bits();
    if bits & 0x8000_0000_0000_0000 != 0 {
        !bits
    } else {
        bits | 0x8000_0000_0000_0000
    }
}

fn f32_equal_with_tolerance(a: f32, b: f32, absolute_tolerance: f32) -> bool {
    if a == b || (a.is_nan() && b.is_nan()) {
        return true;
    }
    if !a.is_finite() || !b.is_finite() {
        return false;
    }
    (a - b).abs() <= absolute_tolerance
        || ordered_f32_bits(a).abs_diff(ordered_f32_bits(b)) <= MAX_FLOAT_ULPS
}

fn f32_equal(a: f32, b: f32) -> bool {
    f32_equal_with_tolerance(a, b, F32_ABS_TOLERANCE)
}

fn f64_equal(a: f64, b: f64) -> bool {
    if a == b || (a.is_nan() && b.is_nan()) {
        return true;
    }
    if !a.is_finite() || !b.is_finite() {
        return false;
    }
    (a - b).abs() <= F64_ABS_TOLERANCE
        || ordered_f64_bits(a).abs_diff(ordered_f64_bits(b)) <= u64::from(MAX_FLOAT_ULPS)
}

fn vector2_equal(a: rbx_types::Vector2, b: rbx_types::Vector2) -> bool {
    f32_equal(a.x, b.x) && f32_equal(a.y, b.y)
}

fn vector3_equal(a: rbx_types::Vector3, b: rbx_types::Vector3) -> bool {
    f32_equal(a.x, b.x) && f32_equal(a.y, b.y) && f32_equal(a.z, b.z)
}

fn vector3_equal_with_tolerance(
    a: rbx_types::Vector3,
    b: rbx_types::Vector3,
    absolute_tolerance: f32,
) -> bool {
    f32_equal_with_tolerance(a.x, b.x, absolute_tolerance)
        && f32_equal_with_tolerance(a.y, b.y, absolute_tolerance)
        && f32_equal_with_tolerance(a.z, b.z, absolute_tolerance)
}

fn cframe_equal(a: rbx_types::CFrame, b: rbx_types::CFrame) -> bool {
    vector3_equal_with_tolerance(a.position, b.position, CFRAME_POSITION_ABS_TOLERANCE)
        && vector3_equal_with_tolerance(
            a.orientation.x,
            b.orientation.x,
            CFRAME_ROTATION_ABS_TOLERANCE,
        )
        && vector3_equal_with_tolerance(
            a.orientation.y,
            b.orientation.y,
            CFRAME_ROTATION_ABS_TOLERANCE,
        )
        && vector3_equal_with_tolerance(
            a.orientation.z,
            b.orientation.z,
            CFRAME_ROTATION_ABS_TOLERANCE,
        )
}

/// Semantic equality for variants without cross-DOM Ref identity.
pub(crate) fn non_ref_variants_equal(a: &Variant, b: &Variant) -> bool {
    use std::mem::discriminant;

    if discriminant(a) != discriminant(b) {
        return false;
    }

    match (a, b) {
        (Variant::Float32(x), Variant::Float32(y)) => f32_equal(*x, *y),
        (Variant::Float64(x), Variant::Float64(y)) => f64_equal(*x, *y),
        (Variant::Vector2(x), Variant::Vector2(y)) => vector2_equal(*x, *y),
        (Variant::Vector3(x), Variant::Vector3(y)) => vector3_equal(*x, *y),
        (Variant::CFrame(x), Variant::CFrame(y)) => cframe_equal(*x, *y),
        (Variant::OptionalCFrame(x), Variant::OptionalCFrame(y)) => match (x, y) {
            (Some(x), Some(y)) => cframe_equal(*x, *y),
            (None, None) => true,
            _ => false,
        },
        (Variant::Color3(x), Variant::Color3(y)) => {
            f32_equal(x.r, y.r) && f32_equal(x.g, y.g) && f32_equal(x.b, y.b)
        }
        (Variant::ColorSequence(x), Variant::ColorSequence(y)) => {
            x.keypoints.len() == y.keypoints.len()
                && x.keypoints.iter().zip(&y.keypoints).all(|(x, y)| {
                    f32_equal(x.time, y.time)
                        && f32_equal(x.color.r, y.color.r)
                        && f32_equal(x.color.g, y.color.g)
                        && f32_equal(x.color.b, y.color.b)
                })
        }
        (Variant::NumberRange(x), Variant::NumberRange(y)) => {
            f32_equal(x.min, y.min) && f32_equal(x.max, y.max)
        }
        (Variant::NumberSequence(x), Variant::NumberSequence(y)) => {
            x.keypoints.len() == y.keypoints.len()
                && x.keypoints.iter().zip(&y.keypoints).all(|(x, y)| {
                    f32_equal(x.time, y.time)
                        && f32_equal(x.value, y.value)
                        && f32_equal(x.envelope, y.envelope)
                })
        }
        (Variant::PhysicalProperties(x), Variant::PhysicalProperties(y)) => match (x, y) {
            (rbx_types::PhysicalProperties::Default, rbx_types::PhysicalProperties::Default) => {
                true
            }
            (
                rbx_types::PhysicalProperties::Custom(x),
                rbx_types::PhysicalProperties::Custom(y),
            ) => {
                f32_equal(x.density(), y.density())
                    && f32_equal(x.friction(), y.friction())
                    && f32_equal(x.elasticity(), y.elasticity())
                    && f32_equal(x.friction_weight(), y.friction_weight())
                    && f32_equal(x.elasticity_weight(), y.elasticity_weight())
            }
            _ => false,
        },
        (Variant::Ray(x), Variant::Ray(y)) => {
            vector3_equal(x.origin, y.origin) && vector3_equal(x.direction, y.direction)
        }
        (Variant::Rect(x), Variant::Rect(y)) => {
            vector2_equal(x.min, y.min) && vector2_equal(x.max, y.max)
        }
        (Variant::Region3(x), Variant::Region3(y)) => {
            vector3_equal(x.min, y.min) && vector3_equal(x.max, y.max)
        }
        (Variant::UDim(x), Variant::UDim(y)) => f32_equal(x.scale, y.scale) && x.offset == y.offset,
        (Variant::UDim2(x), Variant::UDim2(y)) => {
            f32_equal(x.x.scale, y.x.scale)
                && x.x.offset == y.x.offset
                && f32_equal(x.y.scale, y.y.scale)
                && x.y.offset == y.y.offset
        }
        // Studio rewrites equivalent asset URL spellings on save.
        (Variant::Content(a), Variant::Content(b)) => {
            use crate::property_semantics::normalize_asset_uri;
            use rbx_types::ContentType;
            match (a.value(), b.value()) {
                (ContentType::None, ContentType::None) => true,
                (ContentType::Uri(ua), ContentType::Uri(ub)) => {
                    normalize_asset_uri(ua) == normalize_asset_uri(ub)
                }
                // Object refs into the DOM are covered by instance identity.
                (ContentType::Object(_), ContentType::Object(_)) => true,
                _ => false,
            }
        }
        (Variant::ContentId(a), Variant::ContentId(b)) => {
            crate::property_semantics::normalize_asset_uri(a.as_str())
                == crate::property_semantics::normalize_asset_uri(b.as_str())
        }
        (Variant::UniqueId(_), Variant::UniqueId(_)) => true,
        _ => a == b,
    }
}
