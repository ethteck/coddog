export type SymbolMetadata = {
  id: string;
  name: string;
  source_id: number;
  source_name: string;
  project_id: number;
  project_name: string;
};

export type SymbolMatchResult = {
  exact: SymbolMetadata[];
  equivalent: SymbolMetadata[];
  opcode: SymbolMetadata[];
};

export const fetchSymbols = async (
  name: string,
): Promise<Array<SymbolMetadata>> => {
  const res = await fetch('http://localhost:3000/symbols', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name: name }),
  });
  if (!res.ok) throw new Error('Network response was not ok');
  return res.json();
};

export const fetchSymbolMatches = async (
  id: string,
): Promise<SymbolMatchResult> => {
  const res = await fetch(`http://localhost:3000/symbols/${id}`);
  if (!res.ok) throw new Error('Network response was not ok');
  return res.json();
};
