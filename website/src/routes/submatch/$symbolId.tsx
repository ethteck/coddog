import {useQuery} from '@tanstack/react-query';
import {createFileRoute} from '@tanstack/react-router';
import {fetchSymbolSubmatches} from '../../api/symbols.tsx';
import {SymbolLabel} from "../../components/SymbolLabel.tsx";

export const Route = createFileRoute('/submatch/$symbolId')({
    component: SymbolSubmatches,
});

function SymbolSubmatches() {
    const {symbolId} = Route.useParams();

    const {
        data: submatchResults,
        isLoading,
        isError,
        error,
    } = useQuery({
        queryKey: ['match', symbolId],
        queryFn: () => fetchSymbolSubmatches(symbolId, 10),
    });

    if (isLoading) return <div>Loading...</div>;
    if (isError)
        return <div style={{color: 'red'}}>{(error as Error).message}</div>;
    if (!submatchResults)
        return (
            <div style={{color: 'red'}}>Match results could not be loaded</div>
        );

    // Sort submatches by length in descending order
    const sortedSubmatches = [...submatchResults.submatches].sort(
        (a, b) => b.length - a.length
    ).slice(0, 10);

    return (
        <div className="content">
            <h2>Match results</h2>
            <p>
                <b>Query: </b> <SymbolLabel name={submatchResults.query.name}
                                            project_name={submatchResults.query.project_name}
                                            object_name={submatchResults.query.object_name}/>
            </p>

            <h3>Submatches
                ({Math.min(submatchResults.submatches.length, 10)} of {submatchResults.submatches.length})</h3>
            {sortedSubmatches.length === 0 ? (
                <p>No submatches found.</p>
            ) : (
                <div className="submatch-list">
                    {sortedSubmatches.map((submatch) => (
                        <div key={submatch.symbol_id} className="submatch-card" style={{
                            background: '#2c2f33',
                            border: '1px solid #23272a',
                            borderRadius: '6px',
                            padding: '8px 12px',
                            marginBottom: '8px',
                            boxShadow: '0 1px 3px rgba(0, 0, 0, 0.2)'
                        }}>
                            <div style={{fontSize: '1rem', fontWeight: 'bold', color: '#ffb347', marginBottom: '4px'}}>
                                <SymbolLabel name={submatch.symbol_name} project_name={submatch.project_name}
                                             object_name={submatch.object_name}/>
                            </div>
                            <div style={{
                                display: 'grid',
                                gridTemplateColumns: '100px 1fr',
                                rowGap: '2px',
                                fontSize: '0.9rem'
                            }}>
                                <span>Length:</span> <span>{submatch.length}</span>
                                <span>Query:</span>
                                <span>{submatch.query_start} - {submatch.query_start + submatch.length}</span>
                                <span>Target:</span>
                                <span>{submatch.match_start} - {submatch.match_start + submatch.length}</span>
                            </div>
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}
