import { Link } from '@tanstack/react-router';
import type { SymbolMetadata } from '../api/symbols.tsx';

export function SymbolLabel({
  symbol,
  link = true,
}: {
  symbol: SymbolMetadata;
  link?: boolean;
}) {
  const content = (
    <>
      <b>{symbol.name}</b> - {symbol.project_name}
      {symbol.version_name ? ` (${symbol.version_name})` : ''}
    </>
  );

  return link ? (
    <Link to="/symbol/$symbolSlug" params={{ symbolSlug: symbol.slug }}>
      {content}
    </Link>
  ) : (
    content
  );
}
