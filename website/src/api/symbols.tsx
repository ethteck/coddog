export type SymbolMetadata = {
    id: string;
    name: string;
    object_id: number;
    object_name: string;
    project_id: number;
    project_name: string;
};

export type SymbolSubmatch = {
    query_start: number;
    match_start: number;
    length: number;
    symbol_id: number;
    symbol_name: string;
    object_id: number;
    object_name: string;
    project_id: number;
    project_name: string;
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
}

export const fetchSymbolsByName = async (
    symbol_name: string,
): Promise<Array<SymbolMetadata>> => {
    const res = await fetch('http://localhost:3000/symbols', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({name: symbol_name}),
    });
    if (!res.ok) throw new Error('Network response was not ok');
    return res.json();
};

export const fetchSymbolMatches = async (
    symbol_id: string,
): Promise<SymbolMatchResult> => {
    const res = await fetch(`http://localhost:3000/symbols/${symbol_id}/match`);
    if (!res.ok) throw new Error('Network response was not ok');
    return res.json();
};

export const fetchSymbolSubmatches = async (
    symbol_id: string,
    min_length: number = 8,
): Promise<SymbolSubmatchResult> => {
    const res = await fetch(`http://localhost:3000/symbols/${symbol_id}/submatch`, {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({id: parseInt(symbol_id), min_length: min_length}),
    });
    if (!res.ok) throw new Error('Network response was not ok');
    return res.json();
}