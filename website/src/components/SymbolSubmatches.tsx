import { Link } from '@tanstack/react-router';
import type { SymbolMetadata, SymbolSubmatchResult } from '../api/symbols.tsx';
import { SymbolLabel } from './SymbolLabel.tsx';
import styles from './SymbolSubmatches.module.css';

function SubmatchCard({
  submatch,
  querySym,
}: {
  submatch: SymbolSubmatchResult;
  querySym: SymbolMetadata;
}) {
  const querySymLen = querySym.len;
  const matchSymLen = submatch.symbol.len;

  const queryOffsetPercent = submatch.query_start / querySymLen;
  const queryHeightPercent = submatch.len / querySymLen;
  const matchOffsetPercent = submatch.match_start / matchSymLen;
  const matchHeightPercent = submatch.len / matchSymLen;

  return (
    <div
      key={`${submatch.symbol.slug}_${submatch.query_start}_${submatch.match_start}_${submatch.len}`}
      className={styles.submatchCard}
    >
      <div className={styles.submatchHeader}>
        <SymbolLabel symbol={submatch.symbol} />
        <span className={styles.submatchMeta} />
      </div>

      <div className={styles.submatchContent}>
        <div className={styles.visualsContainer}>
          <svg width="30px" height="100px" className={styles.matchVisual}>
            <title>Query Match</title>

            <rect
              x="0%"
              y={`${queryOffsetPercent * 100}%`}
              width="100%"
              height={`${queryHeightPercent * 100}%`}
              className={styles.matchVisualHighlight}
            />
          </svg>

          <svg width="30px" height="100px" className={styles.matchVisual}>
            <title>Match Match lol</title>

            <rect
              x="0%"
              y={`${matchOffsetPercent * 100}%`}
              width="100%"
              height={`${matchHeightPercent * 100}%`}
              className={styles.matchVisualHighlight}
            />
          </svg>
        </div>

        <div className={styles.matchDetails}>
          <span className={styles.matchDetailsLabel}>Length:</span>
          <span className={styles.matchDetailsValue}>{submatch.len}</span>
          <span className={styles.matchDetailsLabel}>Query:</span>
          <span className={styles.matchDetailsValue}>
            {submatch.query_start} - {submatch.query_start + submatch.len} (
            {queryHeightPercent.toLocaleString(undefined, {
              style: 'percent',
              maximumFractionDigits: 2,
            })}
            )
          </span>
          <span className={styles.matchDetailsLabel}>Match:</span>
          <span className={styles.matchDetailsValue}>
            {submatch.match_start} - {submatch.match_start + submatch.len} (
            {matchHeightPercent.toLocaleString(undefined, {
              style: 'percent',
              maximumFractionDigits: 2,
            })}
            )
          </span>
        </div>

        <div className={styles.submatchCompareLink}>
          <Link
            to="/compare"
            search={{
              sym1: querySym.slug,
              start1: submatch.query_start,
              sym2: submatch.symbol.slug,
              start2: submatch.match_start,
              len: submatch.len,
            }}
            className="button"
          >
            Compare
          </Link>
        </div>
      </div>
    </div>
  );
}

export function SymbolSubmatches({
  querySym,
  submatches,
}: {
  querySym: SymbolMetadata;
  submatches: SymbolSubmatchResult[];
}) {
  // Sort submatches by length in descending order
  const sortedSubmatches = submatches.sort((a, b) => b.len - a.len);

  return (
    <>
      {sortedSubmatches.length === 0 ? (
        <p className={styles.noMatches}>No submatches found.</p>
      ) : (
        <div className={styles.submatchList}>
          {sortedSubmatches.map((submatch) => {
            return (
              <SubmatchCard
                key={`${submatch.symbol.slug}_${submatch.query_start}_${submatch.match_start}_${submatch.len}`}
                submatch={submatch}
                querySym={querySym}
              />
            );
          })}
        </div>
      )}
    </>
  );
}
