import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { fetchSymbolSubmatches } from '../api/symbols.tsx';
import { SymbolLabel } from './SymbolLabel.tsx';
import React from 'react';

export function SymbolSubmatches({ slug }: { slug: string }) {
  const [page, setPage] = React.useState(0);

  var pageSize = 10;
  var minLength = 10;

  const {
    data: submatchResults,
    isLoading,
    isError,
    error,
    isFetching,
    isPlaceholderData,
  } = useQuery({
    queryKey: ['match', slug, page, pageSize],
    queryFn: () => fetchSymbolSubmatches(slug, minLength, page, pageSize),
    placeholderData: keepPreviousData,
  });

  if (isLoading) return <div>Loading...</div>;
  if (isError)
    return <div style={{ color: 'red' }}>{(error as Error).message}</div>;
  if (!submatchResults)
    return (
      <div style={{ color: 'red' }}>Match results could not be loaded</div>
    );

  // Sort submatches by length in descending order
  const sortedSubmatches = [...submatchResults.submatches].sort(
    (a, b) => b.length - a.length,
  );

  return (
    <div className="content">
      <h3>
        Submatches
        <span> (page {page + 1}/???) </span>
      </h3>

      <button
        onClick={() => setPage((old) => Math.max(old - 1, 0))}
        disabled={page === 0}
      >
        Previous Page
      </button>

      <button
        onClick={() => {
          // if (!isPlaceholderData && submatchResults.hasMore) {
          if (!isPlaceholderData) {
            setPage((old) => old + 1);
          }
        }}
        // Disable the Next Page button until we know a next page is available
        //disabled={isPlaceholderData || !submatchResults?.hasMore}
        disabled={isPlaceholderData}
      >
        Next Page
      </button>

      {isFetching ? <span> Loading...</span> : null}

      {sortedSubmatches.length === 0 ? (
        <p>No submatches found.</p>
      ) : (
        <div className="submatch-list">
          {sortedSubmatches.map((submatch) => (
            <div
              key={`${submatch.symbol.slug}_${submatch.query_start}_${submatch.match_start}_${submatch.length}`}
              className="submatch-card"
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
                  marginBottom: '4px',
                }}
              >
                <SymbolLabel symbol={submatch.symbol} />
              </div>
              <div
                style={{
                  display: 'grid',
                  gridTemplateColumns: '100px 1fr',
                  rowGap: '2px',
                  fontSize: '0.9rem',
                }}
              >
                <span>Length:</span> <span>{submatch.length}</span>
                <span>Query:</span>
                <span>
                  {submatch.query_start} -{' '}
                  {submatch.query_start + submatch.length}
                </span>
                <span>Target:</span>
                <span>
                  {submatch.match_start} -{' '}
                  {submatch.match_start + submatch.length}
                </span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
