import { useQuery } from '@tanstack/react-query';
import { createFileRoute, Link } from '@tanstack/react-router';
import { ExternalLink } from 'lucide-react';
import {
  fetchSymbolAsm,
  fetchSymbolMetadata,
  isDecompmeScratch,
} from '../../../api/symbols.tsx';
import DecompmeLogo from '../../../components/DecompmeLogo.tsx';
import { SymbolMatches } from '../../../components/SymbolMatches.tsx';

export const Route = createFileRoute('/symbol/$symbolSlug/')({
  component: SymbolInfo,
});

function SymbolInfo() {
  const { symbolSlug } = Route.useParams();

  const {
    data: querySymbol,
    isLoading: isLoadingMetadata,
    isError: isErrorMetadata,
    error: errorMetadata,
  } = useQuery({
    queryKey: ['metadata', symbolSlug],
    queryFn: () => fetchSymbolMetadata(symbolSlug),
  });

  const {
    data: queryAsm,
    isLoading: isLoadingAsm,
    isError: isErrorAsm,
    error: errorAsm,
  } = useQuery({
    queryKey: ['asm', symbolSlug],
    queryFn: () => fetchSymbolAsm(symbolSlug),
  });

  if (isLoadingMetadata)
    return <div className="loading">Loading query metadata...</div>;
  if (isErrorMetadata)
    return <div className="error">{(errorMetadata as Error).message}</div>;
  if (!querySymbol)
    return <div className="error">Query symbol data could not be loaded</div>;

  if (isLoadingAsm)
    return <div className="loading">Loading query assembly...</div>;
  if (isErrorAsm)
    return <div className="error">{(errorAsm as Error).message}</div>;
  if (!queryAsm)
    return <div className="error">Query assembly data could not be loaded</div>;

  const content = isDecompmeScratch(querySymbol) ? (
    <>
      <b>{querySymbol.name}</b> - <DecompmeLogo />
    </>
  ) : (
    <>
      <b>{querySymbol.name}</b> - {querySymbol.project_name}
      {querySymbol.version_name ? ` (${querySymbol.version_name})` : ''}
    </>
  );

  const submatchesAvailable = querySymbol.len > 3;

  const submatchContent = submatchesAvailable ? (
    <Link
      to="/symbol/$symbolSlug/submatch"
      params={{ symbolSlug }}
      className="button"
    >
      Search submatches
    </Link>
  ) : (
    <p className="info">This symbol is too short to contain submatches.</p>
  );

  return (
    <>
      <h2>{content}</h2>

      {isDecompmeScratch(querySymbol) && (
        <Link
          href={`https://decomp.me/scratch/${querySymbol.source_name}`}
          className="decomp-logo"
        >
          <DecompmeLogo />
          {'/'}
          {querySymbol.source_name}{' '}
          <ExternalLink style={{ width: '16px', height: '16px' }} />
        </Link>
      )}

      <SymbolMatches slug={symbolSlug} />

      {submatchContent}
    </>
  );
}
