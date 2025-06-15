import { createFileRoute } from '@tanstack/react-router';
import { SymbolSubmatches } from '../../components/SymbolSubmatches.tsx';
import { SymbolMatches } from '../../components/SymbolMatches.tsx';

export const Route = createFileRoute('/symbol/$symbolSlug')({
  component: SymbolInfo,
});

function SymbolInfo() {
  const { symbolSlug } = Route.useParams();

  return (
    <>
      <h2>Match results</h2>
      {/*<p>*/}
      {/*    <b>Query: </b> <SymbolLabel symbol={matchResults.query}/>*/}
      {/*</p>*/}
      <SymbolMatches slug={symbolSlug} />
      <SymbolSubmatches slug={symbolSlug} />
    </>
  );
}
