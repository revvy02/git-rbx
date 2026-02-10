import { useCallback, memo } from 'react';
import type { TreeNode } from '../types/api';
import type { FlatTreeRow } from '../types/flatTree';
import { useAppContext } from '../context/AppContext';
import { highlightMatch } from '../utils/flattenTree';

interface ExplorerNodeProps {
  row: FlatTreeRow;
  style: React.CSSProperties;
  searchQuery: string;
  onToggleExpand: (key: string, node?: TreeNode) => void;
}

const ROW_HEIGHT = 22;

/** Renders tree line connectors for a virtualized flat row. */
function TreeLines({ depth, isLastChild, ancestorIsLast }: {
  depth: number;
  isLastChild: boolean;
  ancestorIsLast: boolean[];
}) {
  if (depth === 0) return null;
  const lines: React.ReactNode[] = [];

  // Vertical continuation lines for ancestors that are NOT the last child
  for (let d = 0; d < depth - 1; d++) {
    if (!ancestorIsLast[d + 1]) {
      lines.push(
        <div
          key={`v${d}`}
          className="tree-line-vert"
          style={{ left: d * 16 + 7 }}
        />
      );
    }
  }

  // This node's vertical connector (from top to center, or full height if not last)
  const x = (depth - 1) * 16 + 7;
  lines.push(
    <div
      key="vc"
      className="tree-line-vert"
      style={{ left: x, height: isLastChild ? 11 : ROW_HEIGHT }}
    />
  );

  // Horizontal connector from vertical line to node content
  lines.push(
    <div
      key="hc"
      className="tree-line-horiz"
      style={{ left: x, top: 11 }}
    />
  );

  return <>{lines}</>;
}

export const ExplorerNode = memo(function ExplorerNode({
  row,
  style,
  searchQuery,
  onToggleExpand,
}: ExplorerNodeProps) {
  const { state, selectInstance, selectDiffEntry, highlightRef, revealInstance } = useAppContext();

  const isDiffNode = row.kind === 'diff';
  const ref = row.ref;

  // Selection state
  const selectedRef = row.kind === 'tree'
    ? (row.side === 'old' ? state.oldSelectedRef : state.newSelectedRef)
    : null;
  const isSelected = ref != null && selectedRef === ref;
  const isHighlighted = ref != null && state.highlightedRef === ref;

  const rowClasses = [
    'node-row',
    isDiffNode ? 'diff-node' : '',
    row.changeType || '',
    row.kind === 'tree' && row.isUnavailable ? 'unavailable' : '',
    isSelected ? 'selected' : '',
    isHighlighted ? 'highlighted' : '',
  ].filter(Boolean).join(' ');

  const treeNode = row.kind === 'tree' ? row.node : undefined;
  const handleExpand = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    onToggleExpand(row.key, treeNode);
  }, [row.key, treeNode, onToggleExpand]);

  const handleSelect = useCallback(() => {
    if (row.kind === 'tree' && ref) {
      selectInstance(ref, row.side);
      highlightRef(null);
    } else if (row.kind === 'diff' && row.diff) {
      selectDiffEntry(row.diff);
      const oldRef = row.diff.old_ref;
      const newRef = row.diff.new_ref;
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
  }, [row, ref, selectInstance, selectDiffEntry, highlightRef, revealInstance]);

  const chevron = row.hasChildren
    ? (row.isLoading ? '...' : row.isExpanded ? '\u25BC' : '\u25B6')
    : '';

  const className = row.className;

  return (
    <div style={style}>
      <div
        className={rowClasses}
        style={{ paddingLeft: row.depth * 16 + 4 }}
        onClick={handleSelect}
      >
        <TreeLines
          depth={row.depth}
          isLastChild={row.isLastChild}
          ancestorIsLast={row.ancestorIsLast}
        />
        <span
          className="expand-icon"
          onClick={row.hasChildren ? handleExpand : undefined}
        >
          {chevron}
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
        <span className="node-name">{highlightMatch(row.name, searchQuery)}</span>
        {className && <span className="node-class">[{className}]</span>}
      </div>
    </div>
  );
});
