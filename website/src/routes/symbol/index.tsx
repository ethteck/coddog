import { useDebouncedState } from '@tanstack/react-pacer';
import { useQuery } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import React, { useState, useId } from 'react';
import { fetchSymbolsByName } from '../../api/symbols.tsx';
import { SymbolLabel } from '../../components/SymbolLabel.tsx';

type SymbolSearch = {
  name: string;
};

export const Route = createFileRoute('/symbol/')({
  component: SymbolSearch,
  validateSearch: (search: Record<string, unknown>): SymbolSearch => {
    return {
      name: (search?.name as string) || '',
    };
  },
});

function SymbolSearch() {
  const { name } = Route.useSearch();
  const navigate = Route.useNavigate();
  const [query, setQuery] = useState(name);
  const inputId = useId();
  const [debouncedQuery, setDebouncedQuery] = useDebouncedState(query, {
    wait: 300,
    enabled: query.length > 0,
  });

  const {
    data: symbols,
    isLoading,
    isError,
    error,
  } = useQuery({
    queryKey: ['symbol_matches', debouncedQuery],
    queryFn: () => fetchSymbolsByName(debouncedQuery),
    enabled: debouncedQuery.trim().length > 0,
    staleTime: 0,
  });

  React.useEffect(() => {
    setQuery(name);
  }, [name]);

  function handleQueryChange(e: React.ChangeEvent<HTMLInputElement>) {
    const newQuery = e.target.value;
    setQuery(newQuery);
    setDebouncedQuery(newQuery);
    navigate({ search: { name: newQuery } });
  }

  return (
    <div className="content">
      <h2>Symbol lookup</h2>
      <p>Find matches and submatches for the symbol with the given name</p>
      <input
        id={inputId}
        type="text"
        placeholder="Enter symbol name"
        value={query}
        onChange={handleQueryChange}
      />
      {isLoading && <div>Loading...</div>}
      {isError && (
        <div style={{ color: 'red' }}>{(error as Error).message}</div>
      )}
      <ul>
        {symbols?.map((sym) => (
          <li key={sym.slug}>
            <SymbolLabel symbol={sym} />
          </li>
        ))}
      </ul>
    </div>
  );
}
