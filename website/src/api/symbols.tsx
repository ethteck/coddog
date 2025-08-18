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
  project_repo?: string;
  platform: number;
};

export type SymbolMatchResult = {
  subtype: 'exact' | 'equivalent' | 'opcode';
  symbol: SymbolMetadata;
};

export type SymbolSubmatchResult = {
  symbol: SymbolMetadata;
  query_start: number;
  match_start: number;
  len: number;
};

export type SymbolSubmatchResults = {
  total_count: number;
  submatches: SymbolSubmatchResult[];
};

export type AsmInsn = {
  opcode: string;
  address?: string;
  arguments: string[];
  branch_dest?: string;
  symbol?: string;
  addend?: string;
};

export type SymbolAsm = {
  asm: AsmInsn[];
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
): Promise<SymbolMatchResult[]> => {
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
  start: number,
  end: number,
  page: number,
  size: number,
  window_size: number,
  sort_by = 'length',
  sort_dir = 'desc',
): Promise<SymbolSubmatchResults> => {
  const res = await fetch(
    `http://localhost:3000/symbols/${symbol_slug}/submatch`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        window_size: window_size,
        start: start,
        end: end,
        page_num: page,
        page_size: size,
        sort_by: sort_by,
        sort_dir: sort_dir,
      }),
    },
  );
  if (!res.ok) throw new Error('Network response was not ok');
  return await res.json();
};
