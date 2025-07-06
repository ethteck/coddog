import { useQuery } from '@tanstack/react-query';
import { fetchSymbolMatches } from '../api/symbols.tsx';
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

  return (
    <div className="content">
      <h3>Matches ({matchResults.length})</h3>
      {matchResults.length > 0 &&
        matchResults.map((match) => (
          <>
            <div
              key={`${match.symbol.slug}_${match.subtype}`}
              className="match-card"
              style={{
                background: '#2c2f33',
                border: '1px solid #23272a',
                borderRadius: '6px',
                padding: '8px 12px',
                marginBottom: '8px',
                boxShadow: '0 1px 3px rgba(0, 0, 0, 0.2)',
              }}
            >
              <div
                style={{
                  fontSize: '1rem',
                  fontWeight: 'bold',
                  color: '#ffb347',
                  display: 'flex',
                  alignItems: 'center',
                }}
              >
                <span
                  style={{
                    backgroundColor:
                      match.subtype === 'exact'
                        ? '#4caf50'
                        : match.subtype === 'equivalent'
                          ? '#ff9800'
                          : '#2196f3',
                    color: 'white',
                    padding: '2px 8px',
                    marginRight: '8px',
                    borderRadius: '3px',
                    fontSize: '0.75rem',
                    fontWeight: 'normal',
                    textTransform: 'uppercase',
                    letterSpacing: '0.5px',
                    userSelect: 'none',
                  }}
                >
                  {match.subtype}
                </span>
                <SymbolLabel symbol={match.symbol} />
              </div>
            </div>
          </>
        ))}
    </div>
  );
}
