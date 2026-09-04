//! Compact, complete input trees for the Studio merge explorer.
//!
//! The conflicted DOM is already a partially merged result, so the original
//! base/ours/theirs hierarchies cannot be reconstructed from it after the
//! merge. Capture those hierarchies while all three input DOMs still exist.
//! Nodes matched to the same base instance share an id across versions;
//! version-specific additions receive their own ids.

use rbx_dom_weak::{types::Ref, WeakDom};
use std::collections::HashMap;

use crate::diff_dom::DomView;

#[derive(Debug)]
pub(crate) struct ExplorerTreeNode {
    pub id: u32,
    pub parent: Option<u32>,
    pub name: String,
    pub class_name: String,
}

#[derive(Debug)]
pub(crate) struct ExplorerTree {
    pub nodes: Vec<ExplorerTreeNode>,
}

#[derive(Debug)]
pub(crate) struct ExplorerTrees {
    pub base: ExplorerTree,
    pub ours: ExplorerTree,
    pub theirs: ExplorerTree,
    /// Logical node id -> concrete instance in the partially merged DOM.
    pub result_subjects: HashMap<u32, Ref>,

    base_ids: HashMap<Ref, u32>,
    ours_ids: HashMap<Ref, u32>,
    theirs_ids: HashMap<Ref, u32>,
}

#[derive(Clone, Copy)]
pub(crate) enum ExplorerVersion {
    Base,
    Ours,
    Theirs,
}

impl ExplorerTrees {
    pub fn capture(
        base: &dyn DomView,
        ours: &dyn DomView,
        theirs: &dyn DomView,
        ours_matched: &HashMap<Ref, Ref>,
        theirs_matched: &HashMap<Ref, Ref>,
    ) -> Self {
        let mut next_id = 1;
        let (base_tree, base_ids) = capture_tree(base, &HashMap::new(), &mut next_id);

        let ours_known = reverse_matched_ids(ours_matched, &base_ids);
        let (ours_tree, ours_ids) = capture_tree(ours, &ours_known, &mut next_id);

        let theirs_known = reverse_matched_ids(theirs_matched, &base_ids);
        let (theirs_tree, theirs_ids) = capture_tree(theirs, &theirs_known, &mut next_id);

        Self {
            base: base_tree,
            ours: ours_tree,
            theirs: theirs_tree,
            result_subjects: HashMap::new(),
            base_ids,
            ours_ids,
            theirs_ids,
        }
    }

    /// Bind virtual identities to the instances that survived or were created
    /// in the partially merged result. Missing bindings are intentional: a
    /// node can exist in an input version without existing in the result.
    pub fn bind_result(
        &mut self,
        result: &WeakDom,
        ours_created: &HashMap<Ref, Ref>,
        theirs_created: &HashMap<Ref, Ref>,
    ) {
        for (&base_ref, &id) in &self.base_ids {
            if result.get_by_ref(base_ref).is_some() {
                self.result_subjects.insert(id, base_ref);
            }
        }
        bind_created(
            result,
            &self.ours_ids,
            ours_created,
            &mut self.result_subjects,
        );
        bind_created(
            result,
            &self.theirs_ids,
            theirs_created,
            &mut self.result_subjects,
        );
    }

    /// Every ref -> id binding for one version.
    pub(crate) fn ids(&self, version: ExplorerVersion) -> &HashMap<Ref, u32> {
        match version {
            ExplorerVersion::Base => &self.base_ids,
            ExplorerVersion::Ours => &self.ours_ids,
            ExplorerVersion::Theirs => &self.theirs_ids,
        }
    }

    pub(crate) fn id_for(&self, version: ExplorerVersion, referent: Ref) -> Option<u32> {
        match version {
            ExplorerVersion::Base => self.base_ids.get(&referent),
            ExplorerVersion::Ours => self.ours_ids.get(&referent),
            ExplorerVersion::Theirs => self.theirs_ids.get(&referent),
        }
        .copied()
    }

    /// Logical ids in a version-specific subtree, in pre-order. Trees are
    /// captured in pre-order, so the first node whose parent is outside the
    /// growing result marks the end of the requested subtree.
    pub(crate) fn subtree_ids(&self, version: ExplorerVersion, root: u32) -> Vec<u32> {
        let tree = match version {
            ExplorerVersion::Base => &self.base,
            ExplorerVersion::Ours => &self.ours,
            ExplorerVersion::Theirs => &self.theirs,
        };
        let Some(start) = tree.nodes.iter().position(|node| node.id == root) else {
            return Vec::new();
        };

        let mut result = vec![root];
        let mut included = std::collections::HashSet::from([root]);
        for node in tree.nodes.iter().skip(start + 1) {
            if node.parent.is_some_and(|parent| included.contains(&parent)) {
                included.insert(node.id);
                result.push(node.id);
            } else {
                break;
            }
        }
        result
    }
}

fn reverse_matched_ids(
    matched: &HashMap<Ref, Ref>,
    base_ids: &HashMap<Ref, u32>,
) -> HashMap<Ref, u32> {
    matched
        .iter()
        .filter_map(|(base_ref, branch_ref)| base_ids.get(base_ref).map(|id| (*branch_ref, *id)))
        .collect()
}

pub(crate) fn capture_tree(
    dom: &dyn DomView,
    known_ids: &HashMap<Ref, u32>,
    next_id: &mut u32,
) -> (ExplorerTree, HashMap<Ref, u32>) {
    let mut nodes = Vec::new();
    let mut ids = HashMap::new();

    fn visit(
        dom: &dyn DomView,
        referent: Ref,
        parent: Option<u32>,
        known_ids: &HashMap<Ref, u32>,
        next_id: &mut u32,
        nodes: &mut Vec<ExplorerTreeNode>,
        ids: &mut HashMap<Ref, u32>,
    ) {
        let instance = dom.get_by_ref(referent).unwrap();
        let id = known_ids.get(&referent).copied().unwrap_or_else(|| {
            let id = *next_id;
            *next_id += 1;
            id
        });
        ids.insert(referent, id);
        nodes.push(ExplorerTreeNode {
            id,
            parent,
            name: instance.name().to_string(),
            class_name: instance.class().to_string(),
        });
        for child in instance.children() {
            visit(dom, child, Some(id), known_ids, next_id, nodes, ids);
        }
    }

    // WeakDom's root is the serialization wrapper. The roots written to an
    // rbxm/rbxl are its children, so those are the explorer roots.
    let root = dom.get_by_ref(dom.root_ref()).unwrap();
    for root in root.children() {
        visit(dom, root, None, known_ids, next_id, &mut nodes, &mut ids);
    }

    (ExplorerTree { nodes }, ids)
}

fn bind_created(
    result: &WeakDom,
    branch_ids: &HashMap<Ref, u32>,
    created: &HashMap<Ref, Ref>,
    subjects: &mut HashMap<u32, Ref>,
) {
    for (&branch_ref, &result_ref) in created {
        let Some(&id) = branch_ids.get(&branch_ref) else {
            continue;
        };
        if result.get_by_ref(result_ref).is_some() {
            subjects.insert(id, result_ref);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbx_dom_weak::InstanceBuilder;

    fn folder(name: &str) -> InstanceBuilder {
        InstanceBuilder::new("Folder").with_name(name)
    }

    #[test]
    fn matched_nodes_share_ids_and_additions_do_not() {
        let base = WeakDom::new(folder("root").with_child(folder("A")));
        let ours = WeakDom::new(folder("root").with_children([folder("A"), folder("O")]));
        let theirs = WeakDom::new(folder("root").with_children([folder("A"), folder("T")]));

        let base_a = base.root().children()[0];
        let ours_a = ours.root().children()[0];
        let theirs_a = theirs.root().children()[0];
        let ours_matched = HashMap::from([(base_a, ours_a)]);
        let theirs_matched = HashMap::from([(base_a, theirs_a)]);

        let trees = ExplorerTrees::capture(&base, &ours, &theirs, &ours_matched, &theirs_matched);
        let id = |tree: &ExplorerTree, name: &str| {
            tree.nodes.iter().find(|node| node.name == name).unwrap().id
        };

        assert_eq!(id(&trees.base, "A"), id(&trees.ours, "A"));
        assert_eq!(id(&trees.base, "A"), id(&trees.theirs, "A"));
        assert_ne!(id(&trees.ours, "O"), id(&trees.theirs, "T"));
    }
}
