interface TagsRendererProps {
  values: string[];
  readOnly?: boolean;
}

export function TagsRenderer({ values, readOnly }: TagsRendererProps) {
  if (values.length === 0) {
    return <span className={`tags-empty ${readOnly ? 'read-only' : ''}`}>(none)</span>;
  }

  return (
    <div className={`tags-renderer ${readOnly ? 'read-only' : ''}`}>
      {values.map((tag) => (
        <span key={tag} className="tag-chip">
          {tag}
        </span>
      ))}
    </div>
  );
}
