//! Small shared queries over Roblox instance trees and reflection metadata.

use rbx_dom_weak::{types::Ref, WeakDom};
use std::collections::HashSet;

pub(crate) fn class_is_a(class_name: &str, ancestor: &str) -> bool {
    let Ok(database) = rbx_reflection_database::get() else {
        return false;
    };
    let mut current = class_name;
    loop {
        if current == ancestor {
            return true;
        }
        let Some(class) = database.classes.get(current) else {
            return false;
        };
        let Some(parent) = class.superclass.as_ref() else {
            return false;
        };
        current = parent;
    }
}

pub(crate) fn ancestors(dom: &WeakDom, mut referent: Ref) -> Vec<Ref> {
    let mut result = vec![referent];
    while let Some(instance) = dom.get_by_ref(referent) {
        let parent = instance.parent();
        if parent.is_none() {
            break;
        }
        result.push(parent);
        referent = parent;
    }
    result
}

pub(crate) fn lowest_common_ancestor(dom: &WeakDom, referents: &[Ref]) -> Ref {
    let (&first, rest) = referents
        .split_first()
        .expect("lowest_common_ancestor requires at least one referent");
    let mut result = first;
    for &other in rest {
        let result_ancestors: HashSet<Ref> = ancestors(dom, result).into_iter().collect();
        let mut candidate = other;
        loop {
            if result_ancestors.contains(&candidate) {
                result = candidate;
                break;
            }
            let Some(parent) = dom.get_by_ref(candidate).map(|instance| instance.parent()) else {
                break;
            };
            if parent.is_none() {
                break;
            }
            candidate = parent;
        }
    }
    result
}

pub(crate) fn is_descendant_or_same(dom: &WeakDom, mut node: Ref, ancestor: Ref) -> bool {
    loop {
        if node == ancestor {
            return true;
        }
        let Some(instance) = dom.get_by_ref(node) else {
            return false;
        };
        let parent = instance.parent();
        if parent.is_none() {
            return false;
        }
        node = parent;
    }
}
