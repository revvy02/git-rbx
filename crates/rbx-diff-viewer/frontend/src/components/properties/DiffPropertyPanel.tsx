import { useMemo, useState, memo } from 'react';
import { useDiffProperties } from '../../hooks/useApi';
import type { DiffEntry } from '../../types/api';
import { DiffPropertyRow } from './DiffPropertyRow';

interface DiffPropertyPanelProps {
  diffEntry: DiffEntry | null;
}

export function DiffPropertyPanel({ diffEntry }: DiffPropertyPanelProps) {
  const { data: diffProperties, loading } = useDiffProperties(diffEntry);

  const getHeaderText = () => {
    if (!diffEntry) return 'Properties';
    const name = diffEntry.path.split('/').pop() || diffEntry.path;
    return `Properties (${name})`;
  };

  // Memoize grouped and sorted categories
  const sortedCategories = useMemo(() => {
    if (!diffProperties || diffProperties.length === 0) return [];

    const groups = new Map<string, typeof diffProperties>();
    for (const dp of diffProperties) {
      const category = dp.property.category || 'Other';
      const existing = groups.get(category) || [];
      existing.push(dp);
      groups.set(category, existing);
    }

    const categoryOrder = ['Appearance', 'Data', 'Transform', 'Part', 'Model', 'Physics', 'Behavior', 'Other'];
    return Array.from(groups.entries()).sort(([a], [b]) => {
      const aIndex = categoryOrder.indexOf(a);
      const bIndex = categoryOrder.indexOf(b);
      return (aIndex === -1 ? 99 : aIndex) - (bIndex === -1 ? 99 : bIndex);
    });
  }, [diffProperties]);

  const renderContent = () => {
    if (!diffEntry) {
      return <div className="no-selection">Select a changed instance to view properties</div>;
    }

    if (loading) {
      return <div className="loading">Loading...</div>;
    }

    if (sortedCategories.length > 0) {
      return sortedCategories.map(([category, properties]) => (
        <DiffPropertyGroup key={category} category={category} properties={properties} />
      ));
    }

    return <div className="no-selection">No properties</div>;
  };

  return (
    <>
      <div className="pane-header">{getHeaderText()}</div>
      <div className="pane-content property-panel">
        {renderContent()}
      </div>
    </>
  );
}

// Simplified property group for diff view
interface DiffPropertyGroupProps {
  category: string;
  properties: Array<{
    property: import('../../types/properties').Property;
    diffType: 'added' | 'removed' | 'changed' | 'unchanged';
    oldProperty?: import('../../types/properties').Property;
  }>;
}

const DiffPropertyGroup = memo(function DiffPropertyGroup({ category, properties }: DiffPropertyGroupProps) {
  const [expanded, setExpanded] = useState(true);

  return (
    <div className="property-group">
      <div className="property-group-header" onClick={() => setExpanded(!expanded)}>
        <span className="expand-arrow">{expanded ? '▼' : '▶'}</span>
        <span className="group-name">{category}</span>
      </div>
      {expanded && (
        <div className="property-group-content">
          {properties.map(dp => (
            <DiffPropertyRow
              key={dp.property.name}
              property={dp.property}
              diffType={dp.diffType}
              oldProperty={dp.oldProperty}
            />
          ))}
        </div>
      )}
    </div>
  );
});
