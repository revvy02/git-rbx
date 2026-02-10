import { createContext, useContext, useReducer, useEffect, type ReactNode } from 'react';
import type { Meta, DiffEntry, TreeNode, ClassIcons, Side, DiffTreeNode } from '../types/api';
import { useEmbeddedData, type RefInfo } from '../hooks/useApi';

// Re-export RefInfo for backwards compatibility
export type { RefInfo } from '../hooks/useApi';

interface AppState {
  isLoaded: boolean;
  meta: Meta | null;
  oldTree: TreeNode | null;
  newTree: TreeNode | null;
  diffs: DiffEntry[];
  diffTree: DiffTreeNode | null;
  classIcons: ClassIcons;
  oldRefMap: Record<string, 'removed' | 'modified'>;
  newRefMap: Record<string, 'added' | 'modified'>;
  // Ref-to-instance info maps for displaying names
  oldRefInfo: Record<string, RefInfo>;
  newRefInfo: Record<string, RefInfo>;
  // Parent maps for reveal-in-tree (ref → parent ref)
  oldParentMap: Record<string, string>;
  newParentMap: Record<string, string>;
  // Independent selections per side
  oldSelectedRef: string | null;
  newSelectedRef: string | null;
  highlightedRef: string | null;
  // Refs to reveal (expand ancestors and scroll into view)
  oldRevealRef: string | null;
  newRevealRef: string | null;
  // Refs that have properties (diff-relevant instances)
  diffRefs: Set<string>;
  // Selected diff entry for CHANGES panel
  diffSelectedEntry: DiffEntry | null;
}

type Action =
  | { type: 'INIT_FROM_EMBEDDED'; payload: {
      meta: Meta;
      oldTree: TreeNode;
      newTree: TreeNode;
      diffs: DiffEntry[];
      classIcons: ClassIcons;
      oldRefInfo: Record<string, RefInfo>;
      newRefInfo: Record<string, RefInfo>;
    }}
  | { type: 'SELECT_INSTANCE'; payload: { ref: string; side: Side } | null }
  | { type: 'SELECT_DIFF_ENTRY'; payload: DiffEntry | null }
  | { type: 'HIGHLIGHT_REF'; payload: string | null }
  | { type: 'REVEAL_INSTANCE'; payload: { ref: string; side: 'old' | 'new' } }
  | { type: 'CLEAR_REVEAL'; payload: { side: 'old' | 'new' } };

const initialState: AppState = {
  isLoaded: false,
  meta: null,
  oldTree: null,
  newTree: null,
  diffs: [],
  diffTree: null,
  classIcons: {},
  oldRefMap: {},
  newRefMap: {},
  oldRefInfo: {},
  newRefInfo: {},
  oldParentMap: {},
  newParentMap: {},
  oldSelectedRef: null,
  newSelectedRef: null,
  highlightedRef: null,
  oldRevealRef: null,
  newRevealRef: null,
  diffRefs: new Set(),
  diffSelectedEntry: null,
};

// Build parent map from tree (ref → parent ref)
function buildParentMap(tree: TreeNode): Record<string, string> {
  const parentMap: Record<string, string> = {};

  function traverse(node: TreeNode, parentRef: string | null) {
    if (parentRef) {
      parentMap[node.ref] = parentRef;
    }
    for (const child of node.children || []) {
      traverse(child, node.ref);
    }
  }

  traverse(tree, null);
  return parentMap;
}

// Build ref maps from diffs for highlighting changes in trees
function buildRefMaps(diffs: DiffEntry[]): {
  oldRefMap: Record<string, 'removed' | 'modified'>;
  newRefMap: Record<string, 'added' | 'modified'>;
} {
  const oldRefMap: Record<string, 'removed' | 'modified'> = {};
  const newRefMap: Record<string, 'added' | 'modified'> = {};

  for (const diff of diffs) {
    if (diff.type === 'removed' && diff.old_ref) {
      oldRefMap[diff.old_ref] = 'removed';
    } else if (diff.type === 'added' && diff.new_ref) {
      newRefMap[diff.new_ref] = 'added';
    } else if (diff.type === 'modified') {
      if (diff.old_ref) oldRefMap[diff.old_ref] = 'modified';
      if (diff.new_ref) newRefMap[diff.new_ref] = 'modified';
    }
  }

  return { oldRefMap, newRefMap };
}


// Build diff tree from flat diffs using actual tree traversal
function buildDiffTree(
  diffs: DiffEntry[],
  oldTree: TreeNode,
  newTree: TreeNode
): DiffTreeNode {
  // Build ref → node and ref → parent maps
  const nodeMap = new Map<string, TreeNode>();
  const parentMap = new Map<string, TreeNode>();

  function indexTree(node: TreeNode, parent: TreeNode | null) {
    nodeMap.set(node.ref, node);
    if (parent) parentMap.set(node.ref, parent);
    for (const child of node.children || []) {
      indexTree(child, node);
    }
  }
  indexTree(oldTree, null);
  indexTree(newTree, null);

  // Root of diff tree
  const root: DiffTreeNode = {
    name: 'DataModel',
    children: {},
    changeType: null,
    diff: null,
    class: 'DataModel',
    hasChangedDescendant: true,
    ref: null
  };

  for (const diff of diffs) {
    // Get the changed node
    const ref = diff.type === 'removed' ? diff.old_ref : diff.new_ref;
    if (!ref) continue;

    const changedNode = nodeMap.get(ref);
    if (!changedNode) continue;

    // Collect ancestor chain (from root to changed node)
    const ancestors: TreeNode[] = [];
    let current: TreeNode | undefined = changedNode;
    while (current) {
      ancestors.unshift(current);
      current = parentMap.get(current.ref);
    }

    // Build path in diff tree
    let diffNode = root;
    for (let i = 0; i < ancestors.length; i++) {
      const ancestor = ancestors[i];
      const isLeaf = i === ancestors.length - 1;

      const key = ancestor.ref;
      if (!diffNode.children[key]) {
        diffNode.children[key] = {
          name: ancestor.name,
          children: {},
          changeType: isLeaf ? diff.type : null,
          diff: isLeaf ? diff : null,
          class: ancestor.class,
          hasChangedDescendant: !isLeaf,
          ref: ancestor.ref
        };
      } else if (!isLeaf) {
        diffNode.children[key].hasChangedDescendant = true;
      }

      diffNode = diffNode.children[key];

      if (isLeaf) {
        diffNode.changeType = diff.type;
        diffNode.diff = diff;
      }
    }
  }

  return root;
}

function reducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    case 'INIT_FROM_EMBEDDED': {
      const { meta, oldTree, newTree, diffs, classIcons, oldRefInfo, newRefInfo } = action.payload;
      const { oldRefMap, newRefMap } = buildRefMaps(diffs);
      const diffTree = buildDiffTree(diffs, oldTree, newTree);
      const oldParentMap = buildParentMap(oldTree);
      const newParentMap = buildParentMap(newTree);
      const diffRefs = new Set<string>();
      for (const diff of diffs) {
        if (diff.old_ref) diffRefs.add(diff.old_ref);
        if (diff.new_ref) diffRefs.add(diff.new_ref);
      }
      return {
        ...state,
        isLoaded: true,
        meta,
        oldTree,
        newTree,
        diffs,
        diffTree,
        classIcons,
        oldRefMap,
        newRefMap,
        oldRefInfo,
        newRefInfo,
        oldParentMap,
        newParentMap,
        diffRefs,
      };
    }
    case 'SELECT_INSTANCE': {
      const { ref, side } = action.payload ?? { ref: null, side: null };
      if (side === 'old') {
        return { ...state, oldSelectedRef: ref };
      } else if (side === 'new') {
        return { ...state, newSelectedRef: ref };
      }
      return state;
    }
    case 'SELECT_DIFF_ENTRY':
      return { ...state, diffSelectedEntry: action.payload };
    case 'HIGHLIGHT_REF':
      return { ...state, highlightedRef: action.payload };
    case 'REVEAL_INSTANCE': {
      const { ref, side } = action.payload;
      if (side === 'old') {
        return { ...state, oldRevealRef: ref };
      } else {
        return { ...state, newRevealRef: ref };
      }
    }
    case 'CLEAR_REVEAL': {
      const { side } = action.payload;
      if (side === 'old') {
        return { ...state, oldRevealRef: null };
      } else {
        return { ...state, newRevealRef: null };
      }
    }
    default:
      return state;
  }
}

interface AppContextValue {
  state: AppState;
  dispatch: React.Dispatch<Action>;
  selectInstance: (ref: string, side: Side) => void;
  selectDiffEntry: (entry: DiffEntry | null) => void;
  highlightRef: (ref: string | null) => void;
  revealInstance: (ref: string, side: 'old' | 'new') => void;
  clearReveal: (side: 'old' | 'new') => void;
}

const AppContext = createContext<AppContextValue | null>(null);

export function AppProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState);
  const { data: embeddedData, error } = useEmbeddedData();

  // Initialize from embedded data
  useEffect(() => {
    if (embeddedData) {
      dispatch({
        type: 'INIT_FROM_EMBEDDED',
        payload: {
          meta: embeddedData.meta,
          oldTree: embeddedData.oldTree,
          newTree: embeddedData.newTree,
          diffs: embeddedData.diffs,
          classIcons: embeddedData.classIcons,
          oldRefInfo: embeddedData.oldRefInfo,
          newRefInfo: embeddedData.newRefInfo,
        }
      });
    }
  }, [embeddedData]);

  // Show error if embedded data is missing
  if (error) {
    return (
      <div style={{ padding: 20, color: '#f14c4c' }}>
        <h2>Error Loading Data</h2>
        <p>{error}</p>
        <p>This HTML file must be generated by the rbx-diff-viewer CLI.</p>
      </div>
    );
  }

  const selectInstance = (ref: string, side: Side) => {
    dispatch({ type: 'SELECT_INSTANCE', payload: { ref, side } });
  };

  const selectDiffEntry = (entry: DiffEntry | null) => {
    dispatch({ type: 'SELECT_DIFF_ENTRY', payload: entry });
  };

  const highlightRef = (ref: string | null) => {
    dispatch({ type: 'HIGHLIGHT_REF', payload: ref });
  };

  const revealInstance = (ref: string, side: 'old' | 'new') => {
    dispatch({ type: 'REVEAL_INSTANCE', payload: { ref, side } });
  };

  const clearReveal = (side: 'old' | 'new') => {
    dispatch({ type: 'CLEAR_REVEAL', payload: { side } });
  };

  return (
    <AppContext.Provider value={{ state, dispatch, selectInstance, selectDiffEntry, highlightRef, revealInstance, clearReveal }}>
      {children}
    </AppContext.Provider>
  );
}

export function useAppContext() {
  const context = useContext(AppContext);
  if (!context) {
    throw new Error('useAppContext must be used within AppProvider');
  }
  return context;
}
