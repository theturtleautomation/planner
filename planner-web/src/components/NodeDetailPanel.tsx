import { useState, useEffect, useMemo } from 'react';
import type { ApiClient } from '../api/client.ts';
import type { BlueprintNode, EdgePayload, ImpactReport, ImpactEntry } from '../types/blueprint.ts';

// ─── Severity badge colors ──────────────────────────────────────────────────

const SEVERITY_COLORS: Record<string, { bg: string; border: string; text: string }> = {
  shallow: { bg: 'rgba(109,170,69,0.10)', border: 'rgba(109,170,69,0.4)', text: '#6daa45' },
  medium:  { bg: 'rgba(232,175,52,0.10)', border: 'rgba(232,175,52,0.4)', text: '#e8af34' },
  deep:    { bg: 'rgba(209,99,167,0.10)', border: 'rgba(209,99,167,0.4)', text: '#d163a7' },
};

const ACTION_LABELS: Record<string, string> = {
  reconverge: 'RECONVERGE',
  update: 'UPDATE',
  invalidate: 'INVALIDATE',
  add: 'ADD',
  remove: 'REMOVE',
};

// ─── Shared styles ──────────────────────────────────────────────────────────

const sectionHeaderStyle: React.CSSProperties = {
  fontSize: '10px',
  fontWeight: 600,
  letterSpacing: '0.08em',
  textTransform: 'uppercase' as const,
  color: 'var(--text-secondary)',
  marginBottom: '6px',
};

const tagStyle: React.CSSProperties = {
  display: 'inline-block',
  padding: '1px 6px',
  fontSize: '10px',
  borderRadius: '2px',
  border: '1px solid var(--border)',
  background: 'var(--bg-tertiary)',
  color: 'var(--text-secondary)',
  fontFamily: 'monospace',
};

// ─── Node type configs ──────────────────────────────────────────────────────

const TYPE_COLORS: Record<string, string> = {
  decision: '#4f98a3',
  technology: '#6daa45',
  component: '#5591c7',
  constraint: '#bb653b',
  pattern: '#a86fdf',
  quality_requirement: '#e8af34',
};

// ─── Sub-components ─────────────────────────────────────────────────────────

function DecisionDetail({ node }: { node: BlueprintNode & { node_type: 'decision' } }) {
  return (
    <>
      <div style={sectionHeaderStyle}>status</div>
      <StatusBadge status={node.status} />

      <div style={{ ...sectionHeaderStyle, marginTop: '12px' }}>context</div>
      <p style={{ fontSize: '12px', color: 'var(--text-primary)', lineHeight: 1.6, margin: 0 }}>
        {node.context}
      </p>

      {node.options.length > 0 && (
        <>
          <div style={{ ...sectionHeaderStyle, marginTop: '12px' }}>options</div>
          {node.options.map((opt, i) => (
            <div key={i} style={{
              padding: '8px 10px',
              background: 'var(--bg-primary)',
              border: '1px solid var(--border)',
              borderRadius: '3px',
              marginBottom: '6px',
            }}>
              <div style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-primary)', marginBottom: '4px' }}>
                {opt.name}
              </div>
              <div style={{ fontSize: '11px', color: 'var(--text-secondary)', marginBottom: '4px' }}>
                {opt.description}
              </div>
              {opt.pros.length > 0 && (
                <div style={{ fontSize: '10px', color: '#6daa45' }}>
                  + {opt.pros.join(' · ')}
                </div>
              )}
              {opt.cons.length > 0 && (
                <div style={{ fontSize: '10px', color: '#bb653b' }}>
                  − {opt.cons.join(' · ')}
                </div>
              )}
            </div>
          ))}
        </>
      )}

      {node.consequences.length > 0 && (
        <>
          <div style={{ ...sectionHeaderStyle, marginTop: '12px' }}>consequences</div>
          {node.consequences.map((c, i) => (
            <div key={i} style={{
              fontSize: '11px', color: 'var(--text-primary)', padding: '4px 0',
              borderBottom: '1px solid var(--border)',
            }}>
              <span style={{
                color: c.type === 'positive' ? '#6daa45' : c.type === 'negative' ? '#bb653b' : 'var(--text-secondary)',
                fontWeight: 600,
                marginRight: '6px',
              }}>
                {c.type === 'positive' ? '+' : c.type === 'negative' ? '−' : '·'}
              </span>
              {c.description}
            </div>
          ))}
        </>
      )}
    </>
  );
}

function TechnologyDetail({ node }: { node: BlueprintNode & { node_type: 'technology' } }) {
  return (
    <>
      <div style={{ display: 'flex', gap: '8px', alignItems: 'center', marginBottom: '8px' }}>
        <span style={tagStyle}>{node.category}</span>
        <RingBadge ring={node.ring} />
        {node.version && <span style={tagStyle}>v{node.version}</span>}
      </div>
      <div style={sectionHeaderStyle}>rationale</div>
      <p style={{ fontSize: '12px', color: 'var(--text-primary)', lineHeight: 1.6, margin: 0 }}>
        {node.rationale}
      </p>
    </>
  );
}

function ComponentDetail({ node }: { node: BlueprintNode & { node_type: 'component' } }) {
  return (
    <>
      <div style={sectionHeaderStyle}>description</div>
      <p style={{ fontSize: '12px', color: 'var(--text-primary)', lineHeight: 1.6, margin: 0 }}>
        {node.description}
      </p>

      {node.responsibilities.length > 0 && (
        <>
          <div style={{ ...sectionHeaderStyle, marginTop: '12px' }}>responsibilities</div>
          <ul style={{ margin: 0, paddingLeft: '16px' }}>
            {node.responsibilities.map((r, i) => (
              <li key={i} style={{ fontSize: '11px', color: 'var(--text-primary)', padding: '2px 0' }}>{r}</li>
            ))}
          </ul>
        </>
      )}

      {node.interfaces.length > 0 && (
        <>
          <div style={{ ...sectionHeaderStyle, marginTop: '12px' }}>interfaces</div>
          {node.interfaces.map((iface, i) => (
            <div key={i} style={{
              padding: '6px 8px', background: 'var(--bg-primary)',
              border: '1px solid var(--border)', borderRadius: '3px', marginBottom: '4px',
            }}>
              <span style={{ fontSize: '11px', fontWeight: 600, color: 'var(--text-primary)' }}>{iface.name}</span>
              <span style={{ fontSize: '10px', color: 'var(--text-secondary)', marginLeft: '6px' }}>
                {iface.direction} · {iface.protocol}
              </span>
            </div>
          ))}
        </>
      )}
    </>
  );
}

function ConstraintDetail({ node }: { node: BlueprintNode & { node_type: 'constraint' } }) {
  return (
    <>
      <div style={{ display: 'flex', gap: '8px', alignItems: 'center', marginBottom: '8px' }}>
        <span style={tagStyle}>{node.source}</span>
        <span style={{
          ...tagStyle,
          borderColor: node.negotiable ? 'rgba(109,170,69,0.4)' : 'rgba(187,101,59,0.4)',
          color: node.negotiable ? '#6daa45' : '#bb653b',
        }}>
          {node.negotiable ? 'negotiable' : 'non-negotiable'}
        </span>
      </div>
      <div style={sectionHeaderStyle}>description</div>
      <p style={{ fontSize: '12px', color: 'var(--text-primary)', lineHeight: 1.6, margin: 0 }}>
        {node.description}
      </p>
    </>
  );
}

function PatternDetail({ node }: { node: BlueprintNode & { node_type: 'pattern' } }) {
  return (
    <>
      <span style={tagStyle}>{node.scope}</span>
      <div style={{ ...sectionHeaderStyle, marginTop: '10px' }}>description</div>
      <p style={{ fontSize: '12px', color: 'var(--text-primary)', lineHeight: 1.6, margin: 0 }}>
        {node.description}
      </p>
    </>
  );
}

function QualityDetail({ node }: { node: BlueprintNode & { node_type: 'quality_requirement' } }) {
  return (
    <>
      <div style={sectionHeaderStyle}>attribute</div>
      <p style={{ fontSize: '12px', color: 'var(--text-primary)', margin: '0 0 8px' }}>{node.attribute}</p>

      <div style={sectionHeaderStyle}>scenario</div>
      <p style={{ fontSize: '12px', color: 'var(--text-primary)', lineHeight: 1.6, margin: '0 0 8px' }}>
        {node.scenario}
      </p>

      <div style={{ display: 'flex', gap: '12px' }}>
        <div>
          <div style={sectionHeaderStyle}>measure</div>
          <span style={{ fontSize: '11px', color: 'var(--text-primary)' }}>{node.measure}</span>
        </div>
        <div>
          <div style={sectionHeaderStyle}>target</div>
          <span style={{ fontSize: '11px', color: '#6daa45' }}>{node.target}</span>
        </div>
      </div>
    </>
  );
}

function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, string> = {
    proposed: '#e8af34',
    accepted: '#6daa45',
    deprecated: '#8a8987',
    superseded: '#bb653b',
  };
  return (
    <span style={{
      ...tagStyle,
      borderColor: `${colors[status] ?? '#8a8987'}66`,
      color: colors[status] ?? '#8a8987',
    }}>
      {status}
    </span>
  );
}

function RingBadge({ ring }: { ring: string }) {
  const colors: Record<string, string> = {
    adopt: '#6daa45',
    trial: '#5591c7',
    assess: '#e8af34',
    hold: '#bb653b',
  };
  return (
    <span style={{
      ...tagStyle,
      borderColor: `${colors[ring] ?? '#8a8987'}66`,
      color: colors[ring] ?? '#8a8987',
    }}>
      {ring}
    </span>
  );
}

// ─── Impact Preview ─────────────────────────────────────────────────────────

function ImpactPreviewSection({ report }: { report: ImpactReport }) {
  if (report.entries.length === 0) {
    return (
      <div style={{ fontSize: '11px', color: 'var(--text-secondary)', padding: '8px 0' }}>
        no downstream impact detected
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
      {/* Summary counts */}
      <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap', marginBottom: '4px' }}>
        {Object.entries(report.summary).map(([action, count]) => (
          <span key={action} style={{
            ...tagStyle,
            fontSize: '9px',
            fontWeight: 700,
          }}>
            {ACTION_LABELS[action] ?? action}: {count}
          </span>
        ))}
      </div>

      {/* Entries */}
      {report.entries.map((entry: ImpactEntry, i: number) => {
        const sev = SEVERITY_COLORS[entry.severity] ?? SEVERITY_COLORS.shallow;
        return (
          <div key={i} style={{
            padding: '8px 10px',
            background: sev.bg,
            border: `1px solid ${sev.border}`,
            borderRadius: '3px',
          }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginBottom: '4px' }}>
              <span style={{
                fontSize: '9px', fontWeight: 700, letterSpacing: '0.06em',
                textTransform: 'uppercase' as const, color: sev.text,
              }}>
                {ACTION_LABELS[entry.action] ?? entry.action}
              </span>
              <span style={{
                fontSize: '9px', fontWeight: 600, color: sev.text,
                opacity: 0.7, textTransform: 'uppercase' as const,
              }}>
                {entry.severity}
              </span>
            </div>
            <div style={{ fontSize: '11px', color: 'var(--text-primary)', fontWeight: 600 }}>
              {entry.node_name}
              <span style={{ fontWeight: 400, color: 'var(--text-secondary)', marginLeft: '6px', fontSize: '10px' }}>
                {entry.node_type}
              </span>
            </div>
            <div style={{ fontSize: '11px', color: 'var(--text-secondary)', marginTop: '2px', lineHeight: 1.5 }}>
              {entry.explanation}
            </div>
          </div>
        );
      })}
    </div>
  );
}

// ─── Main Panel ─────────────────────────────────────────────────────────────

interface NodeDetailPanelProps {
  nodeId: string;
  edges: EdgePayload[];
  api: ApiClient;
  onClose: () => void;
}

export default function NodeDetailPanel({ nodeId, edges, api, onClose }: NodeDetailPanelProps) {
  const [node, setNode] = useState<BlueprintNode | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Impact preview state
  const [impactDesc, setImpactDesc] = useState('');
  const [impactReport, setImpactReport] = useState<ImpactReport | null>(null);
  const [impactLoading, setImpactLoading] = useState(false);

  // Fetch full node
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    setNode(null);
    setImpactReport(null);

    api.getBlueprintNode(nodeId)
      .then(data => { if (!cancelled) setNode(data); })
      .catch(err => { if (!cancelled) setError(err instanceof Error ? err.message : String(err)); })
      .finally(() => { if (!cancelled) setLoading(false); });

    return () => { cancelled = true; };
  }, [nodeId, api]);

  // Connected edges
  const connectedEdges = useMemo(
    () => edges.filter(e => e.source === nodeId || e.target === nodeId),
    [edges, nodeId],
  );

  const handleImpactPreview = async () => {
    if (!impactDesc.trim()) return;
    setImpactLoading(true);
    try {
      const report = await api.impactPreview(nodeId, impactDesc.trim());
      setImpactReport(report);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setImpactLoading(false);
    }
  };

  if (loading) {
    return (
      <div style={panelStyle}>
        <PanelHeader onClose={onClose} title="loading…" color="var(--text-secondary)" />
        <div style={{ padding: '20px', color: 'var(--text-secondary)', fontSize: '12px' }}>
          fetching node details…
        </div>
      </div>
    );
  }

  if (error || !node) {
    return (
      <div style={panelStyle}>
        <PanelHeader onClose={onClose} title="error" color="var(--accent-red)" />
        <div style={{ padding: '12px', color: 'var(--accent-red)', fontSize: '12px' }}>
          {error ?? 'node not found'}
        </div>
      </div>
    );
  }

  const typeColor = TYPE_COLORS[node.node_type] ?? '#8a8987';
  const nodeName = 'title' in node ? node.title
    : 'name' in node ? node.name
    : 'scenario' in node ? node.scenario
    : nodeId;

  // Get tags from any node type
  const tags: string[] = 'tags' in node ? (node as { tags: string[] }).tags : [];

  return (
    <div style={panelStyle}>
      <PanelHeader onClose={onClose} title={nodeName} color={typeColor} subtitle={node.node_type} />

      <div style={{ flex: 1, overflow: 'auto', padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: '8px' }}>
        {/* Tags */}
        {tags.length > 0 && (
          <div style={{ display: 'flex', gap: '4px', flexWrap: 'wrap' }}>
            {tags.map((t, i) => <span key={i} style={tagStyle}>{t}</span>)}
          </div>
        )}

        {/* Node-type-specific detail */}
        {node.node_type === 'decision' && <DecisionDetail node={node} />}
        {node.node_type === 'technology' && <TechnologyDetail node={node} />}
        {node.node_type === 'component' && <ComponentDetail node={node} />}
        {node.node_type === 'constraint' && <ConstraintDetail node={node} />}
        {node.node_type === 'pattern' && <PatternDetail node={node} />}
        {node.node_type === 'quality_requirement' && <QualityDetail node={node} />}

        {/* Connected edges */}
        {connectedEdges.length > 0 && (
          <>
            <div style={{ ...sectionHeaderStyle, marginTop: '12px' }}>
              edges ({connectedEdges.length})
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
              {connectedEdges.map((e, i) => (
                <div key={i} style={{
                  fontSize: '10px', color: 'var(--text-secondary)', fontFamily: 'monospace',
                  padding: '3px 6px', background: 'var(--bg-primary)', borderRadius: '2px',
                  border: '1px solid var(--border)',
                }}>
                  {e.source === nodeId ? '→' : '←'}{' '}
                  <span style={{ color: 'var(--text-primary)' }}>
                    {e.source === nodeId ? e.target : e.source}
                  </span>
                  {' '}
                  <span style={{ opacity: 0.6 }}>{e.edge_type}</span>
                </div>
              ))}
            </div>
          </>
        )}

        {/* Impact Preview */}
        <div style={{ ...sectionHeaderStyle, marginTop: '16px' }}>
          impact preview
        </div>
        <div style={{ display: 'flex', gap: '6px' }}>
          <input
            type="text"
            placeholder="describe a proposed change…"
            value={impactDesc}
            onChange={e => setImpactDesc(e.target.value)}
            onKeyDown={e => { if (e.key === 'Enter') void handleImpactPreview(); }}
            style={{
              flex: 1,
              background: 'var(--bg-primary)',
              border: '1px solid var(--border)',
              borderRadius: '3px',
              padding: '6px 8px',
              fontSize: '11px',
              color: 'var(--text-primary)',
              outline: 'none',
              fontFamily: 'inherit',
            }}
          />
          <button
            onClick={() => void handleImpactPreview()}
            disabled={impactLoading || !impactDesc.trim()}
            style={{
              background: 'transparent',
              border: `1px solid ${typeColor}`,
              color: typeColor,
              padding: '5px 12px',
              fontSize: '10px',
              fontWeight: 700,
              letterSpacing: '0.05em',
              textTransform: 'uppercase' as const,
              cursor: impactLoading || !impactDesc.trim() ? 'not-allowed' : 'pointer',
              borderRadius: '3px',
              fontFamily: 'inherit',
              opacity: impactLoading || !impactDesc.trim() ? 0.4 : 1,
              whiteSpace: 'nowrap',
            }}
          >
            {impactLoading ? '…' : 'analyze'}
          </button>
        </div>

        {impactReport && <ImpactPreviewSection report={impactReport} />}

        {/* Timestamps */}
        <div style={{ marginTop: '16px', fontSize: '10px', color: 'var(--text-secondary)', opacity: 0.6 }}>
          {'created_at' in node && <div>created: {(node as { created_at: string }).created_at}</div>}
          {'updated_at' in node && <div>updated: {(node as { updated_at: string }).updated_at}</div>}
        </div>
      </div>
    </div>
  );
}

// ─── Panel header ───────────────────────────────────────────────────────────

function PanelHeader({ onClose, title, color, subtitle }: { onClose: () => void; title: string; color: string; subtitle?: string }) {
  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      padding: '10px 14px',
      borderBottom: '1px solid var(--border)',
      flexShrink: 0,
    }}>
      <div style={{ minWidth: 0, flex: 1 }}>
        <div style={{
          fontSize: '13px', fontWeight: 600, color,
          overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
        }}>
          {title}
        </div>
        {subtitle && (
          <div style={{ fontSize: '10px', color: 'var(--text-secondary)', letterSpacing: '0.06em', textTransform: 'uppercase' as const }}>
            {subtitle}
          </div>
        )}
      </div>
      <button
        onClick={onClose}
        style={{
          background: 'transparent',
          border: 'none',
          color: 'var(--text-secondary)',
          fontSize: '16px',
          cursor: 'pointer',
          padding: '4px 8px',
          lineHeight: 1,
          flexShrink: 0,
        }}
        aria-label="Close panel"
      >
        ✕
      </button>
    </div>
  );
}

// ─── Panel container style ──────────────────────────────────────────────────

const panelStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  height: '100%',
  background: 'var(--bg-secondary)',
  borderLeft: '1px solid var(--border)',
  overflow: 'hidden',
};
