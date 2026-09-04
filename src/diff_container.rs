//! In-file diff state for the Studio diff viewer.
//!
//! `diff --studio` cannot modify either input, so it stamps the diff
//! document into a temporary copy of the NEW file and opens that. The
//! container mirrors the conflict container's conventions so the resolver's
//! boot path reads it with the same code: chunked manifests, an ObjectValue
//! table from manifest ids to live instances, and snapshots for content
//! that no longer exists live (here, removed subtrees, cloned from the old
//! version so the viewer can render them as ghosts in place).
//!
//! ```text
//! __GitRbxDiff                    Folder, attrs: Version, OldPath, NewPath, counts
//!   VirtualTrees/Old, New         chunked [id, parent, name, class] arrays
//!   VirtualTrees/Subjects/N<id>   ObjectValue -> live instance in this file
//!   Document/Chunk_N              chunked JSON: { ops, pivots }
//!   Removed/R<id>                 Folder, attrs: Id, OldParent; the old subtree
//! ```

use anyhow::{bail, Result};
use rbx_dom_weak::{types::Ref, InstanceBuilder, WeakDom};
use rbx_types::{Attributes, Variant};
use serde::Serialize;

use crate::diff_document::{DiffDocument, DocumentOp, ManifestNode};

pub const DIFF_CONTAINER_NAME: &str = "__GitRbxDiff";
pub const DIFF_SCHEMA_VERSION: u32 = 1;
const CHUNK_BYTES: usize = 100_000;

/// Instances of a DOM in the order the manifests were captured: each root
/// written to the file, then its subtree pre-order. Both the compact diff
/// DOM and the WeakDom decode the same bytes in the same order, so position
/// is the bridge between manifest ids and live refs.
fn preorder(dom: &WeakDom) -> Vec<Ref> {
    let mut out = Vec::new();
    for &root in dom.root().children() {
        let mut stack = vec![root];
        while let Some(referent) = stack.pop() {
            let Some(instance) = dom.get_by_ref(referent) else {
                continue;
            };
            out.push(referent);
            for &child in instance.children().iter().rev() {
                stack.push(child);
            }
        }
    }
    out
}

/// Pair manifest nodes with live refs by position, verifying name and class
/// at every step so a mismatch fails loudly instead of mislabeling ghosts.
fn bind(dom: &WeakDom, manifest: &[ManifestNode], which: &str) -> Result<Vec<(u32, Ref)>> {
    let refs = preorder(dom);
    if refs.len() != manifest.len() {
        bail!(
            "{which} manifest has {} nodes but the file has {} instances",
            manifest.len(),
            refs.len()
        );
    }
    let mut out = Vec::with_capacity(refs.len());
    for (node, referent) in manifest.iter().zip(refs) {
        let instance = dom.get_by_ref(referent).unwrap();
        if instance.name != node.name || instance.class.as_str() != node.class {
            bail!(
                "{which} manifest node {} ({}:{}) does not match the file's {}:{}",
                node.id,
                node.class,
                node.name,
                instance.class,
                instance.name
            );
        }
        out.push((node.id, referent));
    }
    Ok(out)
}

fn utf8_chunks(value: &str, max_bytes: usize) -> impl Iterator<Item = &str> {
    let mut start = 0;
    std::iter::from_fn(move || {
        if start >= value.len() {
            return None;
        }
        let mut end = (start + max_bytes).min(value.len());
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        let chunk = &value[start..end];
        start = end;
        Some(chunk)
    })
}

fn stamp_chunks(dom: &mut WeakDom, parent: Ref, name: &str, encoded: &str, attrs: Attributes) -> Ref {
    let folder = dom.insert(
        parent,
        InstanceBuilder::new("Folder")
            .with_name(name)
            .with_property("Attributes", Variant::Attributes(attrs)),
    );
    for (index, chunk) in utf8_chunks(encoded, CHUNK_BYTES).enumerate() {
        dom.insert(
            folder,
            InstanceBuilder::new("StringValue")
                .with_name(format!("Chunk_{index:06}"))
                .with_property("Value", Variant::String(chunk.to_string())),
        );
    }
    folder
}

fn stamp_manifest(dom: &mut WeakDom, parent: Ref, name: &str, manifest: &[ManifestNode]) {
    let records: Vec<(u32, u32, &str, &str)> = manifest
        .iter()
        .map(|node| (node.id, node.parent.unwrap_or(0), node.name.as_str(), node.class.as_str()))
        .collect();
    let encoded = serde_json::to_string(&records).expect("manifest is serializable");
    stamp_chunks(
        dom,
        parent,
        name,
        &encoded,
        Attributes::new().with("NodeCount", Variant::Float64(manifest.len() as f64)),
    );
}

/// Deep clone of an old-version subtree for ghost rendering. Ref-valued
/// properties are dropped: the targets are either gone or belong to the old
/// version, and a ghost only needs geometry and appearance.
fn clone_subtree(source: &WeakDom, root: Ref) -> InstanceBuilder {
    let instance = source.get_by_ref(root).unwrap();
    let mut builder = InstanceBuilder::new(instance.class.as_str()).with_name(instance.name.as_str());
    for (name, value) in &instance.properties {
        match value {
            Variant::Ref(_) | Variant::UniqueId(_) => {}
            other => builder = builder.with_property(name.as_str(), other.clone()),
        }
    }
    for &child in instance.children() {
        builder = builder.with_child(clone_subtree(source, child));
    }
    builder
}

fn container_parent(dom: &WeakDom) -> Ref {
    dom.root()
        .children()
        .iter()
        .copied()
        .find(|&c| {
            dom.get_by_ref(c)
                .map(|i| i.class.as_str() == "ServerStorage")
                .unwrap_or(false)
        })
        .unwrap_or_else(|| dom.root_ref())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentBody<'a> {
    ops: &'a [DocumentOp],
    pivots: &'a [crate::diff_document::DocumentPivot],
}

/// Stamp `document` (computed from `old` to `new`) into `new`, cloning the
/// removed subtrees out of `old`. `new` is then written as the temporary
/// file the viewer opens.
pub fn stamp_diff(
    new: &mut WeakDom,
    old: &WeakDom,
    document: &DiffDocument,
    old_path: &str,
    new_path: &str,
) -> Result<()> {
    let new_bindings = bind(new, &document.new, "new")?;
    let old_bindings = bind(old, &document.old, "old")?;

    let parent = container_parent(new);
    let counts = &document.counts;
    let container = new.insert(
        parent,
        InstanceBuilder::new("Folder")
            .with_name(DIFF_CONTAINER_NAME)
            .with_property(
                "Attributes",
                Variant::Attributes(
                    Attributes::new()
                        .with("Version", Variant::Float64(DIFF_SCHEMA_VERSION as f64))
                        .with("OldPath", Variant::String(old_path.to_string()))
                        .with("NewPath", Variant::String(new_path.to_string()))
                        .with("Added", Variant::Float64(counts.added as f64))
                        .with("Removed", Variant::Float64(counts.removed as f64))
                        .with("Modified", Variant::Float64(counts.modified as f64))
                        .with("Reparented", Variant::Float64(counts.reparented as f64))
                        .with("Pivoted", Variant::Float64(counts.pivoted as f64)),
                ),
            ),
    );

    let trees = new.insert(
        container,
        InstanceBuilder::new("Folder").with_name("VirtualTrees"),
    );
    stamp_manifest(new, trees, "Old", &document.old);
    stamp_manifest(new, trees, "New", &document.new);
    let subjects = new.insert(trees, InstanceBuilder::new("Folder").with_name("Subjects"));
    for (id, referent) in &new_bindings {
        new.insert(
            subjects,
            InstanceBuilder::new("ObjectValue")
                .with_name(format!("N{id}"))
                .with_property("Value", Variant::Ref(*referent)),
        );
    }

    let body = serde_json::to_string(&DocumentBody {
        ops: &document.ops,
        pivots: &document.pivots,
    })
    .expect("diff document is serializable");
    stamp_chunks(
        new,
        container,
        "Document",
        &body,
        Attributes::new().with("OpCount", Variant::Float64(document.ops.len() as f64)),
    );

    let removed = new.insert(container, InstanceBuilder::new("Folder").with_name("Removed"));
    let old_by_id: std::collections::HashMap<u32, Ref> = old_bindings.into_iter().collect();
    for op in &document.ops {
        let DocumentOp::Remove { id, .. } = op else {
            continue;
        };
        let Some(&root) = old_by_id.get(id) else {
            continue;
        };
        let old_parent = old
            .get_by_ref(root)
            .and_then(|instance| {
                document
                    .old
                    .iter()
                    .find(|node| node.id == *id)
                    .and_then(|node| node.parent)
                    .map(|_| instance.parent())
            })
            .and_then(|parent_ref| {
                old_by_id
                    .iter()
                    .find(|(_, r)| **r == parent_ref)
                    .map(|(parent_id, _)| *parent_id)
            });
        let mut attrs = Attributes::new().with("Id", Variant::Float64(*id as f64));
        if let Some(parent_id) = old_parent {
            attrs = attrs.with("OldParent", Variant::Float64(parent_id as f64));
        }
        new.insert(
            removed,
            InstanceBuilder::new("Folder")
                .with_name(format!("R{id}"))
                .with_property("Attributes", Variant::Attributes(attrs))
                .with_child(clone_subtree(old, root)),
        );
    }

    Ok(())
}
