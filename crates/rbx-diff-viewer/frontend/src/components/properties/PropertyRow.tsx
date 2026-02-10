import type { Property } from '../../types/properties';
import { PropertyValueRenderer } from './renderers';

interface PropertyRowProps {
  property: Property;
}

export function PropertyRow({ property }: PropertyRowProps) {
  return (
    <div className={`property-row ${property.readOnly ? 'read-only' : ''}`}>
      <span className="property-name" title={property.name}>
        {property.name}
      </span>
      <span className="property-value">
        <PropertyValueRenderer value={property.value} readOnly={property.readOnly} />
      </span>
      <span className="property-type" title={property.type}>
        {property.type}
      </span>
    </div>
  );
}
