import { useState } from 'react';
import type { AttributeEntry, InstancePropertyValue } from '../../../types/properties';
import { formatPropertyValue } from '../../../types/properties';

interface AttributesRendererProps {
  entries: AttributeEntry[];
  readOnly?: boolean;
}

// Inline renderer for attribute values to avoid circular import
function AttributeValueRenderer({ value }: { value: InstancePropertyValue }) {
  return <span className="attribute-value">{formatPropertyValue(value)}</span>;
}

export function AttributesRenderer({ entries, readOnly }: AttributesRendererProps) {
  const [expanded, setExpanded] = useState(false);

  if (entries.length === 0) {
    return <span className={`attributes-empty ${readOnly ? 'read-only' : ''}`}>(empty)</span>;
  }

  return (
    <div className={`attributes-renderer ${readOnly ? 'read-only' : ''}`}>
      <span
        className="attributes-summary"
        onClick={() => setExpanded(!expanded)}
      >
        <span className="expand-arrow">{expanded ? '▼' : '▶'}</span>
        {entries.length} attribute{entries.length === 1 ? '' : 's'}
      </span>
      {expanded && (
        <div className="attributes-details">
          {entries.map((entry) => (
            <div key={entry.name} className="attribute-entry">
              <span className="attribute-name">{entry.name}</span>
              <AttributeValueRenderer value={entry.value} />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
