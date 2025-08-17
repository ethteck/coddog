import { useQuery } from '@tanstack/react-query';
import { fetchSymbolMatches } from '../api/symbols.tsx';
import { SymbolLabel } from './SymbolLabel.tsx';
import styles from './SymbolMatches.module.css';

export function SymbolMatches({ slug }: { slug: string }) {
  const {
    data: matchResults,
    isLoading,
    isError,
    error,
  } = useQuery({
    queryKey: ['match', slug],
    queryFn: () => fetchSymbolMatches(slug),
  });

  if (isLoading)
    return <div className={styles.loading}>Loading match results...</div>;
  if (isError)
    return <div className={styles.error}>{(error as Error).message}</div>;
  if (!matchResults)
    return (
      <div className={styles.error}>Match results could not be loaded</div>
    );

  return (
    <div className={styles.matchesSection}>
      <h3 className={styles.matchesTitle}>
        Full function matches ({matchResults.length})
      </h3>
      <div className={styles.matchesList}>
        {matchResults.map((match) => (
          <div
            key={`${match.symbol.slug}_${match.subtype}`}
            className={styles.matchCard}
          >
            <div className={styles.matchHeader}>
              <span
                className={`${styles.matchBadge} ${styles[match.subtype] || styles.default}`}
              >
                {match.subtype}
              </span>
              <SymbolLabel symbol={match.symbol} />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
