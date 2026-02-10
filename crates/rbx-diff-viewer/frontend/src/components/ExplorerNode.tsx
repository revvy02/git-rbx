import { useState, useCallback, useMemo, memo, useEffect, useRef } from 'react';
import type { TreeNode, DiffTreeNode, Side, ChangeType } from '../types/api';
import { fetchChildren } from '../hooks/useApi';
import { useAppContext } from '../context/AppContext';

interface ExplorerNodeProps {
  // For regular tree nodes
  node?: TreeNode;
  // For diff tree nodes
  diffNode?: DiffTreeNode;
  side: Side;
  depth: number;
  refMap?: Record<string, string>;
  diffRefs?: Set<string>;
  isLastChild?: boolean;  // true if this is the last sibling (for tree line styling)
  searchQuery?: string;
}

// Check if a node name matches the search query
function nameMatches(name: string, query: string): boolean {
  if (!query) return true;
  return name.toLowerCase().includes(query.toLowerCase());
}

// Check if node or any descendant matches search (for TreeNode)
function treeNodeMatches(node: TreeNode, query: string): boolean {
  if (!query) return true;
  if (nameMatches(node.name, query)) return true;
  return (node.children || []).some(child => treeNodeMatches(child, query));
}

// Check if node or any descendant matches search (for DiffTreeNode)
function diffNodeMatches(node: DiffTreeNode, query: string): boolean {
  if (!query) return true;
  if (nameMatches(node.name, query)) return true;
  return Object.values(node.children).some(child => diffNodeMatches(child, query));
}

// Highlight matching text in name
function highlightMatch(text: string, query: string): React.ReactNode {
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

// Check if targetRef is a descendant of nodeRef using parent map
function isAncestorOf(nodeRef: string, targetRef: string, parentMap: Record<string, string>): boolean {
  let current = parentMap[targetRef];
  while (current) {
    if (current === nodeRef) return true;
    current = parentMap[current];
  }
  return false;
}

export const ExplorerNode = memo(function ExplorerNode({ node, diffNode, side, depth, refMap, diffRefs, isLastChild = false, searchQuery = '' }: ExplorerNodeProps) {
  const { state, selectInstance, selectDiffEntry, highlightRef, revealInstance, clearReveal } = useAppContext();
  const rowRef = useRef<HTMLDivElement>(null);

  // Check if this node or descendants match search
  const matchesSearch = useMemo(() => {
    if (!searchQuery) return true;
    if (node) return treeNodeMatches(node, searchQuery);
    if (diffNode) return diffNodeMatches(diffNode, searchQuery);
    return true;
  }, [node, diffNode, searchQuery]);

  // Check if this specific node's name matches
  const thisNodeMatches = useMemo(() => {
    if (!searchQuery) return false;
    const nodeName = node?.name ?? diffNode?.name ?? '';
    return nameMatches(nodeName, searchQuery);
  }, [node, diffNode, searchQuery]);

  // For diff nodes: auto-expand if this node has changed descendants (so changes are visible)
  // For regular nodes: collapse by default if depth > 1
  // Also auto-expand if search matches a descendant
  const shouldAutoExpand = diffNode?.hasChangedDescendant ?? false;
  const hasMatchingDescendant = searchQuery && matchesSearch && !thisNodeMatches;
  const [isCollapsed, setIsCollapsed] = useState(
    searchQuery ? !hasMatchingDescendant : (diffNode ? !shouldAutoExpand : depth > 1)
  );
  const [children, setChildren] = useState<TreeNode[] | null>(
    node?.children && node.children.length > 0 ? node.children : null
  );
  const [isLoading, setIsLoading] = useState(false);

  // Auto-expand when search query changes and matches descendants
  useEffect(() => {
    if (searchQuery && hasMatchingDescendant) {
      setIsCollapsed(false);
    }
  }, [searchQuery, hasMatchingDescendant]);

  // Handle both regular nodes and diff nodes
  const isDiffNode = !!diffNode;
  const name = node?.name ?? diffNode?.name ?? '';
  const className = node?.class ?? diffNode?.class ?? '';
  const ref = node?.ref ?? diffNode?.ref ?? '';

  // Check for children - use has_children flag OR existing children array
  const hasChildrenFlag = node?.has_children ?? false;
  const hasLoadedChildren = (node?.children && node.children.length > 0) || (children && children.length > 0);
  const hasDiffChildren = diffNode && Object.keys(diffNode.children).length > 0;
  const hasChildren = hasChildrenFlag || hasLoadedChildren || hasDiffChildren;

  // Get reveal ref and parent map for this side
  const revealRef = side === 'old' ? state.oldRevealRef : side === 'new' ? state.newRevealRef : null;
  const parentMap = side === 'old' ? state.oldParentMap : side === 'new' ? state.newParentMap : {};

  // Reveal-in-tree: expand if we're an ancestor, scroll if we're the target
  useEffect(() => {
    if (!revealRef || !ref || side === 'diff') return;

    if (ref === revealRef) {
      // This is the target node - scroll into view and clear reveal
      setTimeout(() => {
        rowRef.current?.scrollIntoView({ behavior: 'smooth', block: 'center' });
        clearReveal(side as 'old' | 'new');
      }, 50); // Small delay to let expansions render
    } else if (isAncestorOf(ref, revealRef, parentMap)) {
      // This node is an ancestor of the target - expand it
      setIsCollapsed(false);
      // Load children if needed
      if (!isDiffNode && (!children || children.length === 0) && hasChildren) {
        setIsLoading(true);
        fetchChildren(ref, side as 'old' | 'new')
          .then(setChildren)
          .catch(err => console.error('Failed to fetch children:', err))
          .finally(() => setIsLoading(false));
      }
    }
  }, [revealRef, ref, side, parentMap, isDiffNode, children, hasChildren, clearReveal]);

  // Get change type for highlighting
  let changeType: ChangeType = null;
  if (isDiffNode) {
    changeType = diffNode.changeType;
  } else if (refMap && ref) {
    changeType = (refMap[ref] as ChangeType) || null;
  }

  // Non-diff instances are "unavailable" (no properties embedded)
  const isUnavailable = !isDiffNode && !!ref && !!diffRefs && !diffRefs.has(ref);

  // Check if this node is selected or highlighted (per-side independent selections)
  const selectedRef = side === 'old' ? state.oldSelectedRef : side === 'new' ? state.newSelectedRef : null;
  const isSelected = selectedRef === ref;
  const isHighlighted = state.highlightedRef === ref;

  const handleExpand = useCallback(async (e: React.MouseEvent) => {
    e.stopPropagation();

    setIsCollapsed(wasCollapsed => {
      // For non-diff nodes, fetch children if we don't have them yet
      if (!isDiffNode && wasCollapsed && (!children || children.length === 0) && hasChildren && ref) {
        setIsLoading(true);
        fetchChildren(ref, side as 'old' | 'new')
          .then(setChildren)
          .catch(err => console.error('Failed to fetch children:', err))
          .finally(() => setIsLoading(false));
      }
      return !wasCollapsed;
    });
  }, [children, hasChildren, ref, side, isDiffNode]);

  const handleSelect = useCallback(() => {
    if (ref && !isDiffNode) {
      selectInstance(ref, side);
      highlightRef(null); // Clear cross-panel highlight
    } else if (isDiffNode && diffNode?.diff) {
      // For diff nodes, select the diff entry for properties panel
      selectDiffEntry(diffNode.diff);
      // Select and reveal the corresponding instances in both OLD and NEW explorers
      const oldRef = diffNode.diff.old_ref;
      const newRef = diffNode.diff.new_ref;
      if (oldRef) {
        selectInstance(oldRef, 'old');
        revealInstance(oldRef, 'old');
        highlightRef(oldRef);
      }
      if (newRef) {
        selectInstance(newRef, 'new');
        revealInstance(newRef, 'new');
        highlightRef(newRef);
      }
    }
  }, [ref, side, isDiffNode, diffNode, selectInstance, selectDiffEntry, highlightRef, revealInstance]);

  // Build class names for the row
  const rowClasses = [
    'node-row',
    isDiffNode ? 'diff-node' : '',
    changeType || '',
    isUnavailable ? 'unavailable' : '',
    isSelected ? 'selected' : '',
    isHighlighted ? 'highlighted' : ''
  ].filter(Boolean).join(' ');

  // Memoize sorted diff children keys (keyed by ref, sorted by change type then name)
  const sortedDiffChildKeys = useMemo(() => {
    if (!isDiffNode || !diffNode) return [];
    return Object.keys(diffNode.children).sort((a, b) => {
      const aNode = diffNode.children[a];
      const bNode = diffNode.children[b];
      const order: Record<string, number> = { modified: 0, added: 1, removed: 2 };
      const aOrder = aNode.changeType ? order[aNode.changeType] ?? 3 : 3;
      const bOrder = bNode.changeType ? order[bNode.changeType] ?? 3 : 3;
      if (aOrder !== bOrder) return aOrder - bOrder;
      return aNode.name.localeCompare(bNode.name);
    });
  }, [isDiffNode, diffNode]);

  // Render children - only called when not collapsed
  const renderChildren = () => {
    if (isDiffNode && diffNode) {
      return sortedDiffChildKeys.map((childKey, index) => (
        <ExplorerNode
          key={childKey}
          diffNode={diffNode.children[childKey]}
          side={side}
          depth={depth + 1}
          isLastChild={index === sortedDiffChildKeys.length - 1}
          searchQuery={searchQuery}
        />
      ));
    } else if (children) {
      return children.map((child, index) => (
        <ExplorerNode
          key={child.ref}
          node={child}
          side={side}
          depth={depth + 1}
          refMap={refMap}
          diffRefs={diffRefs}
          isLastChild={index === children.length - 1}
          searchQuery={searchQuery}
        />
      ));
    }
    return null;
  };

  // Hide node if it doesn't match search and has no matching descendants
  if (searchQuery && !matchesSearch) {
    return null;
  }

  return (
    <div className={`tree-node${isLastChild ? ' last-child' : ''}`}>
      <div ref={rowRef} className={rowClasses} onClick={handleSelect}>
        <span
          className="expand-icon"
          onClick={hasChildren ? handleExpand : undefined}
        >
          {hasChildren ? (isLoading ? '...' : isCollapsed ? '▶' : '▼') : ''}
        </span>
        {className && state.classIcons[className] ? (
          <img
            className="class-icon"
            src={state.classIcons[className]}
            alt={className}
          />
        ) : (
          <span className="class-icon-placeholder" />
        )}
        <span className="node-name">{highlightMatch(name, searchQuery)}</span>
        {className && <span className="node-class">[{className}]</span>}
      </div>
      {hasChildren && !isCollapsed && (
        <div className="children">
          {renderChildren()}
        </div>
      )}
    </div>
  );
});
