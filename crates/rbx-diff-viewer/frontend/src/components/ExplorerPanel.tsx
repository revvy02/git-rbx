import { ExplorerNode } from './ExplorerNode';
import { PropertyPanel } from './PropertyPanel';
import { DiffPropertyPanel } from './properties/DiffPropertyPanel';
import { ResizeHandle } from './ResizeHandle';
import { useAppContext } from '../context/AppContext';
import type { Side, TreeNode, DiffTreeNode } from '../types/api';
import { useState, useRef, useCallback, useMemo, useEffect } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { fetchChildren } from '../hooks/useApi';
import { flattenTreeNodes, flattenDiffNodes } from '../utils/flattenTree';

const ROW_HEIGHT = 22;

interface ExplorerPanelProps {
  side: Side;
  title: string;
  showProperties?: boolean;
}

export function ExplorerPanel({ side, title, showProperties = true }: ExplorerPanelProps) {
  const { state, clearReveal } = useAppContext();
  const [explorerRatio, setExplorerRatio] = useState(0.6);
  const [searchInput, setSearchInput] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);

  // Lifted expand/collapse state
  const [expandedKeys, setExpandedKeys] = useState<Set<string>>(() => new Set());
  const [loadingKeys, setLoadingKeys] = useState<Set<string>>(() => new Set());
  const [loadGeneration, setLoadGeneration] = useState(0);

  // Pending reveal ref (for two-effect reveal pattern)
  const pendingRevealRef = useRef<string | null>(null);

  // Get tree data
  const tree = side === 'old' ? state.oldTree : side === 'new' ? state.newTree : null;
  const diffTree = side === 'diff' ? state.diffTree : null;
  const refMap = side === 'old' ? state.oldRefMap : side === 'new' ? state.newRefMap : undefined;
  const diffRefs = side !== 'diff' ? state.diffRefs : undefined;
  const selectedRef = side === 'old' ? state.oldSelectedRef : side === 'new' ? state.newSelectedRef : null;

  // Debounce search
  useEffect(() => {
    const timer = setTimeout(() => setSearchQuery(searchInput), 150);
    return () => clearTimeout(timer);
  }, [searchInput]);

  // Initial expansion when tree data loads
  useEffect(() => {
    if (side === 'diff' && diffTree) {
      const initial = new Set<string>();
      function collectAutoExpand(node: DiffTreeNode, key: string) {
        if (node.hasChangedDescendant || node.changeType) {
          initial.add(key);
        }
        for (const childKey of Object.keys(node.children)) {
          collectAutoExpand(node.children[childKey], childKey);
        }
      }
      for (const k of Object.keys(diffTree.children)) {
        collectAutoExpand(diffTree.children[k], k);
      }
      setExpandedKeys(initial);
    } else if (tree) {
      const initial = new Set<string>();
      initial.add(tree.ref);
      setExpandedKeys(initial);
    }
  }, [tree, diffTree, side]);

  // Toggle expand handler
  const toggleExpand = useCallback((key: string, node?: TreeNode) => {
    setExpandedKeys(prev => {
      const next = new Set(prev);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
        // Fetch children if needed for tree nodes
        if (node && (!node.children || node.children.length === 0) && node.has_children && side !== 'diff') {
          setLoadingKeys(prev => { const s = new Set(prev); s.add(key); return s; });
          fetchChildren(key, side as 'old' | 'new')
            .then(children => {
              node.children = children;
              setLoadGeneration(g => g + 1);
              setLoadingKeys(prev => { const s = new Set(prev); s.delete(key); return s; });
            })
            .catch(err => {
              console.error('Failed to fetch children:', err);
              setLoadingKeys(prev => { const s = new Set(prev); s.delete(key); return s; });
            });
        }
      }
      return next;
    });
  }, [side]);

  // Flatten tree into visible rows
  const flatRows = useMemo(() => {
    if (side === 'diff' && diffTree) {
      return flattenDiffNodes(diffTree, expandedKeys, searchQuery, loadingKeys);
    } else if (tree) {
      return flattenTreeNodes(tree, side as 'old' | 'new', expandedKeys, searchQuery, refMap, diffRefs, loadingKeys);
    }
    return [];
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [side, tree, diffTree, expandedKeys, searchQuery, refMap, diffRefs, loadingKeys, loadGeneration]);

  // Virtualizer
  const virtualizer = useVirtualizer({
    count: flatRows.length,
    getScrollElement: () => scrollContainerRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 20,
  });

  // Reveal-in-tree: Effect 1 - expand ancestors
  useEffect(() => {
    if (side === 'diff') return;
    const revealRef = side === 'old' ? state.oldRevealRef : state.newRevealRef;
    if (!revealRef) return;

    const parentMap = side === 'old' ? state.oldParentMap : state.newParentMap;
    const ancestors: string[] = [];
    let current = parentMap[revealRef];
    while (current) {
      ancestors.push(current);
      current = parentMap[current];
    }

    setExpandedKeys(prev => {
      const next = new Set(prev);
      for (const a of ancestors) next.add(a);
      return next;
    });

    pendingRevealRef.current = revealRef;
    clearReveal(side as 'old' | 'new');
  }, [state.oldRevealRef, state.newRevealRef, side, state.oldParentMap, state.newParentMap, clearReveal]);

  // Reveal-in-tree: Effect 2 - scroll to target after flatRows update
  useEffect(() => {
    if (!pendingRevealRef.current) return;
    const target = pendingRevealRef.current;
    const index = flatRows.findIndex(r => r.ref === target);
    if (index !== -1) {
      virtualizer.scrollToIndex(index, { align: 'center', behavior: 'smooth' });
      pendingRevealRef.current = null;
    }
  }, [flatRows, virtualizer]);

  const handleResize = useCallback((delta: number) => {
    if (!containerRef.current) return;
    const containerHeight = containerRef.current.getBoundingClientRect().height;
    const deltaRatio = delta / containerHeight;
    setExplorerRatio(prev => Math.max(0.2, Math.min(0.8, prev + deltaRatio)));
  }, []);

  const treeContent = (
    <div
      ref={scrollContainerRef}
      className="pane-content"
    >
      <div
        style={{
          height: virtualizer.getTotalSize(),
          width: '100%',
          position: 'relative',
        }}
      >
        {virtualizer.getVirtualItems().map(virtualRow => {
          const row = flatRows[virtualRow.index];
          return (
            <ExplorerNode
              key={row.key}
              row={row}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: ROW_HEIGHT,
                transform: `translateY(${virtualRow.start}px)`,
              }}
              searchQuery={searchQuery}
              onToggleExpand={toggleExpand}
            />
          );
        })}
      </div>
    </div>
  );

  return (
    <>
      <div className="panel-header">{title}</div>
      {showProperties ? (
        <div className="split-container" ref={containerRef}>
          <div className="explorer-pane" style={{ flex: explorerRatio }}>
            <div className="pane-header">
              <span>Explorer</span>
              <input
                type="text"
                className="explorer-search"
                placeholder="Search..."
                value={searchInput}
                onChange={(e) => setSearchInput(e.target.value)}
              />
            </div>
            {flatRows.length > 0 ? treeContent : (
              <div className="pane-content">
                <div className="loading">Loading...</div>
              </div>
            )}
          </div>
          <ResizeHandle direction="horizontal" onResize={handleResize} />
          <div className="properties-pane" style={{ flex: 1 - explorerRatio }}>
            {side === 'diff' ? (
              <DiffPropertyPanel diffEntry={state.diffSelectedEntry} />
            ) : (
              <PropertyPanel instanceRef={selectedRef} side={side} />
            )}
          </div>
        </div>
      ) : (
        <div className="panel-content">
          {treeContent}
        </div>
      )}
    </>
  );
}
