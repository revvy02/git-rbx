interface StringRendererProps {
  value: string;
  readOnly?: boolean;
}

export function StringRenderer({ value, readOnly }: StringRendererProps) {
  return (
    <span className={`string-renderer ${readOnly ? 'read-only' : ''}`}>
      {value}
    </span>
  );
}
