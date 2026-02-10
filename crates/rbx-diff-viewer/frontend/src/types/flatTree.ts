import type { TreeNode, DiffTreeNode, ChangeType, DiffEntry } from './api';

/** A single flattened row in the virtualized tree. */
export type FlatTreeRow = FlatTreeNodeRow | FlatDiffNodeRow;

interface FlatRowBase {
  /** Unique key for React */
  key: string;
  /** Nesting depth (0 = root) */
  depth: number;
  /** Node display name */
  name: string;
  /** Class name for icon lookup */
  className: string | null;
  /** Whether this node has children (determines chevron) */
  hasChildren: boolean;
  /** Whether this node is expanded */
  isExpanded: boolean;
  /** Whether this node's children are loading */
  isLoading: boolean;
  /** Whether this is the last sibling */
  isLastChild: boolean;
  /**
   * For tree line rendering: at each ancestor depth, whether that ancestor
   * was the last child of its parent. Length === depth.
   */
  ancestorIsLast: boolean[];
}

export interface FlatTreeNodeRow extends FlatRowBase {
  kind: 'tree';
  node: TreeNode;
  ref: string;
  side: 'old' | 'new';
  changeType: ChangeType;
  isUnavailable: boolean;
}

export interface FlatDiffNodeRow extends FlatRowBase {
  kind: 'diff';
  diffNode: DiffTreeNode;
  ref: string | null;
  changeType: ChangeType;
  diff: DiffEntry | null;
}
