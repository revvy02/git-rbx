//! Reference-bearing Roblox value semantics.
//!
//! Identity matching, hashing, equality, and materialization all use this
//! module so adding a new representation (such as InstanceHandle attributes)
//! does not require another set of type-specific branches.

use rbx_dom_weak::types::Ref;
use rbx_types::{ContentType, Variant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ReferenceKind {
    Ref,
    ContentObject,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ReferenceLocation {
    Property(String, ReferenceKind),
    /// Reserved for InstanceHandle-valued attributes. A nested `Variant::Ref`
    /// is accepted today so topology indexing does not depend on the eventual
    /// upstream rbx-types representation.
    Attribute(String, ReferenceKind),
}

#[derive(Debug, Clone)]
pub(crate) struct ReferenceEdge {
    pub(crate) location: ReferenceLocation,
    pub(crate) target: Option<Ref>,
}

pub(crate) fn direct_reference(value: &Variant) -> Option<(ReferenceKind, Ref)> {
    match value {
        Variant::Ref(target) => Some((ReferenceKind::Ref, *target)),
        Variant::Content(content) => match content.value() {
            ContentType::Object(target) => Some((ReferenceKind::ContentObject, *target)),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn reference_count(value: &Variant) -> usize {
    match value {
        Variant::Attributes(attributes) => attributes
            .into_iter()
            .filter_map(|(_, value)| direct_reference(value))
            .filter(|(_, target)| !target.is_none())
            .count(),
        _ => direct_reference(value)
            .filter(|(_, target)| !target.is_none())
            .is_some() as usize,
    }
}

pub(crate) fn with_direct_reference_target(value: Variant, target: Ref) -> Variant {
    match value {
        Variant::Ref(_) => Variant::Ref(target),
        Variant::Content(content) if matches!(content.value(), ContentType::Object(_)) => {
            Variant::Content(rbx_types::Content::from_referent(target))
        }
        other => other,
    }
}

/// Visit every reference-bearing authored property without allocating a
/// temporary edge list for reference-free instances.
pub(crate) fn visit_reference_edges<'a>(
    properties: impl Iterator<Item = (&'a str, &'a Variant)>,
    mut visit: impl FnMut(ReferenceEdge),
) {
    for (name, value) in properties {
        match value {
            Variant::Attributes(attributes) => {
                for (attribute_name, attribute_value) in attributes {
                    if let Some((kind, target)) = direct_reference(attribute_value) {
                        visit(ReferenceEdge {
                            location: ReferenceLocation::Attribute(attribute_name.clone(), kind),
                            target: (!target.is_none()).then_some(target),
                        });
                    }
                }
            }
            _ => {
                if let Some((kind, target)) = direct_reference(value) {
                    visit(ReferenceEdge {
                        location: ReferenceLocation::Property(name.to_owned(), kind),
                        target: (!target.is_none()).then_some(target),
                    });
                }
            }
        }
    }
}
