import { useDebouncedState } from '@tanstack/react-pacer';
import { useQuery } from '@tanstack/react-query';
import { createFileRoute, Link } from '@tanstack/react-router';
import React, { useState } from 'react';
import { fetchSymbolsByName } from '../../api/symbols.tsx';

type SymbolSearch = {
  name: string;
};

export const Route = createFileRoute('/match/')({
  component: Match,
  validateSearch: (search: Record<string, unknown>): SymbolSearch => {
    return {
      name: (search?.name as string) || '',
    };
  },
});

function Match() {
  const { name } = Route.useSearch();
  const navigate = Route.useNavigate();
  const [query, setQuery] = useState(name);
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
    <>
      <div className="content">
        <h2>Symbol Match</h2>
        <p>Find symbols that match one with the given name</p>
        <form onSubmit={(e) => e.preventDefault()}>
          <input
            id="symbolNameInput"
            type="text"
            placeholder="Enter symbol name"
            value={query}
            onChange={handleQueryChange}
          />
        </form>
        {isLoading && <div>Loading...</div>}
        {isError && (
          <div style={{ color: 'red' }}>{(error as Error).message}</div>
        )}
        <ul>
          {symbols?.map((sym) => (
            <li key={sym.id}>
              <b>
                <Link to="/match/$symbolId" params={{ symbolId: sym.id }}>
                  {sym.name}
                </Link>
              </b>{' '}
              - {sym.project_name} ({sym.object_name})
            </li>
          ))}
        </ul>
      </div>
    </>
  );
}
