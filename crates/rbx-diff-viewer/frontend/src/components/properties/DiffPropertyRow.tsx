import { useState, useCallback, useMemo } from 'react';
import type { Property } from '../../types/properties';
import { getExpandableChildren, formatPropertyValue } from '../../types/properties';
import { PropertyValueRenderer } from './renderers';
import { useAppContext } from '../../context/AppContext';

interface DiffPropertyRowProps {
  property: Property;
  diffType: 'added' | 'removed' | 'changed' | 'unchanged';
  oldProperty?: Property; // For 'changed' - renders as separate red row above
  depth?: number;
}

export function DiffPropertyRow({ property, diffType, oldProperty, depth = 0 }: DiffPropertyRowProps) {
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

  // For expandable properties, show instance name for Refs instead of raw ref ID
  const summaryValue = useMemo(() => {
    if (property.value.kind === 'Ref' && refInfo) {
      return refInfo.name;
    }
    return formatPropertyValue(property.value);
  }, [property.value, refInfo]);

  // Same for old property
  const oldSummaryValue = useMemo(() => {
    if (oldProperty?.value.kind === 'Ref') {
      const oldRefInfo = oldProperty.value.value ? refInfoMap[oldProperty.value.value] : null;
      if (oldRefInfo) return oldRefInfo.name;
    }
    return oldProperty ? formatPropertyValue(oldProperty.value) : '';
  }, [oldProperty, refInfoMap]);

  const handleToggle = () => {
    if (isExpandable) {
      setExpanded(!expanded);
    }
  };

  // Handle clicking on a ref property value - select and reveal in both explorers
  const handleRefClick = useCallback((refValue: string) => {
    selectInstance(refValue, 'old');
    selectInstance(refValue, 'new');
    revealInstance(refValue, 'old');
    revealInstance(refValue, 'new');
    highlightRef(refValue);
  }, [selectInstance, highlightRef, revealInstance]);

  // Determine CSS class based on diffType
  const getDiffClass = (type: 'added' | 'removed' | 'changed' | 'unchanged', isOld: boolean = false) => {
    if (type === 'added') return 'diff-added';
    if (type === 'removed') return 'diff-removed';
    if (type === 'changed') return isOld ? 'diff-old' : 'diff-new';
    return '';
  };

  return (
    <div className="expandable-property">
      {/* For changed properties, render old value row first (red) */}
      {diffType === 'changed' && oldProperty && (
        <div
          className={`property-row ${oldProperty.readOnly ? 'read-only' : ''} ${getDiffClass('changed', true)}`}
        >
          <span className="property-name" title={oldProperty.name}>
            {isExpandable && <span className="expand-arrow" style={{ visibility: 'hidden' }}>▶</span>}
            {oldProperty.name}
          </span>
          <span className="property-type" title={oldProperty.type}>
            {oldProperty.type}
          </span>
          <span className="property-value">
            {isExpandable ? (
              <span className="property-summary">{oldSummaryValue}</span>
            ) : (
              <PropertyValueRenderer value={oldProperty.value} readOnly={oldProperty.readOnly} onRefClick={handleRefClick} refInfoMap={refInfoMap} />
            )}
          </span>
        </div>
      )}

      {/* Main row (new value for changed, or the only value for added/removed/unchanged) */}
      <div
        className={`property-row ${property.readOnly ? 'read-only' : ''} ${isExpandable ? 'expandable' : ''} ${getDiffClass(diffType, false)}`}
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

      {/* Expanded children - inherit diff type for added/removed instances */}
      {expanded && children && (
        <div className="property-children">
          {children.map(child => (
            <DiffPropertyRow
              key={child.name}
              property={child}
              diffType={diffType === 'changed' ? 'unchanged' : diffType}
              depth={depth + 1}
            />
          ))}
        </div>
      )}
    </div>
  );
}
