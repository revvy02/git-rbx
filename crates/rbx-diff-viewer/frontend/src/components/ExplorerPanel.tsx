import { ExplorerNode } from './ExplorerNode';
import { PropertyPanel } from './PropertyPanel';
import { DiffPropertyPanel } from './properties/DiffPropertyPanel';
import { ResizeHandle } from './ResizeHandle';
import { useAppContext } from '../context/AppContext';
import type { Side } from '../types/api';
import { useState, useRef, useCallback, useMemo } from 'react';

interface ExplorerPanelProps {
  side: Side;
  title: string;
  showProperties?: boolean;
}

export function ExplorerPanel({ side, title, showProperties = true }: ExplorerPanelProps) {
  const { state } = useAppContext();
  // Use ratio for explorer/properties split (0.6 = 60% explorer, 40% properties)
  const [explorerRatio, setExplorerRatio] = useState(0.6);
  const [searchQuery, setSearchQuery] = useState('');
  const containerRef = useRef<HTMLDivElement>(null);

  // Get the appropriate tree and ref map
  const tree = side === 'old' ? state.oldTree : side === 'new' ? state.newTree : null;
  const diffTree = side === 'diff' ? state.diffTree : null;
  const refMap = side === 'old' ? state.oldRefMap : side === 'new' ? state.newRefMap : undefined;
  const diffRefs = side !== 'diff' ? state.diffRefs : undefined;

  // Get selected ref for this side (independent per panel)
  const selectedRef = side === 'old' ? state.oldSelectedRef : side === 'new' ? state.newSelectedRef : null;

  const handleResize = useCallback((delta: number) => {
    if (!containerRef.current) return;

    const containerHeight = containerRef.current.getBoundingClientRect().height;
    const deltaRatio = delta / containerHeight;
    setExplorerRatio(prev => Math.max(0.2, Math.min(0.8, prev + deltaRatio)));
  }, []);

  // Memoize sorted diff tree keys
  const sortedDiffKeys = useMemo(() => {
    if (side !== 'diff' || !diffTree) return [];
    return Object.keys(diffTree.children).sort();
  }, [side, diffTree]);

  // Memoize tree rendering
  const treeContent = useMemo(() => {
    if (side === 'diff' && diffTree) {
      return sortedDiffKeys.map(name => (
        <ExplorerNode
          key={name}
          diffNode={diffTree.children[name]}
          side={side}
          depth={0}
          searchQuery={searchQuery}
        />
      ));
    } else if (tree) {
      return (
        <ExplorerNode
          node={tree}
          side={side}
          depth={0}
          refMap={refMap}
          diffRefs={diffRefs}
          searchQuery={searchQuery}
        />
      );
    }
    return <div className="loading">Loading...</div>;
  }, [side, diffTree, sortedDiffKeys, tree, refMap, diffRefs, searchQuery]);

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
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            </div>
            <div className="pane-content">
              {treeContent}
            </div>
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
