import type React from 'react';
import { useCallback, useMemo, useRef } from 'react';
import type { AsmInsn, SymbolAsm, SymbolMetadata } from '../api/symbols.tsx';
import styles from './DualAssemblyViewer.module.css';
import { SymbolLabel } from './SymbolLabel.tsx';

type DiffType = {
  opcodeMatch: boolean;
  argumentsMatch: boolean;
  argumentDiffs: boolean[]; // Array indicating which arguments differ
  addressMatch: boolean;
  branchDestMatch: boolean;
  isIdentical: boolean;
};
interface DualAssemblyViewerProps {
  leftAsm: SymbolAsm;
  rightAsm: SymbolAsm;
  leftStartLine?: number;
  rightStartLine?: number;
  leftMetadata: SymbolMetadata;
  rightMetadata: SymbolMetadata;
  maxDisplayLines: number;
  enableLineHighlight?: boolean;
  contextLines?: number; // Number of context lines to show before/after
}

export const DualAssemblyViewer: React.FC<DualAssemblyViewerProps> = ({
  leftAsm,
  rightAsm,
  leftStartLine = 0,
  rightStartLine = 0,
  leftMetadata,
  rightMetadata,
  maxDisplayLines,
  contextLines = 3,
}) => {
  const leftScrollRef = useRef<HTMLDivElement>(null);
  const rightScrollRef = useRef<HTMLDivElement>(null);

  // Calculate the visible range based on start lines and max display lines
  const { leftRange, rightRange, totalLines } = useMemo(() => {
    const totalContextBefore = contextLines;
    const totalContextAfter = contextLines;

    // Calculate main content lines
    const leftMaxLines = Math.min(
      leftAsm.asm.length - leftStartLine,
      maxDisplayLines,
    );
    const rightMaxLines = Math.min(
      rightAsm.asm.length - rightStartLine,
      maxDisplayLines,
    );
    const maxLines = Math.max(leftMaxLines, rightMaxLines);

    // Calculate context-aware ranges with alignment
    const leftActualContextBefore = Math.min(totalContextBefore, leftStartLine);
    const rightActualContextBefore = Math.min(
      totalContextBefore,
      rightStartLine,
    );

    // Use the maximum context before to ensure alignment
    const alignedContextBefore = Math.max(
      leftActualContextBefore,
      rightActualContextBefore,
    );

    const leftContextStart = leftStartLine - alignedContextBefore;
    const rightContextStart = rightStartLine - alignedContextBefore;

    const leftContextEnd = Math.min(
      leftStartLine + maxLines + totalContextAfter,
      leftAsm.asm.length,
    );
    const rightContextEnd = Math.min(
      rightStartLine + maxLines + totalContextAfter,
      rightAsm.asm.length,
    );

    // Calculate total lines including potential dummy context
    const leftTotalLines =
      leftStartLine -
      leftContextStart +
      maxLines +
      Math.min(
        totalContextAfter,
        leftAsm.asm.length - (leftStartLine + maxLines),
      );
    const rightTotalLines =
      rightStartLine -
      rightContextStart +
      maxLines +
      Math.min(
        totalContextAfter,
        rightAsm.asm.length - (rightStartLine + maxLines),
      );

    return {
      leftRange: {
        contextStart: leftContextStart,
        start: leftStartLine,
        end: Math.min(leftStartLine + maxLines, leftAsm.asm.length),
        contextEnd: leftContextEnd,
        actualContextBefore: leftStartLine - leftContextStart,
        dummyContextBefore:
          alignedContextBefore - (leftStartLine - leftContextStart),
      },
      rightRange: {
        contextStart: rightContextStart,
        start: rightStartLine,
        end: Math.min(rightStartLine + maxLines, rightAsm.asm.length),
        contextEnd: rightContextEnd,
        actualContextBefore: rightStartLine - rightContextStart,
        dummyContextBefore:
          alignedContextBefore - (rightStartLine - rightContextStart),
      },
      totalLines: Math.max(leftTotalLines, rightTotalLines),
    };
  }, [
    leftAsm.asm.length,
    rightAsm.asm.length,
    leftStartLine,
    rightStartLine,
    maxDisplayLines,
    contextLines,
  ]);

  // Sync scrolling between left and right panels
  const handleScroll = useCallback((source: 'left' | 'right') => {
    return (event: React.UIEvent<HTMLDivElement>) => {
      const scrollTop = event.currentTarget.scrollTop;
      const otherRef = source === 'left' ? rightScrollRef : leftScrollRef;

      if (otherRef.current && otherRef.current.scrollTop !== scrollTop) {
        otherRef.current.scrollTop = scrollTop;
      }
    };
  }, []);

  // Compare two assembly instructions and return diff information
  const compareDiff = useCallback(
    (leftAsm: AsmInsn | null, rightAsm: AsmInsn | null): DiffType => {
      if (!leftAsm && !rightAsm) {
        return {
          opcodeMatch: true,
          argumentsMatch: true,
          argumentDiffs: [],
          addressMatch: true,
          branchDestMatch: true,
          isIdentical: true,
        };
      }

      if (!leftAsm || !rightAsm) {
        const maxArgsLength = Math.max(
          leftAsm?.arguments.length || 0,
          rightAsm?.arguments.length || 0,
        );
        return {
          opcodeMatch: false,
          argumentsMatch: false,
          argumentDiffs: Array(maxArgsLength).fill(true), // All arguments are different
          addressMatch: false,
          branchDestMatch: false,
          isIdentical: false,
        };
      }

      const opcodeMatch = leftAsm.opcode === rightAsm.opcode;
      const addressMatch = leftAsm.address === rightAsm.address;
      const branchDestMatch = leftAsm.branch_dest === rightAsm.branch_dest;

      // Compare arguments individually
      const maxArgsLength = Math.max(
        leftAsm.arguments.length,
        rightAsm.arguments.length,
      );
      const argumentDiffs: boolean[] = [];
      let argumentsMatch = true;

      for (let i = 0; i < maxArgsLength; i++) {
        const leftArg = leftAsm.arguments[i] || '';
        const rightArg = rightAsm.arguments[i] || '';
        const argDifferent = leftArg !== rightArg;
        argumentDiffs.push(argDifferent);
        if (argDifferent) {
          argumentsMatch = false;
        }
      }

      const isIdentical =
        opcodeMatch && argumentsMatch && addressMatch && branchDestMatch;

      return {
        opcodeMatch,
        argumentsMatch,
        argumentDiffs,
        addressMatch,
        branchDestMatch,
        isIdentical,
      };
    },
    [],
  );

  const renderAssemblyLineContent = useCallback(
    (asm: AsmInsn, diff?: DiffType) => {
      const opcodeClass = diff && !diff.opcodeMatch ? styles.diffOpcode : '';
      const branchDestClass =
        diff && !diff.branchDestMatch ? styles.diffBranchDest : '';

      // Render arguments individually with diff highlighting
      const renderArguments = () => {
        if (asm.arguments.length === 0) return null;

        return asm.arguments.map((arg, index) => {
          const isLastArg = index === asm.arguments.length - 1;
          const argClass = diff?.argumentDiffs[index]
            ? styles.diffArguments
            : '';

          return (
            <span key={`${arg}-${asm.opcode}-${index}`}>
              <span className={`${styles.argument} ${argClass}`}>{arg}</span>
              {!isLastArg && <span>, </span>}
            </span>
          );
        });
      };

      return (
        <>
          <span className={`${styles.opcode} ${opcodeClass}`}>
            {asm.opcode}
          </span>{' '}
          <span className={styles.arguments}>{renderArguments()}</span>
          {asm.branch_dest && (
            <span className={`${styles.branchDest} ${branchDestClass}`}>
              {asm.arguments.length !== 0 ? ' ' : ''}
              {'->'} {asm.branch_dest}
            </span>
          )}
        </>
      );
    },
    [],
  );

  // Render assembly lines for a given range
  const renderAssemblyLines = useCallback(
    (
      asm: AsmInsn[],
      range: {
        contextStart: number;
        start: number;
        end: number;
        contextEnd: number;
        actualContextBefore: number;
        dummyContextBefore: number;
      },
      totalDisplayLines: number,
      side: 'left' | 'right',
      otherAsm?: AsmInsn[],
      otherRange?: {
        contextStart: number;
        start: number;
        end: number;
        contextEnd: number;
        actualContextBefore: number;
        dummyContextBefore: number;
      },
    ) => {
      const lines: React.ReactNode[] = [];

      // Add dummy context lines first (for alignment when one side starts at 0)
      for (let i = 0; i < range.dummyContextBefore; i++) {
        lines.push(
          <div
            key={`${side}-dummy-${i}`}
            className={`${styles.assemblyLine} ${styles.emptyLine}`}
          >
            <span className={styles.lineNumber} />
            <span className={styles.lineContent} />
          </div>,
        );
      }

      // Add actual assembly lines (including real context)
      for (let i = 0; i < totalDisplayLines - range.dummyContextBefore; i++) {
        const actualIndex = range.contextStart + i;
        const hasContent =
          actualIndex < range.contextEnd &&
          actualIndex < asm.length &&
          actualIndex >= 0;

        // Determine if this line is in the context area or main diff area
        const isContextBefore = actualIndex < range.start;
        const isContextAfter = actualIndex >= range.end;
        const isContextLine = isContextBefore || isContextAfter;

        // Calculate corresponding index in the other assembly for comparison
        let diff: DiffType | undefined;
        if (otherAsm && otherRange && hasContent && !isContextLine) {
          const relativeIndex = actualIndex - range.contextStart;
          const otherActualIndex = otherRange.contextStart + relativeIndex;
          const otherHasContent =
            otherActualIndex < otherRange.contextEnd &&
            otherActualIndex < otherAsm.length &&
            otherActualIndex >= 0;

          const leftAsm =
            side === 'left'
              ? hasContent
                ? asm[actualIndex]
                : null
              : otherHasContent
                ? otherAsm[otherActualIndex]
                : null;
          const rightAsm =
            side === 'right'
              ? hasContent
                ? asm[actualIndex]
                : null
              : otherHasContent
                ? otherAsm[otherActualIndex]
                : null;

          diff = compareDiff(leftAsm, rightAsm);
        }

        // Apply appropriate styling based on line type
        let lineClass = '';
        if (!hasContent) {
          lineClass = styles.emptyLine;
        } else if (isContextLine) {
          lineClass = styles.contextLine;
        } else if (diff) {
          lineClass = diff.isIdentical
            ? styles.identicalLine
            : styles.differentLine;
        }

        lines.push(
          <div
            key={`${side}-${actualIndex}`}
            className={`${styles.assemblyLine} ${lineClass}`}
          >
            <span className={styles.lineNumber}>
              {hasContent ? (actualIndex * 4).toString(16).toUpperCase() : ''}
            </span>
            <span className={styles.lineContent}>
              {hasContent
                ? renderAssemblyLineContent(
                    asm[actualIndex],
                    isContextLine ? undefined : diff,
                  )
                : ''}
            </span>
          </div>,
        );
      }

      return lines;
    },
    [renderAssemblyLineContent, compareDiff],
  );

  return (
    <div className={styles.dualAssemblyViewer}>
      <div className={styles.header}>
        <h3 className={styles.title}>
          <SymbolLabel symbol={leftMetadata} link={true} />
        </h3>
        <h3 className={styles.title}>
          <SymbolLabel symbol={rightMetadata} link={true} />
        </h3>
      </div>

      <div className={styles.assembliesContainer}>
        <div className={styles.assemblyPanel}>
          <div className={styles.columnHeaders}>
            <span className={styles.headerInsn}>Insn</span>
            <span className={styles.headerAsm}>Asm</span>
          </div>
          <div
            ref={leftScrollRef}
            className={styles.assemblyContainer}
            onScroll={handleScroll('left')}
          >
            {renderAssemblyLines(
              leftAsm.asm,
              leftRange,
              totalLines,
              'left',
              rightAsm.asm,
              rightRange,
            )}
          </div>
        </div>

        <div className={styles.separator} />

        <div className={styles.assemblyPanel}>
          <div className={styles.columnHeaders}>
            <span className={styles.headerInsn}>Insn</span>
            <span className={styles.headerAsm}>Asm</span>
          </div>
          <div
            ref={rightScrollRef}
            className={styles.assemblyContainer}
            onScroll={handleScroll('right')}
          >
            {renderAssemblyLines(
              rightAsm.asm,
              rightRange,
              totalLines,
              'right',
              leftAsm.asm,
              leftRange,
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
