import { Link } from '@tanstack/react-router';
import { SymbolMetadata } from '../api/symbols.tsx';

export function SymbolLabel({ symbol }: { symbol: SymbolMetadata }) {
  return (
    <Link to="/symbol/$symbolSlug" params={{ symbolSlug: symbol.slug }}>
      <b>{symbol.name}</b> - {symbol.project_name}{' '}
      {symbol.version_name ? symbol.version_name : ''}
    </Link>
  );
}
