import {useQuery} from '@tanstack/react-query';
import {createFileRoute} from '@tanstack/react-router';
import {fetchSymbolMatches, SymbolMetadata} from '../../api/symbols.tsx';

export const Route = createFileRoute('/match/$symbolId')({
    component: SymbolMatches,
});

function SymbolMatches() {
    const {symbolId} = Route.useParams();

    const {
        data: matchResults,
        isLoading,
        isError,
        error,
    } = useQuery({
        queryKey: ['match', symbolId],
        queryFn: () => fetchSymbolMatches(symbolId),
    });

    if (isLoading) return <div>Loading...</div>;
    if (isError) return <div style={{color: 'red'}}>{(error as Error).message}</div>;
    if (!matchResults) return <div style={{color: 'red'}}>Match results could not be loaded</div>;

    const renderMatches = (title: string, matches: SymbolMetadata[]) => (
        <>
            <h3>{title} ({matches.length})</h3>
            {matches.length > 0 && (
                <>
                    <ul>
                        {matches.map((match) => (
                            <li key={match.id}>
                                <b>{match.name}</b> - {match.project_name} ({match.source_name})
                            </li>
                        ))}
                    </ul>
                    <br/>
                </>
            )}
        </>
    );

    return (
        <div className="content">
            <h2>Match results</h2>
            <p><b>Query: </b>some_func134 - Some Game (Platform)</p>
            {renderMatches('Exact matches', matchResults.exact)}
            {renderMatches('Equivalent matches', matchResults.equivalent)}
            {renderMatches('Opcode matches', matchResults.opcode)}
        </div>
    );
}
