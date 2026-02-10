// Types matching Rust structs from rbx-diff and rbx-diff-viewer

export interface TreeNode {
  name: string;
  class: string;
  ref: string;
  children: TreeNode[];
  has_children: boolean;
}

export interface Meta {
  old_name: string;
  new_name: string;
  summary: {
    added: number;
    removed: number;
    modified: number;
  };
}

export interface DiffEntry {
  type: 'added' | 'removed' | 'modified';
  old_ref?: string;
  new_ref?: string;
  path: string;
  class: string;
  property_changes?: PropertyChange[];
}

export interface PropertyChange {
  name: string;
  old_value: PropertyValue | null;
  new_value: PropertyValue | null;
}

export type PropertyValue =
  | { type: 'nil' }
  | { type: 'bool'; value: { value: boolean } }
  | { type: 'int32'; value: { value: number } }
  | { type: 'int64'; value: { value: number } }
  | { type: 'float32'; value: { value: number | null } }
  | { type: 'float64'; value: { value: number | null } }
  | { type: 'string'; value: { value: string } }
  | { type: 'binary_string'; value: { len: number } }
  | { type: 'ref'; value: { value: string } }
  | { type: 'vector2'; value: { x: number; y: number } }
  | { type: 'vector3'; value: { x: number; y: number; z: number } }
  | { type: 'cframe'; value: { position: [number, number, number]; orientation: [[number, number, number], [number, number, number], [number, number, number]] } }
  | { type: 'color3'; value: { r: number; g: number; b: number } }
  | { type: 'brick_color'; value: { value: number } }
  | { type: 'enum'; value: { value: number } }
  | { type: 'udim'; value: { scale: number; offset: number } }
  | { type: 'udim2'; value: { x_scale: number; x_offset: number; y_scale: number; y_offset: number } }
  | { type: 'number_range'; value: { min: number; max: number } }
  | { type: 'number_sequence'; value: { keypoints: NumberKeypoint[] } }
  | { type: 'color_sequence'; value: { keypoints: ColorKeypoint[] } }
  | { type: 'rect'; value: { min_x: number; min_y: number; max_x: number; max_y: number } }
  | { type: 'other'; value: { type_name: string } };

export interface NumberKeypoint {
  time: number;
  value: number;
  envelope: number;
}

export interface ColorKeypoint {
  time: number;
  r: number;
  g: number;
  b: number;
}

// Re-export Property type from properties.ts (new structured format from backend)
export type { Property, InstancePropertyValue } from './properties';

// ClassIcons maps class name to base64 data URL
export type ClassIcons = Record<string, string>;

export type ChangeType = 'added' | 'removed' | 'modified' | null;

export type Side = 'old' | 'new' | 'diff';

// Diff tree node (built from flat diff entries)
export interface DiffTreeNode {
  name: string;
  children: Record<string, DiffTreeNode>;
  changeType: ChangeType;              // null for ancestor nodes (context only)
  diff: DiffEntry | null;              // Only for changed leaf nodes
  class: string | null;                // Class name for icon
  hasChangedDescendant: boolean;       // True if any descendant has changes (for auto-expand)
  ref: string | null;                  // Ref for selection syncing
}
