import { useQuery } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import { fetchSourceMetadata } from '../../../api/sources';
import { fetchSymbolsBySourceSlug } from '../../../api/symbols.tsx';
import { SymbolLabel } from '../../../components/SymbolLabel.tsx';

export const Route = createFileRoute('/source/$sourceSlug/')({
  component: SymbolInfo,
});

function SymbolInfo() {
  const { sourceSlug } = Route.useParams();

  const {
    data: querySource,
    isLoading: isLoadingMetadata,
    isError: isErrorMetadata,
    error: errorMetadata,
  } = useQuery({
    queryKey: ['metadata', sourceSlug],
    queryFn: () => fetchSourceMetadata(sourceSlug),
  });

  const {
    data: querySymbols,
    isLoading: isLoadingSymbols,
    isError: isErrorSymbols,
    error: errorSymbols,
  } = useQuery({
    queryKey: ['symbols', sourceSlug],
    queryFn: () => fetchSymbolsBySourceSlug(sourceSlug),
  });

  if (isLoadingMetadata)
    return <div className="loading">Loading query metadata...</div>;
  if (isErrorMetadata)
    return <div className="error">{(errorMetadata as Error).message}</div>;
  if (!querySource)
    return <div className="error">Query source data could not be loaded</div>;

  return (
    <>
      <h2>{querySource.name}</h2>

      {isLoadingSymbols && <div className="loading">Loading...</div>}
      {isErrorSymbols && (
        <div className="error">{(errorSymbols as Error).message}</div>
      )}
      <ul>
        {querySymbols?.map((sym) => (
          <li key={sym.slug}>
            <SymbolLabel symbol={sym} />
          </li>
        ))}
      </ul>
    </>
  );
}
