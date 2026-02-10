interface Color3RendererProps {
  r: number;
  g: number;
  b: number;
  readOnly?: boolean;
}

interface BrickColorRendererProps {
  name: string;
  r: number;
  g: number;
  b: number;
  readOnly?: boolean;
}

export function Color3Renderer({ r, g, b, readOnly }: Color3RendererProps) {
  // Color3 values are 0-1, convert to 0-255 for display
  const r255 = Math.round(r * 255);
  const g255 = Math.round(g * 255);
  const b255 = Math.round(b * 255);
  const cssColor = `rgb(${r255}, ${g255}, ${b255})`;

  return (
    <span className={`color-renderer ${readOnly ? 'read-only' : ''}`}>
      <span
        className="color-swatch"
        style={{ backgroundColor: cssColor }}
      />
      <span className="color-values">
        [{r255}, {g255}, {b255}]
      </span>
    </span>
  );
}

export function BrickColorRenderer({ name, r, g, b, readOnly }: BrickColorRendererProps) {
  // BrickColor RGB values are already 0-255
  const cssColor = `rgb(${r}, ${g}, ${b})`;

  return (
    <span className={`color-renderer brick-color ${readOnly ? 'read-only' : ''}`}>
      <span
        className="color-swatch"
        style={{ backgroundColor: cssColor }}
      />
      <span className="color-name">{name}</span>
    </span>
  );
}
