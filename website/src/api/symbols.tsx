export type SymbolMetadata = {
  slug: string;
  name: string;
  source_id: number;
  source_name: string;
  version_id?: number;
  version_name?: string;
  project_id: number;
  project_name: string;
};

export type SymbolSubmatch = {
  symbol: SymbolMetadata;
  query_start: number;
  match_start: number;
  length: number;
};

export type SymbolMatchResult = {
  query: SymbolMetadata;
  exact: SymbolMetadata[];
  equivalent: SymbolMetadata[];
  opcode: SymbolMetadata[];
};

export type SymbolSubmatchResult = {
  query: SymbolMetadata;
  submatches: SymbolSubmatch[];
};

export const fetchSymbolsByName = async (
  symbol_name: string,
): Promise<Array<SymbolMetadata>> => {
  const res = await fetch('http://localhost:3000/symbols', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name: symbol_name }),
  });
  if (!res.ok) throw new Error('Network response was not ok');
  return res.json();
};

export const fetchSymbolMatches = async (
  symbol_slug: string,
): Promise<SymbolMatchResult> => {
  const res = await fetch(`http://localhost:3000/symbols/${symbol_slug}/match`);
  if (!res.ok) throw new Error('Network response was not ok');
  return res.json();
};

export const fetchSymbolSubmatches = async (
  symbol_slug: string,
  min_length: number = 8,
  page: number,
  size: number,
): Promise<SymbolSubmatchResult> => {
  const res = await fetch(
    `http://localhost:3000/symbols/${symbol_slug}/submatch`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        slug: symbol_slug,
        min_length: min_length,
        page: page,
        size: size,
      }),
    },
  );
  if (!res.ok) throw new Error('Network response was not ok');
  return res.json();
};
