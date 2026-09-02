use git_rbx::diff_doms;
use rbx_dom_weak::{InstanceBuilder, WeakDom};
use rbx_types::{CFrame, Content, Matrix3, Variant, Vector3};

fn attachment(name: &str, x: f32) -> InstanceBuilder {
    InstanceBuilder::new("Attachment")
        .with_name(name)
        .with_property(
            "CFrame",
            Variant::CFrame(CFrame::new(Vector3::new(x, 0.0, 0.0), Matrix3::identity())),
        )
}

fn powerline(permuted: bool) -> WeakDom {
    let attachment_a = attachment("Attachment0", 1.0);
    let attachment_b = attachment("Attachment0", 2.0);
    let attachment_a_ref = attachment_a.referent();
    let attachment_b_ref = attachment_b.referent();

    let constraint_a = InstanceBuilder::new("RopeConstraint")
        .with_name("RopeConstraint")
        .with_property("Attachment0", Variant::Ref(attachment_a_ref))
        .with_property("Attachment1", Variant::Ref(attachment_b_ref))
        .with_property("Length", Variant::Float32(10.0));
    let constraint_b = InstanceBuilder::new("RopeConstraint")
        .with_name("RopeConstraint")
        .with_property("Attachment0", Variant::Ref(attachment_b_ref))
        .with_property("Attachment1", Variant::Ref(attachment_a_ref))
        .with_property("Length", Variant::Float32(20.0));

    // The two parents deliberately have identical shallow identity. Their
    // descendants and labeled reference edges are the only way to distinguish
    // them, matching the RC powerline structure that exposed the regression.
    let part_a = InstanceBuilder::new("Part")
        .with_name("Part")
        .with_property("Anchored", Variant::Bool(true))
        .with_child(attachment_a)
        .with_child(constraint_a);
    let part_b = InstanceBuilder::new("Part")
        .with_name("Part")
        .with_property("Anchored", Variant::Bool(true))
        .with_child(attachment_b)
        .with_child(constraint_b);

    let connections = if permuted {
        InstanceBuilder::new("Folder")
            .with_name("Connections")
            .with_child(part_b)
            .with_child(part_a)
    } else {
        InstanceBuilder::new("Folder")
            .with_name("Connections")
            .with_child(part_a)
            .with_child(part_b)
    };

    WeakDom::new(
        InstanceBuilder::new("DataModel")
            .with_name("root")
            .with_child(connections),
    )
}

fn reference_only_powerline(permuted: bool, content_objects: bool) -> WeakDom {
    let attachment_a = attachment("Attachment0", 0.0);
    let attachment_b = attachment("Attachment0", 0.0);
    let attachment_a_ref = attachment_a.referent();
    let attachment_b_ref = attachment_b.referent();

    // Every local property and containment shape is identical. Only the
    // labeled Ref graph distinguishes the two parent subtrees: A has a
    // self-loop, while B points one endpoint into A.
    let reference = |target| {
        if content_objects {
            Variant::Content(Content::from_referent(target))
        } else {
            Variant::Ref(target)
        }
    };
    let constraint_a = InstanceBuilder::new("RopeConstraint")
        .with_name("RopeConstraint")
        .with_property("Attachment0", reference(attachment_a_ref))
        .with_property("Attachment1", reference(attachment_a_ref))
        .with_property("Length", Variant::Float32(10.0));
    let constraint_b = InstanceBuilder::new("RopeConstraint")
        .with_name("RopeConstraint")
        .with_property("Attachment0", reference(attachment_b_ref))
        .with_property("Attachment1", reference(attachment_a_ref))
        .with_property("Length", Variant::Float32(10.0));

    let part_a = InstanceBuilder::new("Part")
        .with_name("Part")
        .with_property("Anchored", Variant::Bool(true))
        .with_child(attachment_a)
        .with_child(constraint_a);
    let part_b = InstanceBuilder::new("Part")
        .with_name("Part")
        .with_property("Anchored", Variant::Bool(true))
        .with_child(attachment_b)
        .with_child(constraint_b);

    let connections = if permuted {
        InstanceBuilder::new("Folder")
            .with_name("Connections")
            .with_child(part_b)
            .with_child(part_a)
    } else {
        InstanceBuilder::new("Folder")
            .with_name("Connections")
            .with_child(part_a)
            .with_child(part_b)
    };

    WeakDom::new(
        InstanceBuilder::new("DataModel")
            .with_name("root")
            .with_child(connections),
    )
}

fn reflective_sign(permuted: bool) -> WeakDom {
    fn reflective(image: &str) -> InstanceBuilder {
        InstanceBuilder::new("Folder")
            .with_name("Reflective")
            .with_child(
                InstanceBuilder::new("ImageLabel")
                    .with_name("ImageLabel")
                    .with_property("ImageContent", Variant::String(image.to_owned())),
            )
    }

    let first = reflective("rbxassetid://13613335188");
    let second = reflective("rbxassetid://124680656758729");
    let sign = if permuted {
        InstanceBuilder::new("Model")
            .with_name("Sign")
            .with_child(second)
            .with_child(first)
    } else {
        InstanceBuilder::new("Model")
            .with_name("Sign")
            .with_child(first)
            .with_child(second)
    };

    WeakDom::new(
        InstanceBuilder::new("DataModel")
            .with_name("root")
            .with_child(sign),
    )
}

#[test]
fn reordered_duplicate_parents_preserve_reference_graph_identity() {
    let old = powerline(false);
    let new = powerline(true);

    let diffs = diff_doms(&old, &new);
    assert!(
        diffs.is_empty(),
        "a sibling permutation with identical containment and Ref topology is not a change: \
         {diffs:#?}"
    );
}

#[test]
fn labeled_refs_disambiguate_otherwise_identical_parent_subtrees() {
    let old = reference_only_powerline(false, false);
    let new = reference_only_powerline(true, false);
    let diffs = diff_doms(&old, &new);

    assert!(
        diffs.is_empty(),
        "matching must preserve the Attachment0/Attachment1 graph: {diffs:#?}"
    );
}

#[test]
fn content_object_references_use_the_same_graph_identity() {
    let old = reference_only_powerline(false, true);
    let new = reference_only_powerline(true, true);
    let diffs = diff_doms(&old, &new);

    assert!(
        diffs.is_empty(),
        "Content::Object must participate in generalized reference matching: {diffs:#?}"
    );
}

#[test]
fn reordered_duplicate_subtrees_preserve_deep_content_identity() {
    let old = reflective_sign(false);
    let new = reflective_sign(true);
    let diffs = diff_doms(&old, &new);

    assert!(
        diffs.is_empty(),
        "an ordinary sibling permutation must follow exact subtree content: {diffs:#?}"
    );
}
