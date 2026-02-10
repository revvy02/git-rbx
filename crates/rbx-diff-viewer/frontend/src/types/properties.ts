/**
 * Attribute entry for Attributes property
 */
export interface AttributeEntry {
  name: string;
  value: InstancePropertyValue;
}

/**
 * Property value types for type-specific rendering (from /api/properties)
 * Named InstancePropertyValue to avoid conflict with PropertyValue in api.ts (used for diffs)
 */
export type InstancePropertyValue =
  | { kind: 'Bool'; value: boolean }
  | { kind: 'Int'; value: number }
  | { kind: 'Float'; value: number }
  | { kind: 'String'; value: string }
  | { kind: 'Vector2'; x: number; y: number }
  | { kind: 'Vector3'; x: number; y: number; z: number }
  | { kind: 'CFrame'; position: [number, number, number]; orientation: [number, number, number] }
  | { kind: 'Color3'; r: number; g: number; b: number }
  | { kind: 'BrickColor'; name: string; r: number; g: number; b: number }
  | { kind: 'Enum'; value: number; name?: string }
  | { kind: 'Ref'; value: string | null }
  | { kind: 'Binary'; size: number }
  | { kind: 'Attributes'; entries: AttributeEntry[] }
  | { kind: 'Tags'; values: string[] }
  | { kind: 'Unknown'; display: string };

/**
 * Property with structured value and metadata
 */
export interface Property {
  name: string;
  value: InstancePropertyValue;
  type: string;
  category: string;
  readOnly: boolean;
}

/**
 * Properties grouped by category
 */
export interface PropertyGroup {
  category: string;
  properties: Property[];
}

/**
 * Group properties by category
 */
export function groupPropertiesByCategory(properties: Property[]): PropertyGroup[] {
  const groups = new Map<string, Property[]>();

  for (const prop of properties) {
    const existing = groups.get(prop.category) || [];
    existing.push(prop);
    groups.set(prop.category, existing);
  }

  // Convert to array and sort by category order
  const categoryOrder = ['Appearance', 'Data', 'Transform', 'Part', 'Model', 'Physics', 'Behavior', 'Other'];

  return Array.from(groups.entries())
    .sort(([a], [b]) => {
      const aIndex = categoryOrder.indexOf(a);
      const bIndex = categoryOrder.indexOf(b);
      return (aIndex === -1 ? 99 : aIndex) - (bIndex === -1 ? 99 : bIndex);
    })
    .map(([category, properties]) => ({ category, properties }));
}

/**
 * Ref info for displaying instance names instead of raw ref IDs
 */
export interface RefInfo {
  name: string;
  path: string;
  class: string;
}

/**
 * Check if a property value is expandable and return its children as Property[]
 * Returns null if the value is not expandable
 */
export function getExpandableChildren(
  value: InstancePropertyValue,
  refInfo?: RefInfo | null
): Property[] | null {
  switch (value.kind) {
    case 'Attributes':
      if (value.entries.length === 0) return null;
      return value.entries.map(e => ({
        name: e.name,
        value: e.value,
        type: e.value.kind,
        category: '',
        readOnly: true,
      }));
    case 'Vector3':
      return [
        { name: 'X', value: { kind: 'Float', value: value.x }, type: 'Float', category: '', readOnly: true },
        { name: 'Y', value: { kind: 'Float', value: value.y }, type: 'Float', category: '', readOnly: true },
        { name: 'Z', value: { kind: 'Float', value: value.z }, type: 'Float', category: '', readOnly: true },
      ];
    case 'Vector2':
      return [
        { name: 'X', value: { kind: 'Float', value: value.x }, type: 'Float', category: '', readOnly: true },
        { name: 'Y', value: { kind: 'Float', value: value.y }, type: 'Float', category: '', readOnly: true },
      ];
    case 'CFrame':
      return [
        {
          name: 'Position',
          value: { kind: 'Vector3', x: value.position[0], y: value.position[1], z: value.position[2] },
          type: 'Vector3',
          category: '',
          readOnly: true,
        },
        {
          name: 'Orientation',
          value: { kind: 'Vector3', x: value.orientation[0], y: value.orientation[1], z: value.orientation[2] },
          type: 'Vector3',
          category: '',
          readOnly: true,
        },
      ];
    case 'Ref':
      // Only expandable if we have refInfo (know the instance name/path)
      if (!refInfo || value.value === null) return null;
      return [
        { name: 'Ref', value: { kind: 'String', value: value.value }, type: 'String', category: '', readOnly: true },
        { name: 'Path', value: { kind: 'String', value: refInfo.path }, type: 'String', category: '', readOnly: true },
      ];
    default:
      return null;
  }
}

/**
 * Format a property value for display
 */
export function formatPropertyValue(value: InstancePropertyValue): string {
  switch (value.kind) {
    case 'Bool':
      return value.value ? 'true' : 'false';
    case 'Int':
    case 'Float':
      return value.value.toFixed(value.kind === 'Float' ? 3 : 0);
    case 'String':
      return value.value;
    case 'Vector2':
      return `${(value.x ?? 0).toFixed(2)}, ${(value.y ?? 0).toFixed(2)}`;
    case 'Vector3':
      return `${(value.x ?? 0).toFixed(2)}, ${(value.y ?? 0).toFixed(2)}, ${(value.z ?? 0).toFixed(2)}`;
    case 'CFrame':
      return `${(value.position?.[0] ?? 0).toFixed(2)}, ${(value.position?.[1] ?? 0).toFixed(2)}, ${(value.position?.[2] ?? 0).toFixed(2)}`;
    case 'Color3':
      return `[${Math.round(value.r * 255)}, ${Math.round(value.g * 255)}, ${Math.round(value.b * 255)}]`;
    case 'BrickColor':
      return value.name;
    case 'Enum':
      return value.name ?? String(value.value);
    case 'Ref':
      return value.value ?? 'nil';
    case 'Binary':
      return `<${value.size} bytes>`;
    case 'Attributes':
      return value.entries.length === 0 ? '(empty)' : `${value.entries.length} attribute${value.entries.length === 1 ? '' : 's'}`;
    case 'Tags':
      return value.values.length === 0 ? '(none)' : value.values.join(', ');
    case 'Unknown':
      return value.display;
  }
}
