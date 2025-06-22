import { createFileRoute } from '@tanstack/react-router';
import { SymbolSubmatches } from '../../components/SymbolSubmatches.tsx';
import { SymbolMatches } from '../../components/SymbolMatches.tsx';
import { useQuery } from '@tanstack/react-query';
import { fetchSymbolMetadata } from '../../api/symbols.tsx';
import { SymbolLabel } from '../../components/SymbolLabel.tsx';

export const Route = createFileRoute('/symbol/$symbolSlug')({
  component: SymbolInfo,
});

function SymbolInfo() {
  const { symbolSlug } = Route.useParams();

  const {
    data: querySymbol,
    isLoading,
    isError,
    error,
  } = useQuery({
    queryKey: ['metadata', symbolSlug],
    queryFn: () => fetchSymbolMetadata(symbolSlug),
  });

  if (isLoading) return <div>Loading query metadata...</div>;
  if (isError)
    return <div style={{ color: 'red' }}>{(error as Error).message}</div>;
  if (!querySymbol)
    return (
      <div style={{ color: 'red' }}>Query symbol data could not be loaded</div>
    );

  return (
    <>
      <h2>
        Match results for <SymbolLabel symbol={querySymbol} link={false} />
      </h2>
      <SymbolMatches slug={symbolSlug} />
      <SymbolSubmatches slug={symbolSlug} querySym={querySymbol} />
    </>
  );
}
