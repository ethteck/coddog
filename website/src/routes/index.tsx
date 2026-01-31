import Uploady, {
  useItemErrorListener,
  useItemFinishListener,
  useItemProgressListener,
  useItemStartListener,
  useUploady,
} from '@rpldy/uploady';
import { useDebouncedState } from '@tanstack/react-pacer';
import { useQuery } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import type React from 'react';
import { useCallback, useEffect, useId, useRef, useState } from 'react';
import { DndProvider, useDrop } from 'react-dnd';
import { HTML5Backend, NativeTypes } from 'react-dnd-html5-backend';
import { API_BASE_URL } from '../api/config.ts';
import { fetchSymbolsByName } from '../api/symbols.tsx';
import logoSvg from '../assets/coddoglogo.svg';
import { SymbolLabel } from '../components/SymbolLabel.tsx';

export const Route = createFileRoute('/')({
  component: Home,
});

type UploadState = {
  isUploading: boolean;
  error: string | null;
};

const DropZone = ({
  uploadState,
  setUploadState,
  onNavigate,
}: {
  uploadState: UploadState;
  setUploadState: (state: UploadState) => void;
  onNavigate: (slug: string) => void;
}) => {
  const { upload } = useUploady();
  const timeoutRef = useRef<NodeJS.Timeout | null>(null);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  useItemStartListener((item) => {
    console.log('Upload started:', item);
    setUploadState({ isUploading: true, error: null });

    // Set a timeout as a fallback in case listeners don't fire
    timeoutRef.current = setTimeout(() => {
      console.log('Upload timeout - assuming failure');
      setUploadState({ isUploading: false, error: 'Upload timed out' });
    }, 30000); // 30 second timeout
  });

  useItemProgressListener((item) => {
    console.log('Upload progress:', item.completed, '/', item.total);
  });

  // Handle upload errors - this fires for network errors, aborted uploads, etc.
  useItemErrorListener((item) => {
    console.log('Upload error listener triggered:', item);
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    setUploadState({
      isUploading: false,
      error: item.uploadResponse?.data || 'Upload failed due to an error',
    });
  });

  // Handle both success and error cases when upload completes
  useItemFinishListener((item) => {
    console.log('Upload finished listener triggered:', item);
    console.log('Upload status:', item.uploadStatus);
    console.log('Upload response:', item.uploadResponse);

    // Clear timeout since we got a response
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }

    if (item.uploadStatus >= 400) {
      console.log('Upload failed with status:', item.uploadStatus);
      setUploadState({
        isUploading: false,
        error: `Upload failed: ${item.uploadStatus}`,
      });
      return;
    }

    // Handle success case
    if (item.uploadResponse?.data) {
      try {
        const response = JSON.parse(item.uploadResponse.data);
        console.log('Parsed response:', response);

        if (response.slug) {
          // Success - reset state and navigate
          setUploadState({ isUploading: false, error: null });
          onNavigate(response.slug);
        } else if (response.success === false) {
          // Server returned success status but indicated failure in response
          setUploadState({
            isUploading: false,
            error: response.message || 'Upload failed',
          });
        } else {
          // Unexpected response format
          console.log('Unexpected response format:', response);
          setUploadState({
            isUploading: false,
            error: 'Unexpected server response',
          });
        }
      } catch (e) {
        console.error('Failed to parse upload response:', e);
        setUploadState({
          isUploading: false,
          error: 'Failed to parse server response',
        });
      }
    } else {
      // No response data
      console.log('No response data received');
      setUploadState({
        isUploading: false,
        error: 'No response received from server',
      });
    }
  });

  const [{ isDragging }, dropRef] = useDrop({
    accept: NativeTypes.FILE,
    collect: (monitor) => ({
      isDragging: !!monitor.isOver(),
    }),
    drop: (item) => {
      if (!uploadState.isUploading) {
        upload(item.files);
      }
    },
  });

  if (uploadState.isUploading) {
    return (
      <div className="drop-zone uploading">
        <p>Uploading and analyzing file...</p>
        <div className="upload-progress">Please wait</div>
      </div>
    );
  }

  return (
    <div
      ref={dropRef}
      className={`drop-zone ${isDragging ? 'drag-over' : ''} ${uploadState.error ? 'error' : ''}`}
    >
      {uploadState.error ? (
        <>
          <p className="error-message">{uploadState.error}</p>
          <p>Drag & drop another file to try again</p>
        </>
      ) : (
        <p>Drag & drop an object file here to upload it for analysis</p>
      )}
    </div>
  );
};

function Home() {
  const navigate = Route.useNavigate();
  const [query, setQuery] = useState('');
  const [uploadState, setUploadState] = useState<UploadState>({
    isUploading: false,
    error: null,
  });
  const inputId = useId();
  const [debouncedQuery, setDebouncedQuery] = useDebouncedState(query, {
    wait: 300,
    enabled: query.length > 0,
  });

  const {
    data: symbols,
    isLoading,
    isError,
  } = useQuery({
    queryKey: ['symbol_matches', debouncedQuery],
    queryFn: () => fetchSymbolsByName(debouncedQuery),
    enabled: debouncedQuery.trim().length > 0,
    staleTime: 0,
  });

  function handleQueryChange(e: React.ChangeEvent<HTMLInputElement>) {
    const newQuery = e.target.value;
    setQuery(newQuery);
    setDebouncedQuery(newQuery);
  }

  const handleUploadNavigation = useCallback(
    (slug: string) => {
      navigate({
        to: '/source/$sourceSlug',
        params: { sourceSlug: slug },
      });
    },
    [navigate],
  );

  const filterBySize = useCallback((file) => {
    return file.size < 1024 * 1024 * 5;
  }, []);

  return (
    <DndProvider backend={HTML5Backend}>
      <Uploady
        fileFilter={filterBySize}
        destination={{ url: `${API_BASE_URL}/upload` }}
      >
        <div className="home-container">
          {/* Hero Section */}
          <section className="hero">
            <img src={logoSvg} alt="coddog" className="hero-logo" />
            <p className="tagline">The dog that sniffs for cod</p>
            <p className="hero-description">
              Reduce redundant work in decompilation by finding similar
              functions, identifying library code, and discovering partial
              matches within binaries.
            </p>

            {/* Quick Symbol Search */}
            <div className="search-section">
              <h3>Quick Search</h3>
              <div className="search-container">
                <input
                  id={inputId}
                  type="text"
                  placeholder="Enter symbol name (e.g., main, printf, func_80123456)"
                  value={query}
                  onChange={handleQueryChange}
                  className="search-input"
                />
                {isLoading && <div className="search-status">Searching...</div>}
                {isError && (
                  <div className="search-status error">Search failed</div>
                )}
                {symbols && symbols.length > 0 && (
                  <div className="search-results">
                    <p className="results-count">
                      {symbols.length} symbols found:
                    </p>
                    <ul className="results-list">
                      {symbols.slice(0, 5).map((sym) => (
                        <li key={sym.slug}>
                          <SymbolLabel symbol={sym} className="symbol-button" />
                        </li>
                      ))}
                      {symbols.length > 5 && (
                        <li className="more-results">
                          <button
                            type="button"
                            className="button"
                            onClick={() =>
                              navigate({
                                to: '/search',
                                search: { name: query },
                              })
                            }
                          >
                            View all {symbols.length} results →
                          </button>
                        </li>
                      )}
                    </ul>
                  </div>
                )}
              </div>
            </div>
            <div className="upload-section">
              <h3>Upload Object for Analysis</h3>
              <DropZone
                uploadState={uploadState}
                setUploadState={setUploadState}
                onNavigate={handleUploadNavigation}
              />
            </div>
          </section>
        </div>
      </Uploady>
    </DndProvider>
  );
}
