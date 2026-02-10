import { useState } from 'react';

interface CFrameRendererProps {
  position: [number, number, number];
  orientation: [number, number, number];
  readOnly?: boolean;
}

export function CFrameRenderer({ position, orientation, readOnly }: CFrameRendererProps) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className={`cframe-renderer ${readOnly ? 'read-only' : ''}`}>
      <span
        className="cframe-summary"
        onClick={() => setExpanded(!expanded)}
      >
        <span className="expand-arrow">{expanded ? '▼' : '▶'}</span>
        {(position?.[0] ?? 0).toFixed(2)}, {(position?.[1] ?? 0).toFixed(2)}, {(position?.[2] ?? 0).toFixed(2)}
      </span>
      {expanded && (
        <div className="cframe-details">
          <div className="cframe-section">
            <div className="section-header">
              <span className="expand-arrow">▼</span>
              Position
            </div>
            <div className="section-content">
              <div className="cframe-component">
                <span className="component-label">X</span>
                <span className="component-value">{(position?.[0] ?? 0).toFixed(3)}</span>
              </div>
              <div className="cframe-component">
                <span className="component-label">Y</span>
                <span className="component-value">{(position?.[1] ?? 0).toFixed(3)}</span>
              </div>
              <div className="cframe-component">
                <span className="component-label">Z</span>
                <span className="component-value">{(position?.[2] ?? 0).toFixed(3)}</span>
              </div>
            </div>
          </div>
          <div className="cframe-section">
            <div className="section-header">
              <span className="expand-arrow">▼</span>
              Orientation
            </div>
            <div className="section-content">
              <div className="cframe-component">
                <span className="component-label">R</span>
                <span className="component-value">{(orientation?.[0] ?? 0).toFixed(1)}</span>
              </div>
              <div className="cframe-component">
                <span className="component-label">P</span>
                <span className="component-value">{(orientation?.[1] ?? 0).toFixed(1)}</span>
              </div>
              <div className="cframe-component">
                <span className="component-label">Y</span>
                <span className="component-value">{(orientation?.[2] ?? 0).toFixed(1)}</span>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
