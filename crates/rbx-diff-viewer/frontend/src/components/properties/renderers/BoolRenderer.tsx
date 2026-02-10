interface BoolRendererProps {
  value: boolean;
  readOnly?: boolean;
}

export function BoolRenderer({ value, readOnly }: BoolRendererProps) {
  return (
    <span className={`bool-renderer ${readOnly ? 'read-only' : ''}`}>
      {value ? 'true' : 'false'}
    </span>
  );
}
