import type { RefInfo } from '../../../types/properties';

interface RefRendererProps {
  value: string | null;
  readOnly?: boolean;
  onClick?: (refValue: string) => void;
  refInfo?: RefInfo | null;
}

export function RefRenderer({ value, readOnly, onClick, refInfo }: RefRendererProps) {
  if (value === null) {
    return (
      <span className={`ref-renderer nil ${readOnly ? 'read-only' : ''}`}>
        nil
      </span>
    );
  }

  // Show instance name if available, otherwise truncated ref ID
  const displayName = refInfo?.name ?? (value.length > 20 ? `${value.substring(0, 17)}...` : value);
  const isClickable = !!onClick;

  const handleClick = (e: React.MouseEvent) => {
    if (onClick) {
      e.stopPropagation();
      onClick(value);
    }
  };

  return (
    <span
      className={`ref-renderer ${readOnly ? 'read-only' : ''} ${isClickable ? 'clickable' : ''}`}
      title={refInfo ? `${refInfo.path}\n${value}` : value}
      onClick={isClickable ? handleClick : undefined}
    >
      {displayName}
    </span>
  );
}
