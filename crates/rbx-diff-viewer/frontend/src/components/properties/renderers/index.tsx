// Barrel export for all renderers
export { BoolRenderer } from './BoolRenderer';
export { NumberRenderer } from './NumberRenderer';
export { StringRenderer } from './StringRenderer';
export { Vector2Renderer, Vector3Renderer } from './VectorRenderer';
export { CFrameRenderer } from './CFrameRenderer';
export { Color3Renderer, BrickColorRenderer } from './ColorRenderer';
export { EnumRenderer } from './EnumRenderer';
export { RefRenderer } from './RefRenderer';
export { AttributesRenderer } from './AttributesRenderer';
export { TagsRenderer } from './TagsRenderer';

import type { InstancePropertyValue, RefInfo } from '../../../types/properties';
import { BoolRenderer } from './BoolRenderer';
import { NumberRenderer } from './NumberRenderer';
import { StringRenderer } from './StringRenderer';
import { Vector2Renderer, Vector3Renderer } from './VectorRenderer';
import { CFrameRenderer } from './CFrameRenderer';
import { Color3Renderer, BrickColorRenderer } from './ColorRenderer';
import { EnumRenderer } from './EnumRenderer';
import { RefRenderer } from './RefRenderer';
import { AttributesRenderer } from './AttributesRenderer';
import { TagsRenderer } from './TagsRenderer';

interface PropertyValueRendererProps {
  value: InstancePropertyValue;
  readOnly?: boolean;
  onRefClick?: (refValue: string) => void;
  refInfoMap?: Record<string, RefInfo>;
}

/**
 * Renders any property value based on its kind
 */
export function PropertyValueRenderer({ value, readOnly, onRefClick, refInfoMap }: PropertyValueRendererProps) {
  switch (value.kind) {
    case 'Bool':
      return <BoolRenderer value={value.value} readOnly={readOnly} />;
    case 'Int':
      return <NumberRenderer value={value.value} isFloat={false} readOnly={readOnly} />;
    case 'Float':
      return <NumberRenderer value={value.value} isFloat={true} readOnly={readOnly} />;
    case 'String':
      return <StringRenderer value={value.value} readOnly={readOnly} />;
    case 'Vector2':
      return <Vector2Renderer x={value.x} y={value.y} readOnly={readOnly} />;
    case 'Vector3':
      return <Vector3Renderer x={value.x} y={value.y} z={value.z} readOnly={readOnly} />;
    case 'CFrame':
      return <CFrameRenderer position={value.position} orientation={value.orientation} readOnly={readOnly} />;
    case 'Color3':
      return <Color3Renderer r={value.r} g={value.g} b={value.b} readOnly={readOnly} />;
    case 'BrickColor':
      return <BrickColorRenderer name={value.name} r={value.r} g={value.g} b={value.b} readOnly={readOnly} />;
    case 'Enum':
      return <EnumRenderer value={value.value} name={value.name} readOnly={readOnly} />;
    case 'Ref':
      return <RefRenderer value={value.value} readOnly={readOnly} onClick={onRefClick} refInfo={refInfoMap?.[value.value ?? '']} />;
    case 'Binary':
      return <span className={`binary-renderer ${readOnly ? 'read-only' : ''}`}>&lt;{value.size} bytes&gt;</span>;
    case 'Attributes':
      return <AttributesRenderer entries={value.entries} readOnly={readOnly} />;
    case 'Tags':
      return <TagsRenderer values={value.values} readOnly={readOnly} />;
    case 'Unknown':
      return <span className={`unknown-renderer ${readOnly ? 'read-only' : ''}`}>{value.display}</span>;
    default:
      return <span className="unknown-renderer">Unknown type</span>;
  }
}
