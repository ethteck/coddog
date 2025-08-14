import { useDebouncedState } from '@tanstack/react-pacer';
import { useQuery } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import type React from 'react';
import { useId, useState } from 'react';
import { fetchSymbolsByName } from '../api/symbols.tsx';
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

  function handleSymbolSelect(symbolName: string) {
    navigate({ to: '/symbol', search: { name: symbolName } });
  }

  return (
    <div className="home-container">
      {/* Hero Section */}
      <section className="hero">
        <h1>🐕 coddog</h1>
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
                      <button
                        type="button"
                        onClick={() => handleSymbolSelect(sym.name)}
                        className="symbol-button"
                      >
                        <SymbolLabel symbol={sym} />
                      </button>
                    </li>
                  ))}
                  {symbols.length > 5 && (
                    <li className="more-results">
                      <button
                        type="button"
                        className="button"
                        onClick={() =>
                          navigate({ to: '/symbol', search: { name: query } })
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

      {/* Problem & Solution */}
      <section className="problem-solution">
        <div className="problem">
          <h2>🧩 The Decompilation Challenge</h2>
          <p>
            When decompiling binaries, you often encounter similar or identical
            functions across different projects. Sometimes you can only match{' '}
            <em>parts</em> of functions, making it hard to identify library code
            or find patterns.
          </p>
          <ul>
            <li>❌ Redundant reverse engineering work</li>
            <li>❌ Partial function matches go unnoticed</li>
            <li>❌ Library functions get re-analyzed repeatedly</li>
            <li>❌ Code patterns are hard to identify</li>
          </ul>
        </div>

        <div className="solution">
          <h2>✨ Meet coddog</h2>
          <p>
            coddog solves these problems by providing intelligent function
            matching and sub-function search capabilities, helping you work more
            efficiently.
          </p>
          <ul>
            <li>✅ Find similar functions across binaries</li>
            <li>✅ Discover partial matches within functions</li>
            <li>✅ Identify and de-duplicate library code</li>
            <li>✅ Analyze code patterns and clusters</li>
          </ul>
        </div>
      </section>

      {/* Features */}
      <section className="features">
        <h2>🛠️ Core Features</h2>

        <div className="feature-grid">
          <div className="feature">
            <h3>🎯 Function Matching</h3>
            <p>Find functions similar to your query with confidence scores</p>
            <div className="code-example">
              <code>coddog match func_80348C08 -t 0.7</code>
              <div className="code-output">
                100.00% - func_802ECC44 (decompiled)
                <br />
                73.33% - finishLevel (decompiled)
                <br />
                71.88% - osAiSetFrequency (decompiled)
              </div>
            </div>
          </div>

          <div className="feature">
            <h3>🔗 Function Clustering</h3>
            <p>Group identical or near-identical functions for deduplication</p>
            <div className="code-example">
              <code>coddog cluster -m 10</code>
              <div className="code-output">
                Cluster func_802C8998 has 23 symbols
                <br />
                Cluster func_802E1110 has 12 symbols
                <br />
                Cluster func_802C2D00 has 10 symbols
              </div>
            </div>
          </div>

          <div className="feature">
            <h3>🔍 Partial Matching</h3>
            <p>Find code segments that match within larger functions</p>
            <div className="code-example">
              <code>coddog submatch finishLevel 30</code>
              <div className="code-output">
                func_credits_801DE060:
                <br />
                &nbsp;&nbsp;query [41-77] matches [101-137] (36 total)
                <br />
                updateIdle:
                <br />
                &nbsp;&nbsp;query [23-89] matches [107-173] (66 total)
              </div>
            </div>
          </div>

          <div className="feature">
            <h3>⚖️ Cross-Binary Comparison</h3>
            <p>Compare functions between different binaries and projects</p>
            <div className="code-example">
              <code>coddog compare2 proj1.yaml proj2.yaml</code>
              <div className="code-output">
                alMainBusPull - alMainBusPull (98.61%)
                <br />
                __ll_div - __ll_div (100.00%)
                <br />
                Vec3fDiff - func_8000E958 (100.00%)
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Getting Started */}
      <section className="getting-started">
        <h2>🚀 Getting Started</h2>
        <div className="steps">
          <div className="step">
            <div className="step-number">1</div>
            <div className="step-content">
              <h4>Search for Symbols</h4>
              <p>
                Use the search above or visit the{' '}
                <a href="/symbol">Symbol page</a> to find functions by name
              </p>
            </div>
          </div>
          <div className="step">
            <div className="step-number">2</div>
            <div className="step-content">
              <h4>Explore Matches</h4>
              <p>
                View similar functions and their confidence scores to identify
                duplicates
              </p>
            </div>
          </div>
          <div className="step">
            <div className="step-number">3</div>
            <div className="step-content">
              <h4>Analyze Submatches</h4>
              <p>
                Find partial matches within functions to understand code
                patterns
              </p>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
