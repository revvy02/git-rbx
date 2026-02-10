import { useState, useCallback, useMemo, memo } from 'react';
import type { Property } from '../../types/properties';
import { getExpandableChildren, formatPropertyValue } from '../../types/properties';
import { PropertyValueRenderer } from './renderers';
import { useAppContext } from '../../context/AppContext';

interface ExpandablePropertyRowProps {
  property: Property;
  depth?: number;
  side?: 'old' | 'new';  // Side for ref click handling
}

export const ExpandablePropertyRow = memo(function ExpandablePropertyRow({ property, depth = 0, side }: ExpandablePropertyRowProps) {
  const { state, selectInstance, highlightRef, revealInstance } = useAppContext();
  const [expanded, setExpanded] = useState(false);

  // Combine both ref info maps for looking up instance names
  const refInfoMap = useMemo(() => ({
    ...state.oldRefInfo,
    ...state.newRefInfo
  }), [state.oldRefInfo, state.newRefInfo]);

  // Get refInfo for this property if it's a Ref type
  const refInfo = property.value.kind === 'Ref' && property.value.value
    ? refInfoMap[property.value.value]
    : undefined;

  // Pass refInfo to getExpandableChildren so Ref types can be expandable
  const children = getExpandableChildren(property.value, refInfo);
  const isExpandable = children !== null;

  const handleToggle = () => {
    if (isExpandable) {
      setExpanded(!expanded);
    }
  };

  // Handle clicking on a ref property value
  const handleRefClick = useCallback((refValue: string) => {
    if (side) {
      // From OLD or NEW panel - select and reveal only in that explorer
      selectInstance(refValue, side);
      revealInstance(refValue, side);
    } else {
      // No side means diff context - select and reveal in both
      selectInstance(refValue, 'old');
      selectInstance(refValue, 'new');
      revealInstance(refValue, 'old');
      revealInstance(refValue, 'new');
    }
    highlightRef(refValue);
  }, [selectInstance, highlightRef, revealInstance, side]);

  // For expandable properties, show instance name for Refs instead of raw ref ID
  const summaryValue = useMemo(() => {
    if (property.value.kind === 'Ref' && refInfo) {
      return refInfo.name;
    }
    return formatPropertyValue(property.value);
  }, [property.value, refInfo]);

  return (
    <div className="expandable-property">
      {/* Main row - layout: name | type | value (right-aligned) */}
      <div
        className={`property-row ${property.readOnly ? 'read-only' : ''} ${isExpandable ? 'expandable' : ''}`}
        onClick={isExpandable ? handleToggle : undefined}
      >
        <span className="property-name" title={property.name}>
          {isExpandable && (
            <span className="expand-arrow">{expanded ? '▼' : '▶'}</span>
          )}
          {property.name}
        </span>
        <span className="property-type" title={property.type}>
          {property.type}
        </span>
        <span className="property-value">
          {isExpandable ? (
            <span className="property-summary">{summaryValue}</span>
          ) : (
            <PropertyValueRenderer value={property.value} readOnly={property.readOnly} onRefClick={handleRefClick} refInfoMap={refInfoMap} />
          )}
        </span>
      </div>

      {/* Expanded children */}
      {expanded && children && (
        <div className="property-children">
          {children.map(child => (
            <ExpandablePropertyRow
              key={child.name}
              property={child}
              depth={depth + 1}
              side={side}
            />
          ))}
        </div>
      )}
    </div>
  );
});
