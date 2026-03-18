import { useMemo } from 'react';
import type { EdgePayload, NodeSummary } from '../types/blueprint.ts';

interface DependenciesViewProps {
  nodes: NodeSummary[];
  edges: EdgePayload[];
  selectedNodeId: string | null;
  onSelectNode: (nodeId: string) => void;
}

type ComponentDependency = {
  component: NodeSummary;
  dependsOn: NodeSummary[];
  usedTechnologies: NodeSummary[];
  incoming: NodeSummary[];
};

export default function DependenciesView({
  nodes,
  edges,
  selectedNodeId,
  onSelectNode,
}: DependenciesViewProps) {
  const nodeById = useMemo(() => new Map(nodes.map(node => [node.id, node])), [nodes]);

  const dependencyData = useMemo<ComponentDependency[]>(() => {
    return nodes
      .filter(node => node.node_type === 'component')
      .map(component => {
        const dependsOn = edges
          .filter(edge => edge.source === component.id && edge.edge_type === 'depends_on')
          .map(edge => nodeById.get(edge.target))
          .filter((value): value is NodeSummary => Boolean(value));
        const usedTechnologies = edges
          .filter(edge => edge.source === component.id && edge.edge_type === 'uses')
          .map(edge => nodeById.get(edge.target))
          .filter((value): value is NodeSummary => Boolean(value));
        const incoming = edges
          .filter(edge => edge.target === component.id && edge.edge_type === 'depends_on')
          .map(edge => nodeById.get(edge.source))
          .filter((value): value is NodeSummary => Boolean(value));

        return { component, dependsOn, usedTechnologies, incoming };
      })
      .sort((left, right) => {
        const leftScore = left.dependsOn.length + left.usedTechnologies.length + left.incoming.length;
        const rightScore = right.dependsOn.length + right.usedTechnologies.length + right.incoming.length;
        return rightScore - leftScore || left.component.name.localeCompare(right.component.name);
      });
  }, [edges, nodeById, nodes]);

  const technologyConsumers = useMemo(() => {
    return nodes
      .filter(node => node.node_type === 'technology')
      .map(technology => {
        const consumers = edges
          .filter(edge => edge.target === technology.id && edge.edge_type === 'uses')
          .map(edge => nodeById.get(edge.source))
          .filter((value): value is NodeSummary => Boolean(value));
        return { technology, consumers };
      })
      .filter(entry => entry.consumers.length > 0)
      .sort((left, right) => right.consumers.length - left.consumers.length || left.technology.name.localeCompare(right.technology.name));
  }, [edges, nodeById, nodes]);

  const isolatedComponents = useMemo(
    () =>
      dependencyData.filter(entry =>
        entry.dependsOn.length === 0
        && entry.usedTechnologies.length === 0
        && entry.incoming.length === 0,
      ),
    [dependencyData],
  );

  const stats = useMemo(() => ({
    componentDependencies: edges.filter(edge => edge.edge_type === 'depends_on').length,
    technologyUsages: edges.filter(edge => edge.edge_type === 'uses').length,
    connectedComponents: dependencyData.filter(entry =>
      entry.dependsOn.length > 0 || entry.usedTechnologies.length > 0 || entry.incoming.length > 0,
    ).length,
    isolatedComponents: isolatedComponents.length,
  }), [dependencyData, edges, isolatedComponents.length]);

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
          { label: 'Component Dependencies', value: stats.componentDependencies, tone: 'var(--color-primary)' },
          { label: 'Technology Usages', value: stats.technologyUsages, tone: 'var(--color-blue)' },
          { label: 'Connected Components', value: stats.connectedComponents, tone: 'var(--color-success)' },
          { label: 'Isolated Components', value: stats.isolatedComponents, tone: 'var(--color-warning)' },
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
          background: 'linear-gradient(135deg, color-mix(in srgb, var(--color-surface) 92%, var(--color-blue) 8%), var(--color-surface))',
          padding: 'var(--space-4)',
          display: 'flex',
          flexDirection: 'column',
          gap: 'var(--space-2)',
        }}
      >
        <div style={{ fontWeight: 600, color: 'var(--color-text)' }}>
          Dependencies belong in a readable list before they belong on a canvas.
        </div>
        <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-muted)' }}>
          Use this view to inspect upstream and downstream coupling, then open the map only if you need spatial exploration.
        </div>
      </section>

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
          <h2 style={{ margin: 0, fontSize: '1rem', color: 'var(--color-text)' }}>Dependency Ledger</h2>
          <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)' }}>
            Component-to-component and component-to-technology links
          </span>
        </div>

        {dependencyData.length > 0 ? (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            {dependencyData.map(entry => (
              <button
                key={entry.component.id}
                type="button"
                onClick={() => onSelectNode(entry.component.id)}
                style={{
                  display: 'grid',
                  gridTemplateColumns: 'minmax(180px, 1fr) minmax(180px, 1fr) minmax(180px, 1fr) minmax(140px, 0.75fr)',
                  gap: 'var(--space-2)',
                  alignItems: 'start',
                  border: selectedNodeId === entry.component.id
                    ? '1px solid color-mix(in srgb, var(--color-primary) 70%, white 30%)'
                    : '1px solid var(--color-border)',
                  borderRadius: 'var(--radius-md)',
                  background: selectedNodeId === entry.component.id
                    ? 'color-mix(in srgb, var(--color-surface) 88%, var(--color-primary) 12%)'
                    : 'color-mix(in srgb, var(--color-surface) 94%, var(--color-bg) 6%)',
                  padding: '12px',
                  textAlign: 'left',
                  cursor: 'pointer',
                }}
              >
                <div>
                  <div style={{ fontSize: '0.6875rem', fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase', color: 'var(--color-text-faint)' }}>
                    Component
                  </div>
                  <div style={{ fontSize: 'var(--text-sm)', fontWeight: 600, color: 'var(--color-text)', marginTop: '6px' }}>
                    {entry.component.name}
                  </div>
                </div>

                <div>
                  <div style={{ fontSize: '0.6875rem', fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase', color: 'var(--color-text-faint)' }}>
                    Depends on
                  </div>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px', marginTop: '6px' }}>
                    {entry.dependsOn.length > 0 ? entry.dependsOn.map(node => (
                      <span key={`${entry.component.id}-${node.id}`} style={{ fontSize: '0.6875rem', color: 'var(--color-text-muted)', border: '1px solid var(--color-border)', borderRadius: 'var(--radius-full)', padding: '3px 8px' }}>
                        {node.name}
                      </span>
                    )) : <span style={{ fontSize: '0.75rem', color: 'var(--color-text-faint)' }}>None</span>}
                  </div>
                </div>

                <div>
                  <div style={{ fontSize: '0.6875rem', fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase', color: 'var(--color-text-faint)' }}>
                    Uses tech
                  </div>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px', marginTop: '6px' }}>
                    {entry.usedTechnologies.length > 0 ? entry.usedTechnologies.map(node => (
                      <span key={`${entry.component.id}-${node.id}`} style={{ fontSize: '0.6875rem', color: 'var(--color-text-muted)', border: '1px solid var(--color-border)', borderRadius: 'var(--radius-full)', padding: '3px 8px' }}>
                        {node.name}
                      </span>
                    )) : <span style={{ fontSize: '0.75rem', color: 'var(--color-text-faint)' }}>None</span>}
                  </div>
                </div>

                <div>
                  <div style={{ fontSize: '0.6875rem', fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase', color: 'var(--color-text-faint)' }}>
                    Incoming
                  </div>
                  <div style={{ fontSize: '1rem', fontWeight: 700, color: 'var(--color-text)', marginTop: '6px' }}>
                    {entry.incoming.length}
                  </div>
                </div>
              </button>
            ))}
          </div>
        ) : (
          <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-faint)' }}>
            No components in the current filtered view.
          </div>
        )}
      </section>

      <section style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))', gap: 'var(--space-3)' }}>
        <div
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
            <h2 style={{ margin: 0, fontSize: '1rem', color: 'var(--color-text)' }}>Technology Footprint</h2>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)' }}>
              Consumer count by technology
            </span>
          </div>

          {technologyConsumers.length > 0 ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              {technologyConsumers.map(entry => (
                <button
                  key={entry.technology.id}
                  type="button"
                  onClick={() => onSelectNode(entry.technology.id)}
                  style={{
                    border: selectedNodeId === entry.technology.id
                      ? '1px solid color-mix(in srgb, var(--color-primary) 70%, white 30%)'
                      : '1px solid var(--color-border)',
                    borderRadius: 'var(--radius-md)',
                    background: selectedNodeId === entry.technology.id
                      ? 'color-mix(in srgb, var(--color-surface) 88%, var(--color-primary) 12%)'
                      : 'color-mix(in srgb, var(--color-surface) 94%, var(--color-bg) 6%)',
                    padding: '12px',
                    textAlign: 'left',
                    cursor: 'pointer',
                  }}
                >
                  <div style={{ display: 'flex', justifyContent: 'space-between', gap: 'var(--space-2)', alignItems: 'baseline' }}>
                    <div style={{ fontSize: 'var(--text-sm)', fontWeight: 600, color: 'var(--color-text)' }}>
                      {entry.technology.name}
                    </div>
                    <div style={{ fontSize: '0.75rem', color: 'var(--color-primary)', fontWeight: 700 }}>
                      {entry.consumers.length} consumer{entry.consumers.length === 1 ? '' : 's'}
                    </div>
                  </div>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px', marginTop: '10px' }}>
                    {entry.consumers.slice(0, 5).map(node => (
                      <span key={`${entry.technology.id}-${node.id}`} style={{ fontSize: '0.6875rem', color: 'var(--color-text-muted)', border: '1px solid var(--color-border)', borderRadius: 'var(--radius-full)', padding: '3px 8px' }}>
                        {node.name}
                      </span>
                    ))}
                  </div>
                </button>
              ))}
            </div>
          ) : (
            <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-faint)' }}>
              No technology usage edges in the current filtered view.
            </div>
          )}
        </div>

        <div
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
            <h2 style={{ margin: 0, fontSize: '1rem', color: 'var(--color-text)' }}>Low-Coupling Targets</h2>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)' }}>
              Components with no current dependency surface
            </span>
          </div>

          {isolatedComponents.length > 0 ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              {isolatedComponents.slice(0, 8).map(entry => (
                <button
                  key={entry.component.id}
                  type="button"
                  onClick={() => onSelectNode(entry.component.id)}
                  style={{
                    border: selectedNodeId === entry.component.id
                      ? '1px solid color-mix(in srgb, var(--color-primary) 70%, white 30%)'
                      : '1px solid var(--color-border)',
                    borderRadius: 'var(--radius-md)',
                    background: selectedNodeId === entry.component.id
                      ? 'color-mix(in srgb, var(--color-surface) 88%, var(--color-primary) 12%)'
                      : 'color-mix(in srgb, var(--color-surface) 94%, var(--color-warning) 6%)',
                    padding: '12px',
                    textAlign: 'left',
                    cursor: 'pointer',
                  }}
                >
                  <div style={{ fontSize: 'var(--text-sm)', fontWeight: 600, color: 'var(--color-text)' }}>
                    {entry.component.name}
                  </div>
                  <div style={{ fontSize: '0.75rem', color: 'var(--color-text-muted)', marginTop: '6px' }}>
                    No `depends_on` or `uses` links yet
                  </div>
                </button>
              ))}
            </div>
          ) : (
            <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-faint)' }}>
              Every visible component participates in at least one dependency relationship.
            </div>
          )}
        </div>
      </section>
    </div>
  );
}
