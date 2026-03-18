import { useMemo } from 'react';
import { labelNodeType } from '../lib/taxonomy.ts';
import type { EdgePayload, NodeSummary } from '../types/blueprint.ts';

interface TraceabilityViewProps {
  nodes: NodeSummary[];
  edges: EdgePayload[];
  selectedNodeId: string | null;
  onSelectNode: (nodeId: string) => void;
}

type TraceEntry = {
  node: NodeSummary;
  linked: NodeSummary[];
  linkLabel: string;
  isOrphan: boolean;
};

function cardButtonStyle(selected: boolean) {
  return {
    border: selected
      ? '1px solid color-mix(in srgb, var(--color-primary) 70%, white 30%)'
      : '1px solid var(--color-border)',
    borderRadius: 'var(--radius-md)',
    background: selected
      ? 'color-mix(in srgb, var(--color-surface) 86%, var(--color-primary) 14%)'
      : 'color-mix(in srgb, var(--color-surface) 94%, var(--color-bg) 6%)',
    padding: '12px',
    textAlign: 'left' as const,
    cursor: 'pointer',
  };
}

export default function TraceabilityView({
  nodes,
  edges,
  selectedNodeId,
  onSelectNode,
}: TraceabilityViewProps) {
  const nodeById = useMemo(() => new Map(nodes.map(node => [node.id, node])), [nodes]);

  const semanticEdges = useMemo(
    () =>
      edges.filter(edge =>
        edge.edge_type === 'constrains'
        || edge.edge_type === 'satisfies'
        || edge.edge_type === 'decided_by'
        || edge.edge_type === 'affects'
        || edge.edge_type === 'implements',
      ),
    [edges],
  );

  const linkedByNode = useMemo(() => {
    const map = new Map<string, EdgePayload[]>();
    for (const edge of semanticEdges) {
      const sourceEdges = map.get(edge.source) ?? [];
      sourceEdges.push(edge);
      map.set(edge.source, sourceEdges);

      const targetEdges = map.get(edge.target) ?? [];
      targetEdges.push(edge);
      map.set(edge.target, targetEdges);
    }
    return map;
  }, [semanticEdges]);

  const constraints = useMemo<TraceEntry[]>(() => {
    return nodes
      .filter(node => node.node_type === 'constraint')
      .map(node => {
        const linked = semanticEdges
          .filter(edge => edge.source === node.id && edge.edge_type === 'constrains')
          .map(edge => nodeById.get(edge.target))
          .filter((value): value is NodeSummary => Boolean(value));

        return {
          node,
          linked,
          linkLabel: 'Constrained items',
          isOrphan: linked.length === 0,
        };
      })
      .sort((left, right) => Number(left.isOrphan) - Number(right.isOrphan) || right.linked.length - left.linked.length || left.node.name.localeCompare(right.node.name));
  }, [nodeById, nodes, semanticEdges]);

  const qualityRequirements = useMemo<TraceEntry[]>(() => {
    return nodes
      .filter(node => node.node_type === 'quality_requirement')
      .map(node => {
        const linked = semanticEdges
          .filter(edge => edge.target === node.id && edge.edge_type === 'satisfies')
          .map(edge => nodeById.get(edge.source))
          .filter((value): value is NodeSummary => Boolean(value));

        return {
          node,
          linked,
          linkLabel: 'Satisfied by',
          isOrphan: linked.length === 0,
        };
      })
      .sort((left, right) => Number(left.isOrphan) - Number(right.isOrphan) || right.linked.length - left.linked.length || left.node.name.localeCompare(right.node.name));
  }, [nodeById, nodes, semanticEdges]);

  const decisions = useMemo<TraceEntry[]>(() => {
    return nodes
      .filter(node => node.node_type === 'decision')
      .map(node => {
        const linked = semanticEdges
          .filter(edge =>
            (edge.target === node.id && edge.edge_type === 'decided_by')
            || (edge.source === node.id && (edge.edge_type === 'affects' || edge.edge_type === 'satisfies')),
          )
          .map(edge => nodeById.get(edge.source === node.id ? edge.target : edge.source))
          .filter((value): value is NodeSummary => Boolean(value));

        return {
          node,
          linked,
          linkLabel: 'Connected architecture',
          isOrphan: linked.length === 0,
        };
      })
      .sort((left, right) => Number(left.isOrphan) - Number(right.isOrphan) || right.linked.length - left.linked.length || left.node.name.localeCompare(right.node.name));
  }, [nodeById, nodes, semanticEdges]);

  const orphanNodes = useMemo(
    () =>
      nodes.filter(node =>
        (node.node_type === 'constraint'
          || node.node_type === 'quality_requirement'
          || node.node_type === 'decision')
        && (linkedByNode.get(node.id)?.length ?? 0) === 0,
      ),
    [linkedByNode, nodes],
  );

  const traceabilityStats = useMemo(() => {
    const totalTraceNodes = constraints.length + qualityRequirements.length + decisions.length;
    const linkedTraceNodes = [...constraints, ...qualityRequirements, ...decisions]
      .filter(entry => !entry.isOrphan)
      .length;
    return {
      semanticEdges: semanticEdges.length,
      linkedTraceNodes,
      totalTraceNodes,
      orphanNodes: orphanNodes.length,
    };
  }, [constraints, decisions, orphanNodes.length, qualityRequirements, semanticEdges.length]);

  const sections: { title: string; subtitle: string; entries: TraceEntry[] }[] = [
    { title: 'Constraint Coverage', subtitle: 'What currently limits architecture choices', entries: constraints },
    { title: 'Quality Goals', subtitle: 'Which components or decisions satisfy quality scenarios', entries: qualityRequirements },
    { title: 'Decision Trace', subtitle: 'Decision links into architecture and quality', entries: decisions },
  ];

  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        overflowY: 'auto',
        overscrollBehavior: 'contain',
        padding: 'var(--space-5)',
        display: 'flex',
        flexDirection: 'column',
        gap: 'var(--space-5)',
      }}
    >
      <section
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(190px, 1fr))',
          gap: 'var(--space-3)',
        }}
      >
        {[
          { label: 'Semantic Links', value: traceabilityStats.semanticEdges, tone: 'var(--color-primary)' },
          { label: 'Linked Trace Nodes', value: traceabilityStats.linkedTraceNodes, tone: 'var(--color-success)' },
          { label: 'Traceability Scope', value: traceabilityStats.totalTraceNodes, tone: 'var(--color-blue)' },
          { label: 'Orphans', value: traceabilityStats.orphanNodes, tone: 'var(--color-warning)' },
        ].map(stat => (
          <div
            key={stat.label}
            style={{
              border: '1px solid var(--color-border)',
              borderRadius: 'var(--radius-lg)',
              background: 'var(--color-surface)',
              padding: 'var(--space-3)',
              boxShadow: 'var(--shadow-sm)',
            }}
          >
            <div
              style={{
                fontSize: '0.6875rem',
                fontWeight: 700,
                letterSpacing: '0.08em',
                textTransform: 'uppercase',
                color: 'var(--color-text-faint)',
                marginBottom: 'var(--space-2)',
              }}
            >
              {stat.label}
            </div>
            <div style={{ fontSize: '1.4rem', fontWeight: 700, color: stat.tone }}>{stat.value}</div>
          </div>
        ))}
      </section>

      <section
        style={{
          border: '1px solid var(--color-border)',
          borderRadius: 'var(--radius-lg)',
          background: 'linear-gradient(135deg, color-mix(in srgb, var(--color-surface) 90%, var(--color-primary) 10%), var(--color-surface))',
          padding: 'var(--space-4)',
          display: 'flex',
          flexDirection: 'column',
          gap: 'var(--space-2)',
        }}
      >
        <div style={{ fontWeight: 600, color: 'var(--color-text)' }}>
          Traceability is where blueprint knowledge becomes actionable.
        </div>
        <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-muted)' }}>
          Review constraints, quality goals, and decisions here first. Use the map only when you need to inspect spatial topology.
        </div>
      </section>

      {orphanNodes.length > 0 && (
        <section
          style={{
            border: '1px solid var(--color-border)',
            borderRadius: 'var(--radius-lg)',
            background: 'var(--color-surface)',
            boxShadow: 'var(--shadow-sm)',
            padding: 'var(--space-3)',
            display: 'flex',
            flexDirection: 'column',
            gap: 'var(--space-3)',
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', gap: 'var(--space-3)', alignItems: 'baseline' }}>
            <h2 style={{ margin: 0, fontSize: '1rem', color: 'var(--color-text)' }}>Unlinked Knowledge</h2>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)' }}>
              Nodes missing semantic relationships
            </span>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(260px, 1fr))', gap: 'var(--space-3)' }}>
            {orphanNodes.slice(0, 9).map(node => (
              <button
                key={node.id}
                type="button"
                onClick={() => onSelectNode(node.id)}
                style={cardButtonStyle(selectedNodeId === node.id)}
              >
                <div style={{ fontSize: '0.6875rem', color: 'var(--color-warning)', fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase' }}>
                  {labelNodeType(node.node_type, 'short')}
                </div>
                <div style={{ fontSize: 'var(--text-sm)', fontWeight: 600, color: 'var(--color-text)', marginTop: '4px' }}>
                  {node.name}
                </div>
                <div style={{ fontSize: '0.75rem', color: 'var(--color-text-muted)', marginTop: '6px' }}>
                  No semantic relationships yet
                </div>
              </button>
            ))}
          </div>
        </section>
      )}

      {sections.map(section => (
        <section key={section.title} style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-3)' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', gap: 'var(--space-3)', alignItems: 'baseline' }}>
            <div>
              <h2 style={{ margin: 0, fontSize: '1rem', color: 'var(--color-text)' }}>{section.title}</h2>
              <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)', marginTop: '4px' }}>
                {section.subtitle}
              </div>
            </div>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)' }}>
              {section.entries.length} item{section.entries.length === 1 ? '' : 's'}
            </span>
          </div>

          {section.entries.length > 0 ? (
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))', gap: 'var(--space-3)' }}>
              {section.entries.map(entry => (
                <button
                  key={entry.node.id}
                  type="button"
                  onClick={() => onSelectNode(entry.node.id)}
                  style={cardButtonStyle(selectedNodeId === entry.node.id)}
                >
                  <div style={{ display: 'flex', justifyContent: 'space-between', gap: 'var(--space-2)', alignItems: 'baseline' }}>
                    <span style={{ fontSize: '0.6875rem', fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase', color: entry.isOrphan ? 'var(--color-warning)' : 'var(--color-primary)' }}>
                      {entry.isOrphan ? 'Needs link' : entry.linkLabel}
                    </span>
                    <span style={{ fontSize: '0.6875rem', color: 'var(--color-text-faint)' }}>
                      {entry.linked.length}
                    </span>
                  </div>
                  <div style={{ fontSize: 'var(--text-sm)', fontWeight: 600, color: 'var(--color-text)', marginTop: '6px' }}>
                    {entry.node.name}
                  </div>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px', marginTop: '10px' }}>
                    {entry.linked.length > 0 ? entry.linked.slice(0, 4).map(linkedNode => (
                      <span
                        key={`${entry.node.id}-${linkedNode.id}`}
                        style={{
                          fontSize: '0.6875rem',
                          color: 'var(--color-text-muted)',
                          border: '1px solid var(--color-border)',
                          borderRadius: 'var(--radius-full)',
                          padding: '3px 8px',
                        }}
                      >
                        {linkedNode.name}
                      </span>
                    )) : (
                      <span style={{ fontSize: '0.75rem', color: 'var(--color-text-muted)' }}>
                        No semantic relationships yet
                      </span>
                    )}
                  </div>
                </button>
              ))}
            </div>
          ) : (
            <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-faint)' }}>
              No {section.title.toLowerCase()} in the current filtered view.
            </div>
          )}
        </section>
      ))}
    </div>
  );
}
