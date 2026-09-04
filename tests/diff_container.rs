//! The Studio diff container: a temporary copy of the new file carrying the
//! diff document, manifests, live-instance bindings, and ghost snapshots.
use git_rbx::{
    diff_model_compact_doms_document, stamp_diff, DiffConfig, DiffDom, DIFF_CONTAINER_NAME,
};
use rbx_dom_weak::{types::Ref, InstanceBuilder, WeakDom};
use rbx_types::Variant;

fn part(name: &str, transparency: f32) -> InstanceBuilder {
    InstanceBuilder::new("Part")
        .with_name(name)
        .with_property("Transparency", Variant::Float32(transparency))
}

fn old_dom() -> WeakDom {
    WeakDom::new(
        InstanceBuilder::new("Folder")
            .with_name("root")
            .with_child(
                InstanceBuilder::new("Folder")
                    .with_name("A")
                    .with_child(part("P", 0.0))
                    .with_child(
                        InstanceBuilder::new("Model")
                            .with_name("Gone")
                            .with_child(part("G1", 0.0))
                            .with_child(part("G2", 0.0)),
                    ),
            ),
    )
}

fn new_dom() -> WeakDom {
    WeakDom::new(
        InstanceBuilder::new("Folder")
            .with_name("root")
            .with_child(InstanceBuilder::new("Folder").with_name("A").with_child(part("P", 0.5)))
            .with_child(InstanceBuilder::new("Folder").with_name("Fresh").with_child(part("F", 0.0))),
    )
}

fn compact(dom: &WeakDom) -> DiffDom {
    // Through bytes, the way the CLI loads files.
    let mut bytes = Vec::new();
    rbx_binary::to_writer(&mut bytes, dom, dom.root().children()).unwrap();
    DiffDom::from_binary_reader(bytes.as_slice()).unwrap()
}

fn child(dom: &WeakDom, parent: Ref, name: &str) -> Option<Ref> {
    dom.get_by_ref(parent)?
        .children()
        .iter()
        .copied()
        .find(|&c| dom.get_by_ref(c).map(|i| i.name == name).unwrap_or(false))
}

#[test]
fn container_carries_document_bindings_and_ghosts() {
    let old = old_dom();
    let old_compact = compact(&old);
    let mut new_compact = compact(&new_dom());
    let document =
        diff_model_compact_doms_document(&old_compact, &mut new_compact, &DiffConfig::default());
    assert_eq!(document.counts.added, 1);
    assert_eq!(document.counts.removed, 1);
    assert_eq!(document.counts.modified, 1);

    // The WeakDom the viewer opens is a separate load of the same bytes.
    let mut bytes = Vec::new();
    let new = new_dom();
    rbx_binary::to_writer(&mut bytes, &new, new.root().children()).unwrap();
    let mut stamped: WeakDom = rbx_binary::from_reader(bytes.as_slice()).unwrap();
    stamp_diff(&mut stamped, &old, &document, "old.rbxm", "new.rbxm").unwrap();

    let container = child(&stamped, stamped.root_ref(), DIFF_CONTAINER_NAME).expect("container");
    let trees = child(&stamped, container, "VirtualTrees").unwrap();
    assert!(child(&stamped, trees, "Old").is_some());
    assert!(child(&stamped, trees, "New").is_some());
    let subjects = child(&stamped, trees, "Subjects").unwrap();
    assert_eq!(
        stamped.get_by_ref(subjects).unwrap().children().len(),
        document.new.len(),
        "every new-manifest id binds to a live instance"
    );
    // A binding points at the instance the manifest describes.
    let fresh_id = document.new.iter().find(|n| n.name == "Fresh").unwrap().id;
    let binding = child(&stamped, subjects, &format!("N{fresh_id}")).unwrap();
    let Some(Variant::Ref(target)) = stamped.get_by_ref(binding).unwrap().properties.get(&"Value".into())
    else {
        panic!("binding is an ObjectValue");
    };
    assert_eq!(stamped.get_by_ref(*target).unwrap().name, "Fresh");

    assert!(child(&stamped, container, "Document").is_some());

    let removed = child(&stamped, container, "Removed").unwrap();
    let gone_id = document.old.iter().find(|n| n.name == "Gone").unwrap().id;
    let ghost = child(&stamped, removed, &format!("R{gone_id}")).expect("removed subtree snapshot");
    let snapshot = child(&stamped, ghost, "Gone").expect("cloned root");
    assert_eq!(stamped.get_by_ref(snapshot).unwrap().children().len(), 2, "clone is deep");
    let Some(Variant::Attributes(attrs)) = stamped.get_by_ref(ghost).unwrap().properties.get(&"Attributes".into())
    else {
        panic!("ghost attrs");
    };
    // "root" is the DOM root, not a manifest node; the ghost's old parent is A.
    let a_id = document.old.iter().find(|n| n.name == "A").unwrap().id;
    assert_eq!(attrs.get("OldParent"), Some(&Variant::Float64(a_id as f64)));
}

#[test]
fn stamping_refuses_a_file_that_does_not_match_the_manifest() {
    let old = old_dom();
    let old_compact = compact(&old);
    let mut new_compact = compact(&new_dom());
    let document =
        diff_model_compact_doms_document(&old_compact, &mut new_compact, &DiffConfig::default());
    // A different file than the one the document was computed from.
    let mut other = old_dom();
    let err = stamp_diff(&mut other, &old, &document, "old.rbxm", "new.rbxm").unwrap_err();
    assert!(err.to_string().contains("manifest"), "{err}");
}
