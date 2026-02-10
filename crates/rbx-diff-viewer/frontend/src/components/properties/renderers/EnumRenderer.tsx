interface EnumRendererProps {
  value: number;
  name?: string;
  readOnly?: boolean;
}

export function EnumRenderer({ value, name, readOnly }: EnumRendererProps) {
  return (
    <span className={`enum-renderer ${readOnly ? 'read-only' : ''}`}>
      {name ?? value}
    </span>
  );
}
