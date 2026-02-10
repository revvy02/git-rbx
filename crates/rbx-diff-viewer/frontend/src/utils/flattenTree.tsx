import type { TreeNode, DiffTreeNode, ChangeType } from '../types/api';
import type { FlatTreeNodeRow, FlatDiffNodeRow } from '../types/flatTree';

// ============================================================================
// Search helpers
// ============================================================================

export function nameMatches(name: string, query: string): boolean {
  if (!query) return true;
  return name.toLowerCase().includes(query.toLowerCase());
}

export function treeNodeMatches(node: TreeNode, query: string): boolean {
  if (!query) return true;
  if (nameMatches(node.name, query)) return true;
  return (node.children || []).some(child => treeNodeMatches(child, query));
}

export function diffNodeMatches(node: DiffTreeNode, query: string): boolean {
  if (!query) return true;
  if (nameMatches(node.name, query)) return true;
  return Object.values(node.children).some(child => diffNodeMatches(child, query));
}

export function highlightMatch(text: string, query: string): React.ReactNode {
  if (!query) return text;
  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();
  const idx = lowerText.indexOf(lowerQuery);
  if (idx === -1) return text;
  return (
    <>
      {text.slice(0, idx)}
      <mark>{text.slice(idx, idx + query.length)}</mark>
      {text.slice(idx + query.length)}
    </>
  );
}

// ============================================================================
// Flatten regular TreeNode
// ============================================================================

export function flattenTreeNodes(
  root: TreeNode,
  side: 'old' | 'new',
  expandedKeys: Set<string>,
  searchQuery: string,
  refMap: Record<string, string> | undefined,
  diffRefs: Set<string> | undefined,
  loadingKeys: Set<string>,
): FlatTreeNodeRow[] {
  const result: FlatTreeNodeRow[] = [];

  function walk(
    node: TreeNode,
    depth: number,
    isLastChild: boolean,
    ancestorIsLast: boolean[],
  ) {
    // Search: skip nodes where neither this node nor any descendant matches
    if (searchQuery && !treeNodeMatches(node, searchQuery)) {
      return;
    }

    const hasChildren = node.has_children || (node.children && node.children.length > 0);
    const isExpanded = expandedKeys.has(node.ref);
    const changeType = (refMap?.[node.ref] as ChangeType) ?? null;
    const isUnavailable = !!node.ref && !!diffRefs && !diffRefs.has(node.ref);

    // In search mode, force-expand nodes that have matching descendants but don't match themselves
    const thisMatches = searchQuery ? nameMatches(node.name, searchQuery) : false;
    const subtreeMatches = searchQuery ? treeNodeMatches(node, searchQuery) : true;
    const forceExpand = searchQuery !== '' && subtreeMatches && !thisMatches;
    const effectivelyExpanded = isExpanded || forceExpand;

    result.push({
      kind: 'tree',
      key: node.ref,
      depth,
      name: node.name,
      className: node.class,
      hasChildren: !!hasChildren,
      isExpanded: effectivelyExpanded,
      isLoading: loadingKeys.has(node.ref),
      isLastChild,
      ancestorIsLast,
      node,
      ref: node.ref,
      side,
      changeType,
      isUnavailable,
    });

    if (effectivelyExpanded && node.children && node.children.length > 0) {
      const newAncestorIsLast = [...ancestorIsLast, isLastChild];
      for (let i = 0; i < node.children.length; i++) {
        const child = node.children[i];
        // In search mode, skip children that don't match
        if (searchQuery && !treeNodeMatches(child, searchQuery)) {
          continue;
        }
        // Recompute isLastChild considering filtered siblings
        const remainingSiblings = node.children.slice(i + 1);
        const hasMoreVisible = searchQuery
          ? remainingSiblings.some(s => treeNodeMatches(s, searchQuery))
          : i < node.children.length - 1;
        walk(child, depth + 1, !hasMoreVisible, newAncestorIsLast);
      }
    }
  }

  walk(root, 0, true, []);
  return result;
}

// ============================================================================
// Flatten DiffTreeNode
// ============================================================================

function getSortedDiffChildKeys(node: DiffTreeNode): string[] {
  return Object.keys(node.children).sort((a, b) => {
    const aNode = node.children[a];
    const bNode = node.children[b];
    const order: Record<string, number> = { modified: 0, added: 1, removed: 2 };
    const aOrder = aNode.changeType ? order[aNode.changeType] ?? 3 : 3;
    const bOrder = bNode.changeType ? order[bNode.changeType] ?? 3 : 3;
    if (aOrder !== bOrder) return aOrder - bOrder;
    return aNode.name.localeCompare(bNode.name);
  });
}

export function flattenDiffNodes(
  root: DiffTreeNode,
  expandedKeys: Set<string>,
  searchQuery: string,
  loadingKeys: Set<string>,
): FlatDiffNodeRow[] {
  const result: FlatDiffNodeRow[] = [];

  function walk(
    node: DiffTreeNode,
    nodeKey: string,
    depth: number,
    isLastChild: boolean,
    ancestorIsLast: boolean[],
  ) {
    if (searchQuery && !diffNodeMatches(node, searchQuery)) {
      return;
    }

    const childKeys = getSortedDiffChildKeys(node);
    const hasChildren = childKeys.length > 0;
    const isExpanded = expandedKeys.has(nodeKey);

    // Force-expand in search mode for ancestors of matching nodes
    const thisMatches = searchQuery ? nameMatches(node.name, searchQuery) : false;
    const subtreeMatches = searchQuery ? diffNodeMatches(node, searchQuery) : true;
    const forceExpand = searchQuery !== '' && subtreeMatches && !thisMatches;
    const effectivelyExpanded = isExpanded || forceExpand;

    result.push({
      kind: 'diff',
      key: nodeKey,
      depth,
      name: node.name,
      className: node.class,
      hasChildren,
      isExpanded: effectivelyExpanded,
      isLoading: loadingKeys.has(nodeKey),
      isLastChild,
      ancestorIsLast,
      diffNode: node,
      ref: node.ref,
      changeType: node.changeType,
      diff: node.diff,
    });

    if (effectivelyExpanded && hasChildren) {
      const newAncestorIsLast = [...ancestorIsLast, isLastChild];
      // Filter children by search in search mode
      const visibleChildKeys = searchQuery
        ? childKeys.filter(k => diffNodeMatches(node.children[k], searchQuery))
        : childKeys;
      for (let i = 0; i < visibleChildKeys.length; i++) {
        const k = visibleChildKeys[i];
        walk(
          node.children[k],
          k,
          depth + 1,
          i === visibleChildKeys.length - 1,
          newAncestorIsLast,
        );
      }
    }
  }

  // Start from diff tree root's children (skip the root "DataModel")
  const rootChildKeys = getSortedDiffChildKeys(root);
  const visibleRootKeys = searchQuery
    ? rootChildKeys.filter(k => diffNodeMatches(root.children[k], searchQuery))
    : rootChildKeys;
  for (let i = 0; i < visibleRootKeys.length; i++) {
    const k = visibleRootKeys[i];
    walk(
      root.children[k],
      k,
      0,
      i === visibleRootKeys.length - 1,
      [],
    );
  }

  return result;
}
