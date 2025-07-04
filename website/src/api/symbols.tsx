export type SymbolMetadata = {
  slug: string;
  name: string;
  len: number;
  source_id: number;
  source_name: string;
  version_id?: number;
  version_name?: string;
  project_id: number;
  project_name: string;
  platform: number;
};

export type SymbolSubmatch = {
  symbol: SymbolMetadata;
  query_start: number;
  match_start: number;
  len: number;
};

export type SymbolMatchResult = {
  exact: SymbolMetadata[];
  equivalent: SymbolMetadata[];
  opcode: SymbolMetadata[];
};

export type SymbolSubmatchResult = {
  total_count: number;
  submatches: SymbolSubmatch[];
};

export type SymbolAsm = {
  asm: string[];
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

export const fetchSymbolMetadata = async (
  symbol_slug: string,
): Promise<SymbolMetadata> => {
  const res = await fetch(`http://localhost:3000/symbols/${symbol_slug}`);
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

export const fetchSymbolAsm = async (
  symbol_slug: string,
): Promise<SymbolAsm> => {
  const res = await fetch(`http://localhost:3000/symbols/${symbol_slug}/asm`);
  if (!res.ok) throw new Error('Network response was not ok');
  return await res.json();
};

export const fetchSymbolSubmatches = async (
  symbol_slug: string,
  window_size: number = 8,
  page: number,
  size: number,
): Promise<SymbolSubmatchResult> => {
  const res = await fetch(
    `http://localhost:3000/symbols/${symbol_slug}/submatch`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        window_size: window_size,
        page_num: page,
        page_size: size,
      }),
    },
  );
  if (!res.ok) throw new Error('Network response was not ok');
  return await res.json();
};
