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
import styles from './submatch.module.css';

// Define search schema with proper types and defaults
const searchSchema = z.object({
  start: z.number().optional(),
  end: z.number().optional(),
  windowSize: z.number().default(10),
  page: z.number().default(0),
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
  } = useQuery({
    queryKey: [
      'symbol_submatches',
      symbolSlug,
      search.start ?? 0,
      apiEnd,
      search.windowSize ?? DEFAULT_WINDOW_SIZE,
      search.page,
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
        'length',
        'desc',
      ),
    placeholderData: keepPreviousData,
    enabled: !!queryAsm, // Only fetch submatches after assembly is loaded
  });

  // Handler functions that update search params directly
  const handleRangeChange = (range: { start: number; end: number } | null) => {
    navigate({
      search: (prev) => ({
        ...prev,
        start: range?.start,
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
      resetScroll: false,
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
  if (isLoadingMetadata)
    return <div className={styles.loading}>Loading query metadata...</div>;
  if (isLoadingAsm)
    return <div className={styles.loading}>Loading query assembly...</div>;

  // Error states
  if (isErrorMetadata) {
    return (
      <div className={styles.error}>{(errorMetadata as Error).message}</div>
    );
  }
  if (isErrorAsm) {
    return <div className={styles.error}>{(errorAsm as Error).message}</div>;
  }
  if (isErrorSubmatches) {
    return (
      <div className={styles.error}>{(errorSubmatches as Error).message}</div>
    );
  }

  // Data validation
  if (!querySymbol) {
    return (
      <div className={styles.error}>Query symbol data could not be loaded</div>
    );
  }
  if (!queryAsm) {
    return (
      <div className={styles.error}>
        Query assembly data could not be loaded
      </div>
    );
  }

  return (
    <div className={styles.submatchPage}>
      <h2 className={styles.pageTitle}>
        Submatches for <SymbolLabel symbol={querySymbol} link={false} />
      </h2>

      <AssemblyViewer
        asm={queryAsm.asm}
        selectedRange={null}
        setSelectedRange={handleRangeChange}
      />

      <div className={styles.controlsSection}>
        <div className={styles.controlsRow}>
          <div className={styles.sliderGroup}>
            <span className={styles.sliderLabel}>Minimum match length:</span>
            <Slider
              min={MIN_WINDOW_SIZE}
              max={50}
              defaultValue={search.windowSize ?? DEFAULT_WINDOW_SIZE}
              onChange={handleWindowSizeChange}
            />
          </div>
        </div>
      </div>

      <div className={styles.resultsSection}>
        <h3 className={styles.resultsTitle}>
          Search results ({submatchResults?.total_count || 0} total)
          {isLoadingSubmatches && (
            <span className={styles.loadingIndicator}> (Loading...)</span>
          )}
        </h3>

        {submatchResults && (
          <div>
            <PageNavigation
              handlePageChange={handlePageChange}
              search={search}
              isPlaceholderData={isLoadingSubmatches}
              hasMore={hasMore}
              totalPages={totalPages}
            />
            <SymbolSubmatches
              querySym={querySymbol}
              submatches={submatchResults.submatches}
            />

            <PageNavigation
              handlePageChange={handlePageChange}
              search={search}
              isPlaceholderData={isLoadingSubmatches}
              hasMore={hasMore}
              totalPages={totalPages}
            />
          </div>
        )}
      </div>
    </div>
  );
}

function PageNavigation({
  handlePageChange,
  search,
  isPlaceholderData,
  hasMore,
  totalPages,
}: {
  handlePageChange: (newPage: number) => void;
  search: {
    windowSize: number;
    page: number;
    start?: number | undefined;
    end?: number | undefined;
  };
  isPlaceholderData: boolean;
  hasMore: boolean;
  totalPages: number;
}) {
  return (
    <div className={styles.pagination}>
      <button
        type="button"
        onClick={() => handlePageChange(Math.max(search.page - 1, 0))}
        disabled={search.page === 0}
        className={styles.paginationButton}
      >
        Previous Page
      </button>

      <span className={styles.pageInfo}>
        Page {search.page + 1} of {totalPages}
      </span>

      <button
        type="button"
        onClick={() => handlePageChange(search.page + 1)}
        disabled={isPlaceholderData || !hasMore}
        className={styles.paginationButton}
      >
        Next Page
      </button>
    </div>
  );
}
