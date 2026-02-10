interface NumberRendererProps {
  value: number;
  isFloat?: boolean;
  readOnly?: boolean;
}

export function NumberRenderer({ value, isFloat, readOnly }: NumberRendererProps) {
  const formatted = isFloat ? (value ?? 0).toFixed(3) : String(value ?? 0);

  return (
    <span className={`number-renderer ${readOnly ? 'read-only' : ''}`}>
      {formatted}
    </span>
  );
}
