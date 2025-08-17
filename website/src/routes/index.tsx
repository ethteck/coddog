import { useDebouncedState } from '@tanstack/react-pacer';
import { useQuery } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import type React from 'react';
import { useId, useState } from 'react';
import { fetchSymbolsByName } from '../api/symbols.tsx';
import logoSvg from '../assets/coddoglogo.svg';
import { SymbolLabel } from '../components/SymbolLabel.tsx';

export const Route = createFileRoute('/')({
  component: Home,
});

function Home() {
  const navigate = Route.useNavigate();
  const [query, setQuery] = useState('');
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

  return (
    <div className="home-container">
      {/* Hero Section */}
      <section className="hero">
        <img src={logoSvg} alt="coddog" className="hero-logo" />
        <p className="tagline">The dog that sniffs for cod</p>
        <p className="hero-description">
          Reduce redundant work in decompilation by finding similar functions,
          identifying library code, and discovering partial matches within
          binaries.
        </p>

        {/* Quick Symbol Search */}
        <div className="search-section">
          <h3>🔍 Quick Symbol Search</h3>
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
                <p className="results-count">{symbols.length} symbols found:</p>
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
                          navigate({ to: '/search', search: { name: query } })
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
      </section>
    </div>
  );
}
