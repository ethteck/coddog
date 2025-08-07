import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import React, { useState } from 'react';
import {
  fetchSymbolAsm,
  fetchSymbolMetadata,
  fetchSymbolSubmatches,
} from '../../../api/symbols';
import { SymbolSubmatches } from '../../../components/SymbolSubmatches';
import { AssemblyViewer } from '../../../components/AssemblyViewer';

export const Route = createFileRoute('/symbol/$symbolSlug/submatch')({
  component: SymbolSubmatch,
});

interface SelectedRange {
  start: number | null;
  end: number | null;
}

function SymbolSubmatch() {
  const pageSize = 10;
  const windowSize = 10;

  const { symbolSlug } = Route.useParams();
  const [selectedRange, setSelectedRange] = useState<SelectedRange>({
    start: null,
    end: null,
  });
  const [pageNum, setPageNum] = React.useState(0);

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

  const start = selectedRange.start ?? 0;
  const end =
    selectedRange.end ?? (queryAsm?.asm.length ? queryAsm.asm.length - 1 : 0);

  const {
    data: submatchResults,
    isLoading,
    isError,
    error,
    isFetching,
    isPlaceholderData,
  } = useQuery({
    queryKey: ['submatch', symbolSlug, start, end, pageNum, pageSize],
    queryFn: () =>
      fetchSymbolSubmatches(
        symbolSlug,
        start,
        end,
        pageNum,
        pageSize,
        windowSize,
      ),
    placeholderData: keepPreviousData,
  });

  // biome-ignore lint/correctness/useExhaustiveDependencies: <explanation>
  React.useEffect(() => {
    setPageNum(0);
  }, [symbolSlug, selectedRange.start, selectedRange.end]);

  if (isLoading) return <div>Loading submatch results...</div>;
  if (isError)
    return <div style={{ color: 'red' }}>{(error as Error).message}</div>;
  if (!submatchResults)
    return (
      <div style={{ color: 'red' }}>Match results could not be loaded</div>
    );

  if (isLoadingMetadata) return <div>Loading query metadata...</div>;
  if (isErrorMetadata)
    return (
      <div style={{ color: 'red' }}>{(errorMetadata as Error).message}</div>
    );
  if (!querySymbol)
    return (
      <div style={{ color: 'red' }}>Query symbol data could not be loaded</div>
    );

  if (isLoadingAsm) return <div>Loading query assembly...</div>;
  if (isErrorAsm)
    return <div style={{ color: 'red' }}>{(errorAsm as Error).message}</div>;
  if (!queryAsm)
    return (
      <div style={{ color: 'red' }}>
        Query assembly data could not be loaded
      </div>
    );

  if (isFetching) return <div>Loading submatch results...</div>;

  return (
    <>
      <AssemblyViewer
        asm={queryAsm.asm}
        selectedRange={selectedRange}
        setSelectedRange={setSelectedRange}
      />
      <h3>Submatches ({submatchResults?.total_count || 0})</h3>
      <button
        type="button"
        onClick={() => setPageNum((old) => Math.max(old - 1, 0))}
        disabled={pageNum === 0}
      >
        Previous Page
      </button>
      <button
        type="button"
        onClick={() => {
          if (
            !isPlaceholderData &&
            resultsHasMore(submatchResults?.total_count, pageNum, pageSize)
          ) {
            setPageNum((old) => old + 1);
          }
        }}
        // Disable the Next Page button until we know a next page is available
        disabled={
          isPlaceholderData ||
          !resultsHasMore(submatchResults?.total_count, pageNum, pageSize)
        }
      >
        Next Page
      </button>
      {pageNum + 1} / {Math.floor(submatchResults.total_count / pageSize) + 1}
      <SymbolSubmatches
        querySym={querySymbol}
        submatches={submatchResults.submatches}
      />
    </>
  );
}

function resultsHasMore(total: number, pageNum: number, pageSize: number) {
  const start = pageNum * pageSize;
  const end = start + pageSize;
  return total > end;
}
