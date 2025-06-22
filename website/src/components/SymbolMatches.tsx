import { useQuery } from '@tanstack/react-query';
import { fetchSymbolMatches, SymbolMetadata } from '../api/symbols.tsx';
import { SymbolLabel } from './SymbolLabel.tsx';

export function SymbolMatches({ slug }: { slug: string }) {
  const {
    data: matchResults,
    isLoading,
    isError,
    error,
  } = useQuery({
    queryKey: ['match', slug],
    queryFn: () => fetchSymbolMatches(slug),
  });

  if (isLoading) return <div>Loading match results...</div>;
  if (isError)
    return <div style={{ color: 'red' }}>{(error as Error).message}</div>;
  if (!matchResults)
    return (
      <div style={{ color: 'red' }}>Match results could not be loaded</div>
    );

  const renderMatches = (title: string, matches: SymbolMetadata[]) => (
    <>
      <h3>
        {title} ({matches.length})
      </h3>
      {matches.length > 0 && (
        <>
          <ul>
            {matches.map((match) => (
              <li key={match.slug}>
                <SymbolLabel symbol={match} />
              </li>
            ))}
          </ul>
          <br />
        </>
      )}
    </>
  );

  return (
    <div className="content">
      {renderMatches('Exact matches', matchResults.exact)}
      {renderMatches('Equivalent matches', matchResults.equivalent)}
      {renderMatches('Opcode matches', matchResults.opcode)}
    </div>
  );
}
