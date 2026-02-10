import { useState } from 'react';

interface Vector2Props {
  x: number;
  y: number;
  readOnly?: boolean;
}

interface Vector3Props {
  x: number;
  y: number;
  z: number;
  readOnly?: boolean;
}

export function Vector2Renderer({ x, y, readOnly }: Vector2Props) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className={`vector-renderer ${readOnly ? 'read-only' : ''}`}>
      <span
        className="vector-summary"
        onClick={() => setExpanded(!expanded)}
      >
        <span className="expand-arrow">{expanded ? '▼' : '▶'}</span>
        {(x ?? 0).toFixed(2)}, {(y ?? 0).toFixed(2)}
      </span>
      {expanded && (
        <div className="vector-details">
          <div className="vector-component">
            <span className="component-label">X</span>
            <span className="component-value">{(x ?? 0).toFixed(3)}</span>
          </div>
          <div className="vector-component">
            <span className="component-label">Y</span>
            <span className="component-value">{(y ?? 0).toFixed(3)}</span>
          </div>
        </div>
      )}
    </div>
  );
}

export function Vector3Renderer({ x, y, z, readOnly }: Vector3Props) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className={`vector-renderer ${readOnly ? 'read-only' : ''}`}>
      <span
        className="vector-summary"
        onClick={() => setExpanded(!expanded)}
      >
        <span className="expand-arrow">{expanded ? '▼' : '▶'}</span>
        {(x ?? 0).toFixed(2)}, {(y ?? 0).toFixed(2)}, {(z ?? 0).toFixed(2)}
      </span>
      {expanded && (
        <div className="vector-details">
          <div className="vector-component">
            <span className="component-label">X</span>
            <span className="component-value">{(x ?? 0).toFixed(3)}</span>
          </div>
          <div className="vector-component">
            <span className="component-label">Y</span>
            <span className="component-value">{(y ?? 0).toFixed(3)}</span>
          </div>
          <div className="vector-component">
            <span className="component-label">Z</span>
            <span className="component-value">{(z ?? 0).toFixed(3)}</span>
          </div>
        </div>
      )}
    </div>
  );
}
