import { useQuery } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import { fetchSymbolMatches, SymbolMetadata } from '../../api/symbols.tsx';
import { SymbolLabel } from '../../components/SymbolLabel.tsx';

export const Route = createFileRoute('/match/$symbolId')({
  component: SymbolMatches,
});

function SymbolMatches() {
  const { symbolId } = Route.useParams();

  const {
    data: matchResults,
    isLoading,
    isError,
    error,
  } = useQuery({
    queryKey: ['match', symbolId],
    queryFn: () => fetchSymbolMatches(symbolId),
  });

  if (isLoading) return <div>Loading...</div>;
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
              <li key={match.id}>
                <b>{match.name}</b> - {match.project_name} ({match.object_name})
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
      <h2>Match results</h2>
      <p>
        <b>Query: </b>{' '}
        <SymbolLabel
          name={matchResults.query.name}
          project_name={matchResults.query.project_name}
          object_name={matchResults.query.object_name}
        />
      </p>
      {renderMatches('Exact matches', matchResults.exact)}
      {renderMatches('Equivalent matches', matchResults.equivalent)}
      {renderMatches('Opcode matches', matchResults.opcode)}
    </div>
  );
}
