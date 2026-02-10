import { useState } from 'react';
import type { PropertyGroup as PropertyGroupType } from '../../types/properties';
import { ExpandablePropertyRow } from './ExpandablePropertyRow';

interface PropertyGroupProps {
  group: PropertyGroupType;
  defaultExpanded?: boolean;
  side?: 'old' | 'new';  // Side for ref click handling
}

export function PropertyGroup({ group, defaultExpanded = true, side }: PropertyGroupProps) {
  const [expanded, setExpanded] = useState(defaultExpanded);

  return (
    <div className="property-group">
      <div
        className="property-group-header"
        onClick={() => setExpanded(!expanded)}
      >
        <span className="expand-arrow">{expanded ? '▼' : '▶'}</span>
        <span className="group-name">{group.category}</span>
      </div>
      {expanded && (
        <div className="property-group-content">
          {group.properties.map(prop => (
            <ExpandablePropertyRow key={prop.name} property={prop} side={side} />
          ))}
        </div>
      )}
    </div>
  );
}
