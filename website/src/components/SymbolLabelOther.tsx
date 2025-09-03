import { Link } from '@tanstack/react-router';
import { isDecompmeScratch, type SymbolMetadata } from '../api/symbols.tsx';
import DecompmeLogo from './DecompmeLogo.tsx';

export function SymbolLabelOther({
  symbol,
  link = true,
  className = '',
}: {
  symbol: SymbolMetadata;
  link?: boolean;
  className?: string;
}) {
  const content = isDecompmeScratch(symbol) ? (
    <>
      <b>{symbol.name}</b>
      <br />
      <DecompmeLogo />/{symbol.source_name}
    </>
  ) : (
    <>
      <b>{symbol.name}</b>
      <br />
      {symbol.project_name}
      {symbol.version_name ? ` (${symbol.version_name})` : ''}
    </>
  );

  return link ? (
    <Link
      to="/symbol/$symbolSlug"
      params={{ symbolSlug: symbol.slug }}
      className={className}
    >
      {content}
    </Link>
  ) : (
    <span className={className}>{content}</span>
  );
}
