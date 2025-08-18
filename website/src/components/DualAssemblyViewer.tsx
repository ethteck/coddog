import type React from 'react';
import { useCallback, useMemo, useRef } from 'react';
import type { AsmInsn, SymbolAsm, SymbolMetadata } from '../api/symbols.tsx';
import styles from './DualAssemblyViewer.module.css';
import { SymbolLabel } from './SymbolLabel.tsx';

interface DualAssemblyViewerProps {
  leftAsm: SymbolAsm;
  rightAsm: SymbolAsm;
  leftStartLine?: number;
  rightStartLine?: number;
  leftMetadata: SymbolMetadata;
  rightMetadata: SymbolMetadata;
  maxDisplayLines?: number;
  enableLineHighlight?: boolean;
}

export const DualAssemblyViewer: React.FC<DualAssemblyViewerProps> = ({
  leftAsm,
  rightAsm,
  leftStartLine = 0,
  rightStartLine = 0,
  leftMetadata,
  rightMetadata,
  maxDisplayLines = 50,
}) => {
  const leftScrollRef = useRef<HTMLDivElement>(null);
  const rightScrollRef = useRef<HTMLDivElement>(null);

  // Calculate the visible range based on start lines and max display lines
  const { leftRange, rightRange, totalLines } = useMemo(() => {
    const leftMaxLines = Math.min(
      leftAsm.asm.length - leftStartLine,
      maxDisplayLines,
    );
    const rightMaxLines = Math.min(
      rightAsm.asm.length - rightStartLine,
      maxDisplayLines,
    );
    const maxLines = Math.max(leftMaxLines, rightMaxLines);

    return {
      leftRange: {
        start: leftStartLine,
        end: Math.min(leftStartLine + maxLines, leftAsm.asm.length),
      },
      rightRange: {
        start: rightStartLine,
        end: Math.min(rightStartLine + maxLines, rightAsm.asm.length),
      },
      totalLines: maxLines,
    };
  }, [
    leftAsm.asm.length,
    rightAsm.asm.length,
    leftStartLine,
    rightStartLine,
    maxDisplayLines,
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

  // Format hex address based on array length
  const formatAddress = useCallback((index: number, arrayLength: number) => {
    const hexLength = Math.floor(Math.log2(arrayLength * 4 + 1) / 4) + 2;
    return `0x${(index * 4).toString(16).padStart(hexLength, '0')}`;
  }, []);

  const renderAssemblyLineContent = useCallback((asm: AsmInsn) => {
    return (
      <>
        {asm.opcode} {asm.arguments.join(', ')}{' '}
        {asm.branch_dest && (
          <span className={styles.branchDest}>→ {asm.branch_dest}</span>
        )}
      </>
    );
  }, []);

  // Render assembly lines for a given range
  const renderAssemblyLines = useCallback(
    (
      asm: AsmInsn[],
      range: { start: number; end: number },
      totalDisplayLines: number,
      side: 'left' | 'right',
    ) => {
      const lines: React.ReactNode[] = [];

      // Add assembly lines
      for (let i = 0; i < totalDisplayLines; i++) {
        const actualIndex = range.start + i;
        const hasContent = actualIndex < range.end && actualIndex < asm.length;

        lines.push(
          <div
            key={`${side}-${i}`}
            className={`${styles.assemblyLine} ${!hasContent ? styles.emptyLine : ''} `}
          >
            <span className={styles.lineNumber}>
              {hasContent ? actualIndex : ''}
            </span>
            <span className={styles.addressNumber}>
              {hasContent ? formatAddress(actualIndex, asm.length) : ''}
            </span>
            <span className={styles.lineContent}>
              {hasContent ? renderAssemblyLineContent(asm[actualIndex]) : ''}
            </span>
          </div>,
        );
      }

      return lines;
    },
    [formatAddress, renderAssemblyLineContent],
  );

  return (
    <div className={styles.dualAssemblyViewer}>
      <div className={styles.header}>
        <h3 className={styles.title}>
          <SymbolLabel symbol={leftMetadata} link={false} />
        </h3>
        <h3 className={styles.title}>
          <SymbolLabel symbol={rightMetadata} link={false} />
        </h3>
      </div>

      <div className={styles.assembliesContainer}>
        <div className={styles.assemblyPanel}>
          <div className={styles.panelInfo}>
            <span className={styles.infoText}>
              Lines {leftRange.start}-{leftRange.end} of {leftAsm.asm.length}
              {leftStartLine > 0 && ` (offset: +${leftStartLine})`}
            </span>
          </div>
          <div className={styles.columnHeaders}>
            <span className={styles.headerInsn}>Insn</span>
            <span className={styles.headerOffset}>Offset</span>
            <span className={styles.headerAsm}>Asm</span>
          </div>
          <div
            ref={leftScrollRef}
            className={styles.assemblyContainer}
            onScroll={handleScroll('left')}
          >
            {renderAssemblyLines(leftAsm.asm, leftRange, totalLines, 'left')}
          </div>
        </div>

        <div className={styles.separator} />

        <div className={styles.assemblyPanel}>
          <div className={styles.panelInfo}>
            <span className={styles.infoText}>
              Lines {rightRange.start}-{rightRange.end} of {rightAsm.asm.length}
              {rightStartLine > 0 && ` (offset: +${rightStartLine})`}
            </span>
          </div>
          <div className={styles.columnHeaders}>
            <span className={styles.headerInsn}>Insn</span>
            <span className={styles.headerOffset}>Offset</span>
            <span className={styles.headerAsm}>Asm</span>
          </div>
          <div
            ref={rightScrollRef}
            className={styles.assemblyContainer}
            onScroll={handleScroll('right')}
          >
            {renderAssemblyLines(rightAsm.asm, rightRange, totalLines, 'right')}
          </div>
        </div>
      </div>
    </div>
  );
};
