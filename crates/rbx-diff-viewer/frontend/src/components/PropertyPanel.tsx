import { useProperties } from '../hooks/useApi';
import { groupPropertiesByCategory } from '../types/properties';
import { PropertyGroup } from './properties/PropertyGroup';
import type { Side } from '../types/api';

interface PropertyPanelProps {
  instanceRef: string | null;
  side: Side;
}

export function PropertyPanel({ instanceRef, side }: PropertyPanelProps) {
  const { data: properties, loading } = useProperties(
    instanceRef,
    side === 'diff' ? 'new' : side
  );

  const renderContent = () => {
    if (!instanceRef) {
      return <div className="no-selection">Select an instance to view properties</div>;
    }

    if (loading) {
      return <div className="loading">Loading...</div>;
    }

    if (properties && properties.length > 0) {
      const groups = groupPropertiesByCategory(properties);
      // Pass side for ref click handling (undefined for diff = select in both)
      const clickSide = side === 'diff' ? undefined : side;
      return groups.map(group => (
        <PropertyGroup key={group.category} group={group} side={clickSide} />
      ));
    }

    return <div className="no-selection">No properties</div>;
  };

  return (
    <>
      <div className="pane-header">Properties</div>
      <div className="pane-content property-panel">
        {renderContent()}
      </div>
    </>
  );
}
