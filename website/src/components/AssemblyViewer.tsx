import type React from 'react';
import { useCallback, useEffect, useState } from 'react';

interface SelectedRange {
  start: number;
  end: number;
}

interface AssemblyViewerProps {
  asm: string[];
  selectedRange: SelectedRange | null;
  setSelectedRange: (range: SelectedRange | null) => void;
}

export const AssemblyViewer: React.FC<AssemblyViewerProps> = ({
  asm,
  selectedRange,
  setSelectedRange,
}) => {
  // Internal state for tracking incomplete selections
  const [internalSelection, setInternalSelection] = useState<{
    start: number | null;
    end: number | null;
  }>({
    start: null,
    end: null,
  });
  const [isSelecting, setIsSelecting] = useState(false);
  const [startLineInput, setStartLineInput] = useState<string>('');
  const [endLineInput, setEndLineInput] = useState<string>('');
  const [isExpanded, setIsExpanded] = useState(false);

  // Sync input fields with selected range or internal selection
  useEffect(() => {
    if (selectedRange) {
      setStartLineInput((selectedRange.start + 1).toString());
      setEndLineInput((selectedRange.end + 1).toString());
      setInternalSelection({
        start: selectedRange.start,
        end: selectedRange.end,
      });
    } else {
      setStartLineInput(
        internalSelection.start !== null
          ? (internalSelection.start + 1).toString()
          : '',
      );
      setEndLineInput(
        internalSelection.end !== null
          ? (internalSelection.end + 1).toString()
          : '',
      );
    }
  }, [selectedRange, internalSelection]);

  const handleRowClick = useCallback(
    (index: number) => {
      if (internalSelection.start === null) {
        // First click - set start
        setInternalSelection({ start: index, end: null });
        setIsSelecting(true);
      } else if (internalSelection.end === null && isSelecting) {
        // Second click - set end and commit the selection
        const start = internalSelection.start;
        const end = index;
        const finalRange = {
          start: Math.min(start, end),
          end: Math.max(start, end),
        };
        setInternalSelection(finalRange);
        setSelectedRange(finalRange);
        setIsSelecting(false);
      } else {
        // Reset selection
        setInternalSelection({ start: index, end: null });
        setSelectedRange(null);
        setIsSelecting(true);
      }
    },
    [internalSelection, isSelecting, setSelectedRange],
  );

  const clearSelection = useCallback(() => {
    setInternalSelection({ start: null, end: null });
    setSelectedRange(null);
    setIsSelecting(false);
  }, [setSelectedRange]);

  const handleStartLineChange = useCallback(
    (value: string) => {
      // Only allow digits
      const sanitizedValue = value.replace(/[^0-9]/g, '');

      setStartLineInput(sanitizedValue);

      const lineNum = Number.parseInt(sanitizedValue, 10);
      if (!Number.isNaN(lineNum) && lineNum >= 1) {
        const newStart = lineNum > asm.length ? asm.length - 1 : lineNum - 1;
        if (lineNum > asm.length) {
          setStartLineInput(asm.length.toString());
        }

        const newInternalSelection = { ...internalSelection, start: newStart };
        setInternalSelection(newInternalSelection);

        // If both start and end are set, commit the selection
        if (newInternalSelection.end !== null) {
          setSelectedRange({
            start: Math.min(newStart, newInternalSelection.end),
            end: Math.max(newStart, newInternalSelection.end),
          });
        } else {
          setSelectedRange(null);
        }
      } else if (sanitizedValue === '') {
        setInternalSelection({ ...internalSelection, start: null });
        setSelectedRange(null);
      }
    },
    [asm.length, internalSelection, setSelectedRange],
  );

  const handleEndLineChange = useCallback(
    (value: string) => {
      // Only allow digits
      const sanitizedValue = value.replace(/[^0-9]/g, '');

      // Prevent entering 0 or values starting with 0
      if (sanitizedValue.startsWith('0')) {
        return;
      }

      setEndLineInput(sanitizedValue);

      const lineNum = Number.parseInt(sanitizedValue, 10);
      if (!Number.isNaN(lineNum) && lineNum >= 1) {
        const newEnd = lineNum > asm.length ? asm.length - 1 : lineNum - 1;
        if (lineNum > asm.length) {
          setEndLineInput(asm.length.toString());
        }

        const newInternalSelection = { ...internalSelection, end: newEnd };
        setInternalSelection(newInternalSelection);

        // If both start and end are set, commit the selection
        if (newInternalSelection.start !== null) {
          setSelectedRange({
            start: Math.min(newInternalSelection.start, newEnd),
            end: Math.max(newInternalSelection.start, newEnd),
          });
        } else {
          setSelectedRange(null);
        }
      } else if (sanitizedValue === '') {
        setInternalSelection({ ...internalSelection, end: null });
        setSelectedRange(null);
      }
    },
    [asm.length, internalSelection, setSelectedRange],
  );

  const isRowInRange = useCallback(
    (index: number) => {
      // Use selectedRange if available, otherwise use internal selection for visual feedback
      const range = selectedRange || internalSelection;

      if (!range || (range.start === null && range.end === null)) return false;

      // Handle partial selection (only start is set)
      if (range.start !== null && range.end === null) {
        return index === range.start;
      }

      // Handle complete selection
      if (range.start !== null && range.end !== null) {
        const start = Math.min(range.start, range.end);
        const end = Math.max(range.start, range.end);
        return index >= start && index <= end;
      }

      return false;
    },
    [selectedRange, internalSelection],
  );

  const toggleExpanded = useCallback(() => {
    setIsExpanded((prev) => !prev);
  }, []);

  return (
    <>
      <h3>Search range:</h3>
      <div style={{ fontFamily: 'monospace', fontSize: '14px' }}>
        <div
          style={{
            marginBottom: '10px',
            display: 'flex',
            gap: '10px',
            alignItems: 'center',
            flexWrap: 'wrap',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '5px' }}>
            <label>
              Start:
              <input
                type="text"
                value={startLineInput}
                onChange={(e) => handleStartLineChange(e.target.value)}
                placeholder="1"
                style={{
                  width: '40px',
                  padding: '2px 4px',
                  fontFamily: 'monospace',
                  fontSize: '12px',
                }}
              />
            </label>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '5px' }}>
            <label>
              End:
              <input
                type="text"
                value={endLineInput}
                onChange={(e) => handleEndLineChange(e.target.value)}
                placeholder={asm.length.toString()}
                style={{
                  width: '40px',
                  padding: '2px 4px',
                  fontFamily: 'monospace',
                  fontSize: '12px',
                }}
              />
            </label>
          </div>

          <button
            type="button"
            onClick={clearSelection}
            disabled={
              selectedRange === null &&
              internalSelection.start === null &&
              internalSelection.end === null
            }
          >
            Reset selection
          </button>
        </div>
        <button
          type="button"
          onClick={toggleExpanded}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '4px',
            padding: '4px 8px',
            borderRadius: '4px',
            cursor: 'pointer',
          }}
        >
          <span
            style={{
              transform: isExpanded ? 'rotate(90deg)' : 'rotate(0deg)',
              transition: 'transform 0.2s ease',
              fontSize: '12px',
            }}
          >
            ▶
          </span>
          {isExpanded ? 'Collapse' : 'Show asm'}
        </button>
        {isExpanded && (
          <div
            style={{
              border: '1px solid #ccc',
              borderRadius: '4px',
              maxHeight: '500px',
              overflowY: 'auto',
              backgroundColor: '#640d0dff',
            }}
          >
            {asm.map((line, index) => (
              // biome-ignore lint/a11y/useKeyWithClickEvents: div used for interactive row selection
              // biome-ignore lint/a11y/useSemanticElements: div with role button is appropriate here
              <div
                role="button"
                tabIndex={index}
                key={index + line}
                onClick={() => handleRowClick(index)}
                style={{
                  padding: '4px 8px',
                  borderBottom:
                    index < asm.length - 1 ? '1px solid #eee' : 'none',
                  cursor: 'pointer',
                  backgroundColor: isRowInRange(index)
                    ? '#bc8334ff'
                    : 'transparent',
                  display: 'flex',
                  alignItems: 'center',
                }}
                onMouseEnter={(e) => {
                  if (!isRowInRange(index)) {
                    e.currentTarget.style.backgroundColor = '#640d0dff';
                  }
                }}
                onMouseLeave={(e) => {
                  if (!isRowInRange(index)) {
                    e.currentTarget.style.backgroundColor = 'transparent';
                  }
                }}
              >
                <span
                  style={{
                    minWidth: '40px',
                    color: '#666',
                    marginRight: '10px',
                    textAlign: 'right',
                    userSelect: 'none',
                  }}
                >
                  {index + 1}
                </span>
                <span
                  style={{
                    minWidth: '40px',
                    color: '#666',
                    marginRight: '10px',
                    textAlign: 'right',
                    userSelect: 'none',
                  }}
                >
                  0x
                  {(index * 4)
                    .toString(16)
                    .padStart(
                      Math.floor(Math.log2(asm.length + 1) / 4) + 2,
                      '0',
                    )}
                </span>
                <span style={{ userSelect: 'none' }}>{line}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </>
  );
};
