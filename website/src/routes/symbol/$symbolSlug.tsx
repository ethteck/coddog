import { useQuery } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import { fetchSymbolMetadata } from '../../api/symbols.tsx';
import { SymbolLabel } from '../../components/SymbolLabel.tsx';
import { SymbolMatches } from '../../components/SymbolMatches.tsx';
import { SymbolSubmatches } from '../../components/SymbolSubmatches.tsx';

type SymbolMatchSearch = {
  page: number;
};

export const Route = createFileRoute('/symbol/$symbolSlug')({
  component: SymbolInfo,
  validateSearch: (search: Record<string, unknown>): SymbolMatchSearch => {
    return {
      page: (search?.page as number) || 1,
    };
  },
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

  if (isLoadingMetadata) return <div>Loading query metadata...</div>;
  if (isErrorMetadata)
    return (
      <div style={{ color: 'red' }}>{(errorMetadata as Error).message}</div>
    );
  if (!querySymbol)
    return (
      <div style={{ color: 'red' }}>Query symbol data could not be loaded</div>
    );

  return (
    <>
      <h2>
        <SymbolLabel symbol={querySymbol} link={false} />
      </h2>
      <SymbolMatches slug={symbolSlug} />
      <SymbolSubmatches slug={symbolSlug} querySym={querySymbol} />
    </>
  );
}
