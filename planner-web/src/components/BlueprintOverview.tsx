import { useMemo } from 'react';
import { labelNodeType, labelScopeClass, labelScopeVisibility } from '../lib/taxonomy.ts';
import type { EdgePayload, NodeSummary, NodeType } from '../types/blueprint.ts';

interface BlueprintOverviewProps {
  nodes: NodeSummary[];
  edges: EdgePayload[];
  selectedNodeId: string | null;
  onSelectNode: (nodeId: string) => void;
}

const NODE_GROUPS: { type: NodeType; icon: string }[] = [
  { type: 'project', icon: '⬢' },
  { type: 'decision', icon: '◆' },
  { type: 'component', icon: '▪' },
  { type: 'constraint', icon: '◇' },
  { type: 'quality_requirement', icon: '⛨' },
  { type: 'technology', icon: '⬡' },
  { type: 'pattern', icon: '◉' },
];

type RelatedNode = {
  id: string;
  name: string;
  direction: 'incoming' | 'outgoing';
  edgeType: string;
};

type RelationshipIndex = {
  incoming: EdgePayload[];
  outgoing: EdgePayload[];
  related: RelatedNode[];
};

type ProjectStructure = {
  id: string;
  name: string;
  components: NodeSummary[];
};

function formatDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
}

export default function BlueprintOverview({
  nodes,
  edges,
  selectedNodeId,
  onSelectNode,
}: BlueprintOverviewProps) {
  const nodeById = useMemo(() => new Map(nodes.map(node => [node.id, node])), [nodes]);

  const relationships = useMemo(() => {
    const index = new Map<string, RelationshipIndex>();

    for (const node of nodes) {
      index.set(node.id, { incoming: [], outgoing: [], related: [] });
    }

    for (const edge of edges) {
      const source = index.get(edge.source);
      const target = index.get(edge.target);
      const sourceNode = nodeById.get(edge.source);
      const targetNode = nodeById.get(edge.target);

      if (source) {
        source.outgoing.push(edge);
        if (targetNode) {
          source.related.push({
            id: targetNode.id,
            name: targetNode.name,
            direction: 'outgoing',
            edgeType: edge.edge_type,
          });
        }
      }

      if (target) {
        target.incoming.push(edge);
        if (sourceNode) {
          target.related.push({
            id: sourceNode.id,
            name: sourceNode.name,
            direction: 'incoming',
            edgeType: edge.edge_type,
          });
        }
      }
    }

    return index;
  }, [edges, nodeById, nodes]);

  const groupedNodes = useMemo(() => {
    return NODE_GROUPS.map(group => {
      const groupNodes = nodes
        .filter(node => node.node_type === group.type)
        .sort((a, b) => {
          const aConnections = relationships.get(a.id);
          const bConnections = relationships.get(b.id);
          const aEdgeCount = (aConnections?.incoming.length ?? 0) + (aConnections?.outgoing.length ?? 0);
          const bEdgeCount = (bConnections?.incoming.length ?? 0) + (bConnections?.outgoing.length ?? 0);
          if (aEdgeCount !== bEdgeCount) return bEdgeCount - aEdgeCount;
          return a.name.localeCompare(b.name);
        });

      return { ...group, nodes: groupNodes };
    }).filter(group => group.nodes.length > 0);
  }, [nodes, relationships]);

  const projectStructures = useMemo<ProjectStructure[]>(() => {
    const projectNodes = nodes.filter(node => node.node_type === 'project');
    const componentNodes = nodes.filter(node => node.node_type === 'component');

    return projectNodes.map(project => {
      const containedComponentIds = edges
        .filter(edge => edge.edge_type === 'contains' && edge.source === project.id)
        .map(edge => edge.target);
      const containedComponents = componentNodes
        .filter(component => containedComponentIds.includes(component.id))
        .sort((left, right) => {
          const leftLinks = relationships.get(left.id);
          const rightLinks = relationships.get(right.id);
          const leftCount = (leftLinks?.incoming.length ?? 0) + (leftLinks?.outgoing.length ?? 0);
          const rightCount = (rightLinks?.incoming.length ?? 0) + (rightLinks?.outgoing.length ?? 0);
          if (leftCount !== rightCount) return rightCount - leftCount;
          return left.name.localeCompare(right.name);
        });

      return {
        id: project.id,
        name: project.name,
        components: containedComponents,
      };
    }).filter(project => project.components.length > 0);
  }, [edges, nodes, relationships]);

  const relationshipHighlights = useMemo(() => {
    return edges
      .filter(edge => edge.edge_type === 'depends_on' || edge.edge_type === 'uses' || edge.edge_type === 'implements')
      .map(edge => ({
        edge,
        source: nodeById.get(edge.source),
        target: nodeById.get(edge.target),
      }))
      .filter(entry => entry.source && entry.target)
      .slice(0, 12);
  }, [edges, nodeById]);

  const attentionItems = useMemo(() => {
    return nodes.filter(node => {
      if (node.node_type !== 'component') return false;
      if (node.name === 'Root The System Service') return true;
      const relation = relationships.get(node.id);
      const edgeCount = (relation?.incoming.length ?? 0) + (relation?.outgoing.length ?? 0);
      return edgeCount === 0;
    });
  }, [nodes, relationships]);

  const stats = useMemo(() => {
    const connectedNodes = nodes.filter(node => {
      const relation = relationships.get(node.id);
      return (relation?.incoming.length ?? 0) + (relation?.outgoing.length ?? 0) > 0;
    }).length;

    const documentedNodes = nodes.filter(node => node.has_documentation).length;
    const sharedNodes = nodes.filter(node => node.is_shared).length;
    const relationshipCoverage = nodes.length === 0 ? 0 : Math.round((connectedNodes / nodes.length) * 100);

    return {
      connectedNodes,
      documentedNodes,
      sharedNodes,
      relationshipCoverage,
    };
  }, [nodes, relationships]);

  if (nodes.length === 0) {
    return (
      <div
        style={{
          width: '100%',
          height: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'var(--color-text-faint)',
          fontSize: 'var(--text-sm)',
        }}
      >
        No blueprint items yet
      </div>
    );
  }

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
          gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
          gap: 'var(--space-3)',
        }}
      >
        {[
          { label: 'Blueprint Items', value: nodes.length, tone: 'var(--color-primary)' },
          { label: 'Explicit Links', value: edges.length, tone: 'var(--color-blue)' },
          { label: 'Connected Items', value: stats.connectedNodes, tone: 'var(--color-success)' },
          { label: 'Relationship Coverage', value: `${stats.relationshipCoverage}%`, tone: 'var(--color-warning)' },
          { label: 'Documented', value: stats.documentedNodes, tone: 'var(--color-purple)' },
          { label: 'Shared Records', value: stats.sharedNodes, tone: 'var(--color-gold)' },
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

      <div
        style={{
          border: '1px solid var(--color-border)',
          borderRadius: 'var(--radius-lg)',
          background: 'linear-gradient(135deg, color-mix(in srgb, var(--color-surface) 92%, var(--color-primary) 8%), var(--color-surface))',
          padding: 'var(--space-4)',
          display: 'flex',
          flexDirection: 'column',
          gap: 'var(--space-2)',
        }}
      >
        <div style={{ fontWeight: 600, color: 'var(--color-text)' }}>
          {edges.length === 0
            ? 'No explicit relationships found. Showing grouped blueprint cards instead of a relationship map.'
            : 'Overview organizes blueprint items by type, with relationship signals attached to each card.'}
        </div>
        <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-muted)' }}>
          Use this view to scan decisions, components, constraints, and quality scenarios quickly. Switch to
          {' '}Relationships only when you need topology.
        </div>
      </div>

      {projectStructures.length > 0 && (
        <section style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-3)' }}>
          <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', gap: 'var(--space-3)' }}>
            <h2 style={{ margin: 0, fontSize: '1rem', color: 'var(--color-text)' }}>Architecture Lanes</h2>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)' }}>
              Project structure without opening the graph
            </span>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))', gap: 'var(--space-3)' }}>
            {projectStructures.map(project => (
              <div
                key={project.id}
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
                <div>
                  <div style={{ fontSize: '0.625rem', fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase', color: 'var(--color-text-faint)' }}>
                    Project
                  </div>
                  <div style={{ fontSize: 'var(--text-sm)', fontWeight: 700, color: 'var(--color-text)' }}>{project.name}</div>
                </div>

                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                  {project.components.map(component => {
                    const relation = relationships.get(component.id);
                    const outgoingDeps = relation?.outgoing.filter(edge => edge.edge_type === 'depends_on').length ?? 0;
                    const incomingDeps = relation?.incoming.filter(edge => edge.edge_type === 'depends_on').length ?? 0;

                    return (
                      <button
                        key={component.id}
                        type="button"
                        onClick={() => onSelectNode(component.id)}
                        style={{
                          display: 'grid',
                          gridTemplateColumns: 'minmax(0, 1fr) auto',
                          gap: 'var(--space-2)',
                          alignItems: 'center',
                          border: selectedNodeId === component.id
                            ? '1px solid color-mix(in srgb, var(--color-primary) 70%, white 30%)'
                            : '1px solid var(--color-border)',
                          borderRadius: 'var(--radius-md)',
                          background: selectedNodeId === component.id
                            ? 'color-mix(in srgb, var(--color-surface) 88%, var(--color-primary) 12%)'
                            : 'color-mix(in srgb, var(--color-surface) 90%, var(--color-bg) 10%)',
                          padding: '10px 12px',
                          textAlign: 'left',
                          cursor: 'pointer',
                        }}
                      >
                        <div style={{ minWidth: 0 }}>
                          <div style={{ fontSize: 'var(--text-sm)', fontWeight: 600, color: 'var(--color-text)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                            {component.name}
                          </div>
                          <div style={{ fontSize: '0.6875rem', color: 'var(--color-text-muted)' }}>
                            {outgoingDeps} downstream deps · {incomingDeps} upstream deps
                          </div>
                        </div>
                        <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap', justifyContent: 'flex-end' }}>
                          {component.tags.slice(0, 2).map(tag => (
                            <span
                              key={`${component.id}-${tag}`}
                              style={{
                                fontSize: '0.625rem',
                                color: 'var(--color-text-faint)',
                                border: '1px solid var(--color-border)',
                                borderRadius: 'var(--radius-full)',
                                padding: '2px 8px',
                              }}
                            >
                              {tag}
                            </span>
                          ))}
                        </div>
                      </button>
                    );
                  })}
                </div>
              </div>
            ))}
          </div>
        </section>
      )}

      <section style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))', gap: 'var(--space-3)' }}>
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
          <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', gap: 'var(--space-3)' }}>
            <h2 style={{ margin: 0, fontSize: '1rem', color: 'var(--color-text)' }}>Relationship Highlights</h2>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)' }}>
              Dependency and implementation edges
            </span>
          </div>

          {relationshipHighlights.length > 0 ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              {relationshipHighlights.map(({ edge, source, target }) => (
                <button
                  key={`${edge.source}-${edge.target}-${edge.edge_type}`}
                  type="button"
                  onClick={() => source && onSelectNode(source.id)}
                  style={{
                    display: 'grid',
                    gridTemplateColumns: 'minmax(0, 1fr) auto minmax(0, 1fr)',
                    gap: 'var(--space-2)',
                    alignItems: 'center',
                    border: '1px solid var(--color-border)',
                    borderRadius: 'var(--radius-md)',
                    background: 'color-mix(in srgb, var(--color-surface) 92%, var(--color-bg) 8%)',
                    padding: '10px 12px',
                    cursor: 'pointer',
                    textAlign: 'left',
                  }}
                >
                  <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', fontSize: 'var(--text-sm)', color: 'var(--color-text)' }}>
                    {source?.name}
                  </span>
                  <span style={{ fontSize: '0.6875rem', color: 'var(--color-primary)', textTransform: 'uppercase', letterSpacing: '0.08em', fontWeight: 700 }}>
                    {edge.edge_type.replace(/_/g, ' ')}
                  </span>
                  <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', fontSize: 'var(--text-sm)', color: 'var(--color-text-muted)' }}>
                    {target?.name}
                  </span>
                </button>
              ))}
            </div>
          ) : (
            <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-faint)' }}>
              No dependency-style edges yet. The architecture lanes above will become more useful as `depends_on`, `uses`, and `implements` edges accumulate.
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
          <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', gap: 'var(--space-3)' }}>
            <h2 style={{ margin: 0, fontSize: '1rem', color: 'var(--color-text)' }}>Needs Attention</h2>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)' }}>
              Fast cleanup targets
            </span>
          </div>

          {attentionItems.length > 0 ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              {attentionItems.slice(0, 8).map(node => (
                <button
                  key={node.id}
                  type="button"
                  onClick={() => onSelectNode(node.id)}
                  style={{
                    border: '1px solid var(--color-border)',
                    borderRadius: 'var(--radius-md)',
                    background: 'color-mix(in srgb, var(--color-surface) 94%, var(--color-warning) 6%)',
                    padding: '10px 12px',
                    textAlign: 'left',
                    cursor: 'pointer',
                  }}
                >
                  <div style={{ fontSize: 'var(--text-sm)', fontWeight: 600, color: 'var(--color-text)' }}>
                    {node.name}
                  </div>
                  <div style={{ fontSize: '0.6875rem', color: 'var(--color-text-muted)' }}>
                    {node.name === 'Root The System Service' ? 'Legacy generated name' : 'No explicit relationships yet'}
                  </div>
                </button>
              ))}
            </div>
          ) : (
            <div style={{ fontSize: 'var(--text-sm)', color: 'var(--color-text-faint)' }}>
              No obvious cleanup hotspots in the current filtered view.
            </div>
          )}
        </div>
      </section>

      {groupedNodes.map(group => (
        <section key={group.type} style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-3)' }}>
          <div
            style={{
              display: 'flex',
              alignItems: 'baseline',
              justifyContent: 'space-between',
              gap: 'var(--space-3)',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)' }}>
              <span style={{ color: 'var(--color-text-muted)' }}>{group.icon}</span>
              <h2 style={{ margin: 0, fontSize: '1rem', color: 'var(--color-text)' }}>
                {labelNodeType(group.type, 'plural')}
              </h2>
            </div>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)' }}>
              {group.nodes.length} item{group.nodes.length === 1 ? '' : 's'}
            </span>
          </div>

          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))',
              gap: 'var(--space-3)',
            }}
          >
            {group.nodes.map(node => {
              const relation = relationships.get(node.id);
              const incoming = relation?.incoming.length ?? 0;
              const outgoing = relation?.outgoing.length ?? 0;
              const edgeCount = incoming + outgoing;
              const relatedNodes = relation?.related.slice(0, 3) ?? [];
              const isSelected = selectedNodeId === node.id;

              return (
                <button
                  key={node.id}
                  type="button"
                  onClick={() => onSelectNode(node.id)}
                  style={{
                    textAlign: 'left',
                    padding: 'var(--space-3)',
                    borderRadius: 'var(--radius-lg)',
                    border: isSelected
                      ? '1px solid color-mix(in srgb, var(--color-primary) 70%, white 30%)'
                      : '1px solid var(--color-border)',
                    background: isSelected
                      ? 'color-mix(in srgb, var(--color-surface) 90%, var(--color-primary) 10%)'
                      : 'var(--color-surface)',
                    boxShadow: isSelected ? 'var(--shadow-md)' : 'var(--shadow-sm)',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: 'var(--space-3)',
                    cursor: 'pointer',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 'var(--space-3)' }}>
                    <div style={{ minWidth: 0 }}>
                      <div
                        style={{
                          fontSize: '0.625rem',
                          fontWeight: 700,
                          letterSpacing: '0.08em',
                          textTransform: 'uppercase',
                          color: 'var(--color-text-faint)',
                          marginBottom: '4px',
                        }}
                      >
                        {labelNodeType(node.node_type, 'short')}
                      </div>
                      <div
                        style={{
                          fontSize: 'var(--text-sm)',
                          fontWeight: 600,
                          color: 'var(--color-text)',
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                        }}
                        title={node.name}
                      >
                        {node.name}
                      </div>
                    </div>

                    <span
                      style={{
                        flexShrink: 0,
                        fontSize: '0.625rem',
                        fontWeight: 700,
                        color: edgeCount > 0 ? 'var(--color-success)' : 'var(--color-error)',
                        background: edgeCount > 0 ? 'rgba(34,197,94,0.12)' : 'rgba(239,68,68,0.12)',
                        borderRadius: 'var(--radius-full)',
                        padding: '2px 8px',
                      }}
                    >
                      {edgeCount > 0 ? `${edgeCount} link${edgeCount === 1 ? '' : 's'}` : 'unlinked'}
                    </span>
                  </div>

                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px' }}>
                    <span className={`status-badge status-${node.status.toLowerCase().replace(/\s+/g, '-')}`}>{node.status}</span>
                    <span
                      style={{
                        fontSize: '0.625rem',
                        color: 'var(--color-text-muted)',
                        border: '1px solid var(--color-border)',
                        borderRadius: 'var(--radius-full)',
                        padding: '2px 8px',
                      }}
                    >
                      {labelScopeClass(node.scope_class)}
                    </span>
                    <span
                      style={{
                        fontSize: '0.625rem',
                        color: 'var(--color-text-muted)',
                        border: '1px solid var(--color-border)',
                        borderRadius: 'var(--radius-full)',
                        padding: '2px 8px',
                      }}
                    >
                      {labelScopeVisibility(node.scope_visibility, 'short')}
                    </span>
                    {node.has_documentation && (
                      <span
                        style={{
                          fontSize: '0.625rem',
                          color: 'var(--color-blue)',
                          border: '1px solid color-mix(in srgb, var(--color-blue) 50%, var(--color-border) 50%)',
                          borderRadius: 'var(--radius-full)',
                          padding: '2px 8px',
                        }}
                      >
                        docs
                      </span>
                    )}
                  </div>

                  <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, minmax(0, 1fr))', gap: '8px' }}>
                    <div>
                      <div style={{ fontSize: '0.625rem', textTransform: 'uppercase', letterSpacing: '0.08em', color: 'var(--color-text-faint)' }}>
                        Relationships
                      </div>
                      <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
                        {incoming} incoming · {outgoing} outgoing
                      </div>
                    </div>
                    <div>
                      <div style={{ fontSize: '0.625rem', textTransform: 'uppercase', letterSpacing: '0.08em', color: 'var(--color-text-faint)' }}>
                        Updated
                      </div>
                      <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
                        {formatDate(node.updated_at)}
                      </div>
                    </div>
                  </div>

                  {relatedNodes.length > 0 ? (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                      <div style={{ fontSize: '0.625rem', textTransform: 'uppercase', letterSpacing: '0.08em', color: 'var(--color-text-faint)' }}>
                        Related Items
                      </div>
                      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px' }}>
                        {relatedNodes.map(related => (
                          <span
                            key={`${node.id}-${related.id}-${related.edgeType}-${related.direction}`}
                            style={{
                              maxWidth: '100%',
                              fontSize: '0.6875rem',
                              color: 'var(--color-text-muted)',
                              border: '1px solid var(--color-border)',
                              borderRadius: 'var(--radius-full)',
                              padding: '3px 8px',
                              overflow: 'hidden',
                              textOverflow: 'ellipsis',
                              whiteSpace: 'nowrap',
                            }}
                            title={`${related.direction === 'incoming' ? 'From' : 'To'} ${related.name} (${related.edgeType.replace(/_/g, ' ')})`}
                          >
                            {related.direction === 'incoming' ? '←' : '→'} {related.name}
                          </span>
                        ))}
                        {(relation?.related.length ?? 0) > relatedNodes.length && (
                          <span style={{ fontSize: '0.6875rem', color: 'var(--color-text-faint)' }}>
                            +{(relation?.related.length ?? 0) - relatedNodes.length} more
                          </span>
                        )}
                      </div>
                    </div>
                  ) : (
                    <div style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-faint)' }}>
                      No explicit relationships yet.
                    </div>
                  )}

                  {node.tags.length > 0 && (
                    <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px' }}>
                      {node.tags.slice(0, 4).map(tag => (
                        <span
                          key={`${node.id}-${tag}`}
                          style={{
                            fontSize: '0.625rem',
                            color: 'var(--color-text-faint)',
                            background: 'color-mix(in srgb, var(--color-surface) 70%, var(--color-bg) 30%)',
                            borderRadius: 'var(--radius-full)',
                            padding: '2px 8px',
                          }}
                        >
                          {tag}
                        </span>
                      ))}
                    </div>
                  )}
                </button>
              );
            })}
          </div>
        </section>
      ))}
    </div>
  );
}
