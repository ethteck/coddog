import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import { zodValidator } from '@tanstack/zod-adapter';
import { z } from 'zod';
import {
  fetchSymbolAsm,
  fetchSymbolMetadata,
  fetchSymbolSubmatches,
} from '../../../api/symbols';
import { AssemblyViewer } from '../../../components/AssemblyViewer';
import Slider from '../../../components/Slider';
import { SymbolLabel } from '../../../components/SymbolLabel';
import { SymbolSubmatches } from '../../../components/SymbolSubmatches';

type SortBy = 'length' | 'query_start';
type SortDir = 'asc' | 'desc';

// Define search schema with proper types and defaults
const searchSchema = z.object({
  start: z.number().optional(),
  end: z.number().optional(),
  windowSize: z.number().default(10),
  page: z.number().default(0),
  sortBy: z.enum(['length', 'query_start']).default('length'),
  sortDir: z.enum(['asc', 'desc']).default('desc'),
});

// Constants
const PAGE_SIZE = 10;
const DEFAULT_WINDOW_SIZE = 10;
const MIN_WINDOW_SIZE = 8;

export const Route = createFileRoute('/symbol/$symbolSlug/submatch')({
  component: SymbolSubmatch,
  validateSearch: zodValidator(searchSchema),
});

function SymbolSubmatch() {
  // Get params and search from route
  const { symbolSlug } = Route.useParams();
  const search = Route.useSearch();
  const navigate = Route.useNavigate();

  // Fetch assembly query early to get the max length
  const {
    data: queryAsm,
    isLoading: isLoadingAsm,
    isError: isErrorAsm,
    error: errorAsm,
  } = useQuery({
    queryKey: ['asm', symbolSlug],
    queryFn: () => fetchSymbolAsm(symbolSlug),
    staleTime: 5 * 60 * 1000,
  });

  // Calculate actual end value for API calls
  const maxEnd = queryAsm?.asm.length ? queryAsm.asm.length - 1 : 0;

  // Fetch metadata query
  const {
    data: querySymbol,
    isLoading: isLoadingMetadata,
    isError: isErrorMetadata,
    error: errorMetadata,
  } = useQuery({
    queryKey: ['metadata', symbolSlug],
    queryFn: () => fetchSymbolMetadata(symbolSlug),
    staleTime: 5 * 60 * 1000, // Consider metadata fresh for 5 minutes
  });

  // Calculate the actual end value for API calls (use maxEnd if end is undefined)
  const apiEnd = search.end ?? maxEnd;

  // Fetch submatches with current search values
  const {
    data: submatchResults,
    isLoading: isLoadingSubmatches,
    isError: isErrorSubmatches,
    error: errorSubmatches,
    isPlaceholderData,
  } = useQuery({
    queryKey: [
      'symbol_submatches',
      symbolSlug,
      search.start ?? 0,
      apiEnd,
      search.windowSize ?? DEFAULT_WINDOW_SIZE,
      search.page,
      search.sortBy,
      search.sortDir,
      PAGE_SIZE,
    ],
    queryFn: () =>
      fetchSymbolSubmatches(
        symbolSlug,
        search.start ?? 0,
        apiEnd,
        search.page,
        PAGE_SIZE,
        search.windowSize ?? DEFAULT_WINDOW_SIZE,
        search.sortBy,
        search.sortDir,
      ),
    placeholderData: keepPreviousData,
    enabled: !!queryAsm, // Only fetch submatches after assembly is loaded
  });

  // Handler functions that update search params directly
  const handleRangeChange = (range: { start: number; end: number } | null) => {
    navigate({
      search: (prev) => ({
        ...prev,
        start: range?.start ?? 0,
        end: range?.end,
        page: 0, // Reset page when changing range
      }),
      replace: true,
    });
  };

  const handleWindowSizeChange = (value: number) => {
    navigate({
      search: (prev) => ({
        ...prev,
        windowSize: value,
        page: 0, // Reset page when changing window size
      }),
      replace: true,
    });
  };

  const handlePageChange = (newPage: number) => {
    navigate({
      search: (prev) => ({ ...prev, page: newPage }),
    });
  };

  const handleSortChange = (value: SortBy) => {
    navigate({
      search: (prev) => ({ ...prev, sortBy: value }),
    });
  };

  const handleSortDirectionChange = (value: SortDir) => {
    navigate({
      search: (prev) => ({ ...prev, sortDir: value }),
    });
  };

  // Calculate pagination info
  const totalPages = submatchResults
    ? Math.ceil(submatchResults.total_count / PAGE_SIZE)
    : 0;

  const hasMore = submatchResults
    ? (search.page + 1) * PAGE_SIZE < submatchResults.total_count
    : false;

  // Loading states
  if (isLoadingMetadata) return <div>Loading query metadata...</div>;
  if (isLoadingAsm) return <div>Loading query assembly...</div>;

  // Error states
  if (isErrorMetadata) {
    return (
      <div style={{ color: 'red' }}>{(errorMetadata as Error).message}</div>
    );
  }
  if (isErrorAsm) {
    return <div style={{ color: 'red' }}>{(errorAsm as Error).message}</div>;
  }
  if (isErrorSubmatches) {
    return (
      <div style={{ color: 'red' }}>{(errorSubmatches as Error).message}</div>
    );
  }

  // Data validation
  if (!querySymbol) {
    return (
      <div style={{ color: 'red' }}>Query symbol data could not be loaded</div>
    );
  }
  if (!queryAsm) {
    return (
      <div style={{ color: 'red' }}>
        Query assembly data could not be loaded
      </div>
    );
  }

  return (
    <>
      <h2>
        Submatches for <SymbolLabel symbol={querySymbol} link={false} />
      </h2>

      <AssemblyViewer
        asm={queryAsm.asm}
        selectedRange={null}
        setSelectedRange={handleRangeChange}
      />

      <div
        style={{
          marginBottom: '10px',
          display: 'flex',
          gap: '10px',
          alignItems: 'center',
          flexWrap: 'wrap',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '20px' }}>
          <span>Minimum match length:</span>
          <Slider
            min={MIN_WINDOW_SIZE}
            max={50}
            defaultValue={search.windowSize ?? DEFAULT_WINDOW_SIZE}
            onChange={handleWindowSizeChange}
          />
        </div>
      </div>

      <div
        style={{
          display: 'flex',
          gap: '10px',
          alignItems: 'center',
          flexWrap: 'wrap',
        }}
      >
        <span>Sort by:</span>
        <select
          value={search.sortBy}
          onChange={(e) => handleSortChange(e.target.value as SortBy)}
        >
          <option value="length">Length</option>
          <option value="query_start">Query start</option>
        </select>
        <span>Sort direction:</span>
        <select
          value={search.sortDir}
          onChange={(e) => handleSortDirectionChange(e.target.value as SortDir)}
        >
          <option value="asc">Ascending</option>
          <option value="desc">Descending</option>
        </select>
      </div>

      <br />

      <h3>
        Search results ({submatchResults?.total_count || 0} total)
        {isLoadingSubmatches && <span> (Loading...)</span>}
      </h3>

      <div style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
        <button
          type="button"
          onClick={() => handlePageChange(Math.max(search.page - 1, 0))}
          disabled={search.page === 0}
        >
          Previous Page
        </button>

        <button
          type="button"
          onClick={() => handlePageChange(search.page + 1)}
          disabled={isPlaceholderData || !hasMore}
        >
          Next Page
        </button>

        <span>
          Page {search.page + 1} of {totalPages}
        </span>
      </div>

      {submatchResults && (
        <SymbolSubmatches
          querySym={querySymbol}
          submatches={submatchResults.submatches}
        />
      )}
    </>
  );
}
