import { useEffect, useMemo, useState } from 'react';

import type { ApiClient } from '../api/client.ts';
import type {
  BlueprintNode,
  DecisionNode,
  TechnologyNode,
  ComponentNode,
  ConstraintNode,
  PatternNode,
  QualityRequirementNode,
  EdgePayload,
} from '../types/blueprint.ts';
import { labelNodeType } from '../lib/taxonomy.ts';

interface NodeDetailPanelProps {
  nodeId: string | null;
  edges: EdgePayload[];
  api: ApiClient;
  onClose: () => void;
}

function nodeTitle(node: BlueprintNode): string {
  switch (node.node_type) {
    case 'decision':
      return node.title;
    case 'technology':
    case 'component':
    case 'pattern':
      return node.name;
    case 'constraint':
      return node.title;
    case 'quality_requirement':
      return node.scenario;
  }
}

function renderNodeBody(node: BlueprintNode) {
  switch (node.node_type) {
    case 'decision': {
      const decision = node as DecisionNode;
      return (
        <>
          <p>{decision.context}</p>
          {decision.options.length > 0 && <p>Options: {decision.options.map(option => option.name).join(', ')}</p>}
        </>
      );
    }
    case 'technology': {
      const technology = node as TechnologyNode;
      return (
        <>
          <p>{technology.rationale}</p>
          <p>{technology.category} · {technology.ring}{technology.version ? ` · v${technology.version}` : ''}</p>
        </>
      );
    }
    case 'component': {
      const component = node as ComponentNode;
      return (
        <>
          <p>{component.description}</p>
          <p>Provides: {component.provides.join(', ') || 'none'}</p>
          <p>Consumes: {component.consumes.join(', ') || 'none'}</p>
        </>
      );
    }
    case 'constraint': {
      const constraint = node as ConstraintNode;
      return (
        <>
          <p>{constraint.description}</p>
          <p>Source: {constraint.source}</p>
        </>
      );
    }
    case 'pattern': {
      const pattern = node as PatternNode;
      return (
        <>
          <p>{pattern.description}</p>
          <p>{pattern.rationale}</p>
        </>
      );
    }
    case 'quality_requirement': {
      const quality = node as QualityRequirementNode;
      return (
        <>
          <p>{quality.scenario}</p>
          <p>{quality.attribute} · {quality.priority}</p>
        </>
      );
    }
  }
}

export default function NodeDetailPanel({ nodeId, edges, api, onClose }: NodeDetailPanelProps) {
  const [node, setNode] = useState<BlueprintNode | null>(null);

  useEffect(() => {
    if (!nodeId) {
      setNode(null);
      return;
    }

    let cancelled = false;
    api.getBlueprintNode(nodeId)
      .then(result => {
        if (!cancelled) {
          setNode(result);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setNode(null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [api, nodeId]);

  const edgeCount = useMemo(() => {
    if (!nodeId) return 0;
    return edges.filter(edge => edge.source === nodeId || edge.target === nodeId).length;
  }, [edges, nodeId]);

  if (!nodeId) {
    return null;
  }

  return (
    <aside className="drawer open" style={{ position: 'relative', inset: 'auto', width: '100%' }}>
      <div className="drawer-header">
        <div>
          <div className="drawer-title">{node ? nodeTitle(node) : 'Loading…'}</div>
          {node && (
            <div className="drawer-badges">
              <span className={`badge badge-${node.node_type}`}>{labelNodeType(node.node_type, 'short')}</span>
              <span className="status-badge">{edgeCount} edges</span>
            </div>
          )}
        </div>
        <button className="drawer-close" onClick={onClose} aria-label="Close node detail panel">×</button>
      </div>
      <div className="drawer-body">
        {!node && <p style={{ color: 'var(--color-text-faint)' }}>Node details unavailable.</p>}
        {node && (
          <>
            {renderNodeBody(node)}
            {node.documentation && (
              <>
                <h4 style={{ marginTop: '1rem' }}>Documentation</h4>
                <pre style={{ whiteSpace: 'pre-wrap', margin: 0 }}>{node.documentation}</pre>
              </>
            )}
          </>
        )}
      </div>
    </aside>
  );
}
