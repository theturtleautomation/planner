import { useEffect, useState, useMemo, useCallback, useRef } from 'react';
import Layout from '../components/Layout.tsx';
import BlueprintGraph from '../components/BlueprintGraph.tsx';
import NodeDetailPanel from '../components/NodeDetailPanel.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import type { BlueprintResponse, NodeType, NodeSummary } from '../types/blueprint.ts';

// ─── Node type filter config ────────────────────────────────────────────────

const NODE_TYPES: { value: NodeType | null; label: string; color: string }[] = [
  { value: null,                  label: 'All',       color: 'var(--text-primary)' },
  { value: 'decision',           label: 'Decision',   color: '#4f98a3' },
  { value: 'technology',         label: 'Technology',  color: '#6daa45' },
  { value: 'component',          label: 'Component',   color: '#5591c7' },
  { value: 'constraint',         label: 'Constraint',  color: '#bb653b' },
  { value: 'pattern',            label: 'Pattern',     color: '#a86fdf' },
  { value: 'quality_requirement', label: 'Quality',    color: '#e8af34' },
];

// ─── BlueprintPage ──────────────────────────────────────────────────────────

export default function BlueprintPage() {
  const getToken = useGetAccessToken();
  const api = useMemo(() => createApiClient(getToken), [getToken]);

  // Data
  const [blueprint, setBlueprint] = useState<BlueprintResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [fetchError, setFetchError] = useState<string | null>(null);

  // UI state
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [hoveredNodeId, setHoveredNodeId] = useState<string | null>(null);
  const [filterType, setFilterType] = useState<NodeType | null>(null);

  // Container sizing
  const containerRef = useRef<HTMLDivElement>(null);
  const [dimensions, setDimensions] = useState({ width: 800, height: 600 });

  // Fetch blueprint
  const loadBlueprint = useCallback(async () => {
    setLoading(true);
    setFetchError(null);
    try {
      const data = await api.getBlueprint();
      setBlueprint(data);
    } catch (err) {
      setFetchError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [api]);

  useEffect(() => {
    void loadBlueprint();
  }, [loadBlueprint]);

  // Resize observer
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const observer = new ResizeObserver(entries => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        setDimensions({ width: Math.floor(width), height: Math.floor(height) });
      }
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  // Filtered node counts for chips
  const filteredCounts = useMemo(() => {
    if (!blueprint) return {};
    return blueprint.counts;
  }, [blueprint]);

  // Graph dimensions account for detail panel
  const detailPanelWidth = 340;
  const graphWidth = selectedNodeId
    ? Math.max(dimensions.width - detailPanelWidth, 300)
    : dimensions.width;

  // Hovered node info for tooltip
  const hoveredNode: NodeSummary | null = useMemo(() => {
    if (!hoveredNodeId || !blueprint) return null;
    return blueprint.nodes.find(n => n.id === hoveredNodeId) ?? null;
  }, [hoveredNodeId, blueprint]);

  return (
    <Layout>
      <div style={{
        flex: 1,
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
      }}>
        {/* Toolbar */}
        <div style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '8px 16px',
          borderBottom: '1px solid var(--border)',
          background: 'var(--bg-secondary)',
          flexShrink: 0,
          gap: '12px',
          flexWrap: 'wrap',
        }}>
          {/* Left: title + counts */}
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <a
              href="/"
              style={{
                color: 'var(--text-secondary)',
                fontSize: '11px',
                textDecoration: 'none',
                opacity: 0.7,
              }}
            >
              ← sessions
            </a>
            <span style={{
              fontSize: '13px',
              fontWeight: 600,
              color: 'var(--text-primary)',
              letterSpacing: '0.02em',
            }}>
              blueprint
            </span>
            {blueprint && (
              <span style={{
                fontSize: '10px',
                color: 'var(--text-secondary)',
                fontFamily: 'monospace',
              }}>
                {blueprint.total_nodes} nodes · {blueprint.total_edges} edges
              </span>
            )}
          </div>

          {/* Right: type filter chips */}
          <div style={{ display: 'flex', alignItems: 'center', gap: '4px', flexWrap: 'wrap' }}>
            {NODE_TYPES.map(t => {
              const isActive = filterType === t.value;
              const count = t.value === null
                ? (blueprint?.total_nodes ?? 0)
                : (filteredCounts[t.value] ?? 0);
              return (
                <button
                  key={t.label}
                  onClick={() => setFilterType(isActive && t.value !== null ? null : t.value)}
                  style={{
                    background: isActive ? `${t.color}18` : 'transparent',
                    border: `1px solid ${isActive ? t.color : 'var(--border)'}`,
                    color: isActive ? t.color : 'var(--text-secondary)',
                    padding: '3px 8px',
                    fontSize: '10px',
                    fontWeight: isActive ? 600 : 400,
                    letterSpacing: '0.04em',
                    borderRadius: '10px',
                    cursor: 'pointer',
                    fontFamily: 'inherit',
                    transition: 'all 180ms ease',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {t.label}
                  {count > 0 && (
                    <span style={{ marginLeft: '4px', opacity: 0.6 }}>{count}</span>
                  )}
                </button>
              );
            })}

            {/* Refresh */}
            <button
              onClick={() => void loadBlueprint()}
              title="Refresh blueprint data"
              style={{
                background: 'transparent',
                border: '1px solid var(--border)',
                color: 'var(--text-secondary)',
                padding: '3px 8px',
                fontSize: '10px',
                borderRadius: '10px',
                cursor: 'pointer',
                fontFamily: 'inherit',
              }}
            >
              ↻
            </button>
          </div>
        </div>

        {/* Main content area */}
        <div style={{ flex: 1, display: 'flex', overflow: 'hidden', position: 'relative' }}>
          {/* Graph container */}
          <div
            ref={containerRef}
            style={{
              flex: 1,
              overflow: 'hidden',
              position: 'relative',
              background: 'var(--bg-primary)',
            }}
          >
            {loading && (
              <div style={{
                position: 'absolute',
                inset: 0,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: 'var(--text-secondary)',
                fontSize: '13px',
                zIndex: 2,
              }}>
                loading blueprint…
              </div>
            )}

            {fetchError && (
              <div style={{
                position: 'absolute',
                inset: 0,
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '12px',
                zIndex: 2,
              }}>
                <div style={{ color: 'var(--accent-red)', fontSize: '13px' }}>
                  failed to load blueprint
                </div>
                <div style={{ color: 'var(--text-secondary)', fontSize: '11px', maxWidth: '400px', textAlign: 'center' }}>
                  {fetchError}
                </div>
                <button
                  onClick={() => void loadBlueprint()}
                  style={{
                    background: 'transparent',
                    border: '1px solid var(--accent-cyan)',
                    color: 'var(--accent-cyan)',
                    padding: '6px 16px',
                    fontSize: '11px',
                    cursor: 'pointer',
                    borderRadius: '3px',
                    fontFamily: 'inherit',
                  }}
                >
                  retry
                </button>
              </div>
            )}

            {!loading && !fetchError && blueprint && (
              <BlueprintGraph
                nodes={blueprint.nodes}
                edges={blueprint.edges}
                selectedNodeId={selectedNodeId}
                onSelectNode={setSelectedNodeId}
                onHoverNode={setHoveredNodeId}
                width={graphWidth}
                height={dimensions.height}
                filterType={filterType}
              />
            )}

            {/* Hover tooltip */}
            {hoveredNode && !selectedNodeId && (
              <div style={{
                position: 'absolute',
                bottom: '16px',
                left: '16px',
                background: 'var(--bg-secondary)',
                border: '1px solid var(--border)',
                borderRadius: '3px',
                padding: '8px 12px',
                fontSize: '11px',
                color: 'var(--text-primary)',
                pointerEvents: 'none',
                zIndex: 10,
                maxWidth: '280px',
                boxShadow: '0 4px 12px rgba(0,0,0,0.4)',
              }}>
                <div style={{ fontWeight: 600, marginBottom: '2px' }}>{hoveredNode.name}</div>
                <div style={{ color: 'var(--text-secondary)', fontSize: '10px' }}>
                  {hoveredNode.node_type}
                  {hoveredNode.tags.length > 0 && ` · ${hoveredNode.tags.join(', ')}`}
                </div>
              </div>
            )}

            {/* Legend */}
            <div style={{
              position: 'absolute',
              top: '12px',
              right: selectedNodeId ? `${detailPanelWidth + 16}px` : '12px',
              background: 'var(--bg-secondary)',
              border: '1px solid var(--border)',
              borderRadius: '3px',
              padding: '8px 12px',
              fontSize: '10px',
              display: 'flex',
              flexDirection: 'column',
              gap: '3px',
              opacity: 0.85,
              transition: 'right 200ms ease',
            }}>
              {NODE_TYPES.filter(t => t.value !== null).map(t => (
                <div key={t.label} style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                  <span style={{
                    width: '8px', height: '8px', borderRadius: '2px',
                    background: t.color, display: 'inline-block', flexShrink: 0,
                  }} />
                  <span style={{ color: 'var(--text-secondary)' }}>{t.label}</span>
                </div>
              ))}
            </div>
          </div>

          {/* Detail panel (slides in when a node is selected) */}
          {selectedNodeId && blueprint && (
            <div style={{
              width: `${detailPanelWidth}px`,
              flexShrink: 0,
              overflow: 'hidden',
            }}>
              <NodeDetailPanel
                nodeId={selectedNodeId}
                edges={blueprint.edges}
                api={api}
                onClose={() => setSelectedNodeId(null)}
              />
            </div>
          )}
        </div>
      </div>
    </Layout>
  );
}
