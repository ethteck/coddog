import type React from 'react';
import { useCallback, useState, useEffect } from 'react';

interface SelectedRange {
  start: number | null;
  end: number | null;
}

interface AssemblyViewerProps {
  asm: string[];
  selectedRange: SelectedRange;
  setSelectedRange: React.Dispatch<React.SetStateAction<SelectedRange>>;
}

export const AssemblyViewer: React.FC<AssemblyViewerProps> = ({
  asm,
  selectedRange,
  setSelectedRange,
}) => {
  const [isSelecting, setIsSelecting] = useState(false);
  const [startLineInput, setStartLineInput] = useState<string>('');
  const [endLineInput, setEndLineInput] = useState<string>('');

  // Sync input fields with selected range
  useEffect(() => {
    setStartLineInput(
      selectedRange.start !== null ? (selectedRange.start + 1).toString() : '',
    );
    setEndLineInput(
      selectedRange.end !== null ? (selectedRange.end + 1).toString() : '',
    );
  }, [selectedRange]);

  const handleRowClick = useCallback(
    (index: number) => {
      if (selectedRange.start === null) {
        // First click - set start
        setSelectedRange({ start: index, end: null });
        setIsSelecting(true);
      } else if (selectedRange.end === null && isSelecting) {
        // Second click - set end
        const start = selectedRange.start;
        const end = index;
        setSelectedRange({
          start: Math.min(start, end),
          end: Math.max(start, end),
        });
        setIsSelecting(false);
      } else {
        // Reset selection
        setSelectedRange({ start: index, end: null });
        setIsSelecting(true);
      }
    },
    [
      selectedRange,
      isSelecting, // Reset selection
      setSelectedRange,
    ],
  );

  const clearSelection = useCallback(() => {
    setSelectedRange({ start: null, end: null });
    setIsSelecting(false);
  }, [setSelectedRange]);

  const handleStartLineChange = useCallback(
    (value: string) => {
      // Only allow digits
      const sanitizedValue = value.replace(/[^0-9]/g, '');

      // Prevent entering 0 or values starting with 0
      if (sanitizedValue.startsWith('0')) {
        return;
      }

      setStartLineInput(sanitizedValue);

      const lineNum = Number.parseInt(sanitizedValue, 10);
      if (!Number.isNaN(lineNum) && lineNum >= 1) {
        if (lineNum > asm.length) {
          // Reset to max value if beyond bounds
          setStartLineInput(asm.length.toString());
          setSelectedRange((prev) => ({ ...prev, start: asm.length - 1 }));
        } else {
          setSelectedRange((prev) => ({ ...prev, start: lineNum - 1 }));
        }
      } else if (sanitizedValue === '') {
        setSelectedRange((prev) => ({ ...prev, start: null }));
      }
    },
    [asm.length, setSelectedRange],
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
        if (lineNum > asm.length) {
          // Reset to max value if beyond bounds
          setEndLineInput(asm.length.toString());
          setSelectedRange((prev) => ({ ...prev, end: asm.length - 1 }));
        } else {
          setSelectedRange((prev) => ({ ...prev, end: lineNum - 1 }));
        }
      } else if (sanitizedValue === '') {
        setSelectedRange((prev) => ({ ...prev, end: null }));
      }
    },
    [asm.length, setSelectedRange],
  );

  const isRowInRange = useCallback(
    (index: number) => {
      const start = selectedRange.start ?? 0;
      const end = selectedRange.end ?? asm.length - 1;

      if (selectedRange.start === null && selectedRange.end === null)
        return false;

      return index >= start && index <= end;
    },
    [selectedRange, asm.length],
  );

  return (
    <>
      <h3>Assembly Code</h3>
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
              selectedRange.start === null && selectedRange.end === null
            }
          >
            Clear Selection
          </button>
        </div>

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
            // biome-ignore lint/a11y/useKeyWithClickEvents: <explanation>
            // biome-ignore lint/a11y/useSemanticElements: <explanation>
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
                  .padStart(Math.floor(Math.log2(asm.length + 1) / 4) + 2, '0')}
              </span>
              <span style={{ userSelect: 'none' }}>{line}</span>
            </div>
          ))}
        </div>
      </div>
    </>
  );
};
