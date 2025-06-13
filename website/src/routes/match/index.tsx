import { useDebouncedState } from '@tanstack/react-pacer';
import { useQuery } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import { useState } from 'react';
import { fetchSymbols } from '../../api/symbols.tsx';

export const Route = createFileRoute('/match/')({
  component: Match,
});

function Match() {
  const [query, setQuery] = useState('');
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
    queryKey: ['symbols', debouncedQuery],
    queryFn: () => fetchSymbols(debouncedQuery),
    enabled: debouncedQuery.trim().length > 0,
    staleTime: 0,
  });

  function handleQueryChange(e: React.ChangeEvent<HTMLInputElement>) {
    const newQuery = e.target.value.trim();
    setQuery(newQuery);
    setDebouncedQuery(newQuery);
  }

  return (
    <>
      <div className="content">
        <h2>Symbol Match</h2>
        <p>Find symbols that match one with the given name</p>
        <form onSubmit={(e) => e.preventDefault()}>
          <input
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
                <a href={`match/${sym.id}`}>{sym.name}</a>
              </b>{' '}
              - {sym.project_name} ({sym.source_name})
            </li>
          ))}
        </ul>
      </div>
    </>
  );
}
