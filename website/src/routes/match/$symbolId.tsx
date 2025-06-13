import { useQuery } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import { fetchSymbolMatches } from '../../api/symbols.tsx';

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
  return (
    <>
      <div className="content">
        <h2>Match results</h2>

        {isLoading && <div>Loading...</div>}
        {isError && (
          <div style={{ color: 'red' }}>{(error as Error).message}</div>
        )}

        {matchResults?.exact.length > 0 && (
          <>
            <h3>Exact matches</h3>
            <ul>
              {matchResults.exact.map((match) => (
                <li key={match.id}>
                  <b>{match.name}</b> - {match.project_name} (
                  {match.source_name})
                </li>
              ))}
            </ul>
            <br />
          </>
        )}

        {matchResults?.equivalent.length > 0 && (
          <>
            <h3>Equivalent matches</h3>
            <ul>
              {matchResults.equivalent.map((match) => (
                <li key={match.id}>
                  <b>{match.name}</b> - {match.project_name} (
                  {match.source_name})
                </li>
              ))}
            </ul>
          </>
        )}

        {matchResults?.opcode.length > 0 && (
          <>
            <h3>Opcode matches</h3>
            <ul>
              {matchResults.opcode.map((match) => (
                <li key={match.id}>
                  <b>{match.name}</b> - {match.project_name} (
                  {match.source_name})
                </li>
              ))}
            </ul>
          </>
        )}
      </div>
    </>
  );
}
