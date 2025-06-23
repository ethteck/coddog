import { keepPreviousData, useQuery } from '@tanstack/react-query';
import {
  fetchSymbolSubmatches,
  SymbolMetadata,
  SymbolSubmatch,
} from '../api/symbols.tsx';
import { SymbolLabel } from './SymbolLabel.tsx';
import React, { useState } from 'react';

function SubmatchCard({
  submatch,
  asm,
  querySym,
}: {
  submatch: SymbolSubmatch;
  asm: string[];
  querySym: SymbolMetadata;
}) {
  const [expanded, setExpanded] = useState(false);
  const querySymLen = querySym.len;
  const matchSymLen = submatch.symbol.len;

  const queryOffsetPercent = submatch.query_start / querySymLen;
  const queryHeightPercent = submatch.len / querySymLen;
  const matchOffsetPercent = submatch.match_start / matchSymLen;
  const matchHeightPercent = submatch.len / matchSymLen;

  return (
    <div
      key={`${submatch.symbol.slug}_${submatch.query_start}_${submatch.match_start}_${submatch.len}`}
      className="submatch-card"
      style={{
        background: '#2c2f33',
        border: '1px solid #23272a',
        borderRadius: '6px',
        padding: '8px 12px',
        marginBottom: '8px',
        boxShadow: '0 1px 3px rgba(0, 0, 0, 0.2)',
        cursor: 'pointer',
      }}
      onClick={() => setExpanded(!expanded)}
    >
      <div
        style={{
          fontSize: '1rem',
          fontWeight: 'bold',
          color: '#ffb347',
          marginBottom: '4px',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
        }}
      >
        <SymbolLabel symbol={submatch.symbol} />
        <span style={{ fontSize: '0.8rem', color: '#aaa' }}>
          {expanded ? '▼ Click to collapse' : '▶ Click to expand'}
        </span>
      </div>

      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '10px',
        }}
      >
        <div style={{ display: 'flex', gap: '5px' }}>
          <svg width="30px" height="100px">
            <rect
              x="0"
              y="0"
              width="100%"
              height="100%"
              style={{
                fill: 'tan',
                stroke: 'black',
              }}
            />

            <rect
              x="5%"
              y={queryOffsetPercent * 100 + '%'}
              width="90%"
              height={queryHeightPercent * 100 + '%'}
              style={{
                fill: 'saddlebrown',
              }}
            ></rect>
          </svg>

          <svg width="30px" height="100px">
            <rect
              x="0"
              y="0"
              width="100%"
              height="100%"
              style={{
                fill: 'tan',
                stroke: 'black',
              }}
            />

            <rect
              x="5%"
              y={matchOffsetPercent * 100 + '%'}
              width="90%"
              height={matchHeightPercent * 100 + '%'}
              style={{
                fill: 'saddlebrown',
              }}
            ></rect>
          </svg>
        </div>

        <div
          style={{
            display: 'grid',
            gridTemplateColumns: '70px 1fr',
            rowGap: '2px',
            fontSize: '0.9rem',
            flexGrow: 1,
          }}
        >
          <span>Length:</span>
          <span>{submatch.len} </span>
          <span>Query:</span>
          <span>
            {submatch.query_start} - {submatch.query_start + submatch.len} (
            {queryHeightPercent.toLocaleString(undefined, {
              style: 'percent',
              maximumFractionDigits: 2,
            })}
            )
          </span>
          <span>Match:</span>
          <span>
            {submatch.match_start} - {submatch.match_start + submatch.len} (
            {matchHeightPercent.toLocaleString(undefined, {
              style: 'percent',
              maximumFractionDigits: 2,
            })}
            )
          </span>
        </div>
      </div>

      {expanded && asm.length > 0 && (
        <div
          style={{
            marginTop: '10px',
            backgroundColor: '#1e2124',
            borderRadius: '4px',
            border: '1px solid #4f545c',
            padding: '8px',
            overflow: 'auto',
            maxHeight: '300px',
          }}
        >
          <pre
            style={{
              margin: 0,
              color: '#e5e5e5',
              fontSize: '0.9rem',
              fontFamily: 'monospace',
            }}
          >
            <code>{asm.join('\n')}</code>
          </pre>
        </div>
      )}
      {expanded && asm.length === 0 && (
        <div
          style={{
            marginTop: '10px',
            color: '#aaa',
            fontStyle: 'italic',
          }}
        >
          No assembly code available.
        </div>
      )}
    </div>
  );
}

export function SymbolSubmatches({
  slug,
  querySym,
}: {
  slug: string;
  querySym: SymbolMetadata;
}) {
  const [page, setPage] = React.useState(0);

  React.useEffect(() => {
    setPage(0);
  }, [slug]);

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

  if (isLoading) return <div>Loading submatch results...</div>;
  if (isError)
    return <div style={{ color: 'red' }}>{(error as Error).message}</div>;
  if (!submatchResults)
    return (
      <div style={{ color: 'red' }}>Match results could not be loaded</div>
    );

  // Sort submatches by length in descending order
  const sortedSubmatches = [...submatchResults.submatches].sort(
    (a, b) => b.len - a.len,
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
          {sortedSubmatches.map((submatch) => {
            const asm = submatchResults.asm.get(submatch.symbol.slug) || [];
            return (
              <SubmatchCard
                key={`${submatch.symbol.slug}_${submatch.query_start}_${submatch.match_start}_${submatch.len}`}
                submatch={submatch}
                asm={asm}
                querySym={querySym}
              />
            );
          })}
        </div>
      )}
    </div>
  );
}
