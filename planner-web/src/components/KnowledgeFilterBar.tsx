import { useEffect, useMemo, useRef, useState } from 'react';

export interface KnowledgeFilterOption {
  value: string;
  label: string;
  count: number;
}

export interface KnowledgeFilterDescriptor {
  key: string;
  label: string;
  shortLabel: string;
  placement: 'primary' | 'overflow';
  value: string;
  options: KnowledgeFilterOption[];
  onChange: (value: string) => void;
}

interface KnowledgeFilterBarProps {
  descriptors: KnowledgeFilterDescriptor[];
}

function FilterSelect({ descriptor }: { descriptor: KnowledgeFilterDescriptor }) {
  const isDisabled = descriptor.options.length <= 1;

  return (
    <label className="knowledge-filter-control" htmlFor={`knowledge-filter-${descriptor.key}`}>
      <span className="knowledge-filter-label">{descriptor.shortLabel}</span>
      <select
        id={`knowledge-filter-${descriptor.key}`}
        className="knowledge-filter-select"
        aria-label={descriptor.label}
        value={descriptor.value}
        disabled={isDisabled}
        onChange={(event) => descriptor.onChange(event.target.value)}
      >
        {descriptor.options.map((option) => (
          <option key={`${descriptor.key}-${option.value}`} value={option.value}>
            {`${option.label} (${option.count})`}
          </option>
        ))}
      </select>
    </label>
  );
}

export default function KnowledgeFilterBar({ descriptors }: KnowledgeFilterBarProps) {
  const [overflowOpen, setOverflowOpen] = useState(false);
  const overflowRef = useRef<HTMLDivElement | null>(null);

  const primaryFilters = useMemo(
    () => descriptors.filter(descriptor => descriptor.placement === 'primary'),
    [descriptors],
  );
  const overflowFilters = useMemo(
    () => descriptors.filter(descriptor => descriptor.placement === 'overflow'),
    [descriptors],
  );

  useEffect(() => {
    if (!overflowOpen) return;

    const handlePointerDown = (event: MouseEvent) => {
      if (!overflowRef.current?.contains(event.target as Node)) {
        setOverflowOpen(false);
      }
    };

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setOverflowOpen(false);
      }
    };

    window.addEventListener('mousedown', handlePointerDown);
    window.addEventListener('keydown', handleEscape);
    return () => {
      window.removeEventListener('mousedown', handlePointerDown);
      window.removeEventListener('keydown', handleEscape);
    };
  }, [overflowOpen]);

  return (
    <div className="knowledge-filter-bar" aria-label="Knowledge filters">
      <div className="knowledge-filter-row">
        {primaryFilters.map((descriptor) => (
          <FilterSelect key={descriptor.key} descriptor={descriptor} />
        ))}

        {overflowFilters.length > 0 && (
          <div className="knowledge-filter-more" ref={overflowRef}>
            <button
              type="button"
              className="knowledge-filter-more-trigger"
              aria-haspopup="true"
              aria-expanded={overflowOpen}
              onClick={() => setOverflowOpen(previous => !previous)}
            >
              More Filters
            </button>
            {overflowOpen && (
              <div className="knowledge-filter-overflow-panel" role="region" aria-label="More filters">
                {overflowFilters.map((descriptor) => (
                  <FilterSelect key={descriptor.key} descriptor={descriptor} />
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
