import { useQuery } from '@tanstack/react-query';
import { createFileRoute, Link } from '@tanstack/react-router';
import { fetchSymbolAsm, fetchSymbolMetadata } from '../../../api/symbols.tsx';
import { SymbolLabel } from '../../../components/SymbolLabel.tsx';
import { SymbolMatches } from '../../../components/SymbolMatches.tsx';

type SymbolMatchSearch = {
  page: number;
};

export const Route = createFileRoute('/symbol/$symbolSlug/')({
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

  return (
    <>
      <h2>
        <SymbolLabel symbol={querySymbol} link={false} />
      </h2>

      {querySymbol.project_repo && (
        <Link to={querySymbol.project_repo}>Repo</Link>
      )}

      <SymbolMatches slug={symbolSlug} />

      <Link
        to="/symbol/$symbolSlug/submatch"
        params={{ symbolSlug }}
        className="button"
      >
        Search submatches
      </Link>
    </>
  );
}
