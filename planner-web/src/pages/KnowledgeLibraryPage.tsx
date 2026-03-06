import { useEffect, useState, useCallback, useMemo } from 'react';
import Layout from '../components/Layout.tsx';
import NodeListPanel from '../components/NodeListPanel.tsx';
import DetailDrawer from '../components/DetailDrawer.tsx';
import DeleteNodeDialog from '../components/DeleteNodeDialog.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import type { BlueprintResponse, NodeType } from '../types/blueprint.ts';

// ─── Tab config ─────────────────────────────────────────────────────────────

const TABS: { key: NodeType | 'all'; label: string; icon: string }[] = [
  { key: 'all',                  label: 'All',           icon: '◎' },
  { key: 'decision',            label: 'Decisions',      icon: '◆' },
  { key: 'technology',          label: 'Technologies',   icon: '⬡' },
  { key: 'component',           label: 'Components',     icon: '▪' },
  { key: 'constraint',          label: 'Constraints',    icon: '◇' },
  { key: 'pattern',             label: 'Patterns',       icon: '◉' },
  { key: 'quality_requirement', label: 'Quality',        icon: '⛨' },
];

// ─── Page Component ─────────────────────────────────────────────────────────

export default function KnowledgeLibraryPage() {
  const getToken = useGetAccessToken();
  const api = useMemo(() => createApiClient(getToken), [getToken]);

  const [blueprint, setBlueprint] = useState<BlueprintResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [activeTab, setActiveTab] = useState<NodeType | 'all'>('all');
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  // Delete state
  const [deleteNodeId, setDeleteNodeId] = useState<string | null>(null);
  const [deleteNodeName, setDeleteNodeName] = useState<string | null>(null);

  // ─── Data loading ───────────────────────────────────────────────────────

  const loadBlueprint = useCallback(async () => {
    try {
      const data = await api.getBlueprint();
      setBlueprint(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [api]);

  useEffect(() => {
    void loadBlueprint();
  }, [loadBlueprint]);

  // ─── Handlers ───────────────────────────────────────────────────────────

  const handleSelectNode = useCallback((nodeId: string) => {
    setSelectedNodeId(nodeId);
  }, []);

  const handleNavigateNode = useCallback((nodeId: string) => {
    setSelectedNodeId(nodeId);
  }, []);

  const handleRequestDelete = useCallback((nodeId: string) => {
    const node = blueprint?.nodes.find(n => n.id === nodeId);
    setDeleteNodeId(nodeId);
    setDeleteNodeName(node?.name ?? nodeId);
  }, [blueprint]);

  const handleConfirmDelete = useCallback(async (nodeId: string) => {
    await api.deleteBlueprintNode(nodeId);
    if (selectedNodeId === nodeId) setSelectedNodeId(null);
    await loadBlueprint();
    setDeleteNodeId(null);
    setDeleteNodeName(null);
  }, [api, selectedNodeId, loadBlueprint]);

  const handleDeleteClose = useCallback(() => {
    setDeleteNodeId(null);
    setDeleteNodeName(null);
  }, []);

  // ─── Derived data ─────────────────────────────────────────────────────

  const effectiveType: NodeType | null = activeTab === 'all' ? null : activeTab;
  const nodes = blueprint?.nodes ?? [];
  const edges = blueprint?.edges ?? [];

  // Counts per type for tab badges
  const typeCounts = useMemo(() => {
    const counts: Record<string, number> = { all: nodes.length };
    for (const n of nodes) {
      counts[n.node_type] = (counts[n.node_type] ?? 0) + 1;
    }
    return counts;
  }, [nodes]);

  // ─── Render ───────────────────────────────────────────────────────────

  return (
    <Layout>
      <div className="knowledge-page">
        {/* Header */}
        <div className="knowledge-header">
          <div style={{ flex: 1 }}>
            <h1 style={{ margin: 0, fontSize: 'var(--text-lg)', fontWeight: 600 }}>Knowledge Library</h1>
            <p style={{ margin: '4px 0 0', fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
              Browse, search, and manage all architectural knowledge in one place.
            </p>
          </div>
          {blueprint && (
            <div className="knowledge-summary">
              <div className="knowledge-stat">
                <span className="knowledge-stat-value">{nodes.length}</span>
                <span className="knowledge-stat-label">Nodes</span>
              </div>
              <div className="knowledge-stat">
                <span className="knowledge-stat-value">{edges.length}</span>
                <span className="knowledge-stat-label">Edges</span>
              </div>
              <div className="knowledge-stat">
                <span className="knowledge-stat-value">{Object.keys(blueprint.counts).length}</span>
                <span className="knowledge-stat-label">Types</span>
              </div>
            </div>
          )}
        </div>

        {/* Tabs */}
        <div className="knowledge-tabs">
          {TABS.map(tab => (
            <button
              key={tab.key}
              className={`knowledge-tab${activeTab === tab.key ? ' active' : ''}`}
              onClick={() => setActiveTab(tab.key)}
            >
              <span className="knowledge-tab-icon">{tab.icon}</span>
              {tab.label}
              <span className="knowledge-tab-count">{typeCounts[tab.key] ?? 0}</span>
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="knowledge-content">
          {loading && (
            <div style={{ display: 'flex', justifyContent: 'center', padding: 'var(--space-12)' }}>
              <div className="skeleton-pulse" />
            </div>
          )}

          {error && (
            <div style={{ padding: 'var(--space-8)', textAlign: 'center', color: 'var(--color-error)' }}>
              Failed to load blueprint: {error}
            </div>
          )}

          {!loading && !error && blueprint && (
            <NodeListPanel
              nodes={nodes}
              edges={edges}
              nodeType={effectiveType}
              onSelectNode={handleSelectNode}
            />
          )}
        </div>

        {/* Detail Drawer */}
        <DetailDrawer
          nodeId={selectedNodeId}
          allNodes={nodes}
          edges={edges}
          api={api}
          onClose={() => setSelectedNodeId(null)}
          onNavigateNode={handleNavigateNode}
          onImpactPreview={() => {}} // Impact preview available on Blueprint page
          onRequestDelete={handleRequestDelete}
          onNodeUpdated={loadBlueprint}
        />

        {/* Delete confirmation */}
        <DeleteNodeDialog
          isOpen={deleteNodeId !== null}
          nodeId={deleteNodeId}
          nodeName={deleteNodeName}
          onClose={handleDeleteClose}
          onConfirm={handleConfirmDelete}
        />
      </div>
    </Layout>
  );
}
