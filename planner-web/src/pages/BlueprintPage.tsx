import { useEffect, useState, useMemo, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import BlueprintGraph from '../components/BlueprintGraph.tsx';
import TableView from '../components/TableView.tsx';
import RadarView from '../components/RadarView.tsx';
import DetailDrawer from '../components/DetailDrawer.tsx';
import ImpactPreviewModal from '../components/ImpactPreviewModal.tsx';
import CreateNodeModal from '../components/CreateNodeModal.tsx';
import DeleteNodeDialog from '../components/DeleteNodeDialog.tsx';
import AddEdgeModal from '../components/AddEdgeModal.tsx';
import ReconvergencePanel from '../components/ReconvergencePanel.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import { buildKnowledgeDeepLink } from '../lib/knowledgeDeepLinks.ts';
import type { BlueprintResponse, BlueprintNode, NodeType, NodeSummary, ImpactReport, EdgeType, ReconvergenceResult, ReconvergenceStep, ScopeClass } from '../types/blueprint.ts';

// ─── View types ─────────────────────────────────────────────────────────────

type ViewMode = 'graph' | 'table' | 'radar';

// ─── Node type filter config ────────────────────────────────────────────────

const NODE_TYPES: { value: NodeType | null; label: string; icon: string }[] = [
  { value: null,                  label: 'All Nodes',    icon: '◎' },
  { value: 'decision',           label: 'Decisions',     icon: '◆' },
  { value: 'technology',         label: 'Technologies',  icon: '⬡' },
  { value: 'component',          label: 'Components',    icon: '▪' },
  { value: 'constraint',         label: 'Constraints',   icon: '◇' },
  { value: 'pattern',            label: 'Patterns',      icon: '◉' },
  { value: 'quality_requirement', label: 'Quality',      icon: '⛨' },
];

// ─── Edge type labels ───────────────────────────────────────────────────────

const EDGE_STYLES: { type: string; label: string; dash: string }[] = [
  { type: 'depends_on',  label: 'depends on', dash: '' },
  { type: 'decided_by',  label: 'decided by', dash: '8,4' },
  { type: 'supersedes',  label: 'supersedes', dash: '4,2,1,2' },
  { type: 'constrains',  label: 'constrains', dash: '3,3' },
  { type: 'uses',        label: 'uses',       dash: '' },
  { type: 'implements',  label: 'implements', dash: '2,4' },
  { type: 'satisfies',   label: 'satisfies',  dash: '8,3,2,3' },
  { type: 'affects',     label: 'affects',     dash: '6,4' },
];

// ─── BlueprintPage ──────────────────────────────────────────────────────────

export default function BlueprintPage() {
  const navigate = useNavigate();
  const getToken = useGetAccessToken();
  const api = useMemo(() => createApiClient(getToken), [getToken]);

  // Data state
  const [blueprint, setBlueprint] = useState<BlueprintResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [fetchError, setFetchError] = useState<string | null>(null);

  // UI state
  const [viewMode, setViewMode] = useState<ViewMode>('graph');
  const [layoutMode, setLayoutMode] = useState<'force' | 'hierarchical'>('force');
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [hoveredNodeId, setHoveredNodeId] = useState<string | null>(null);
  const [filterType, setFilterType] = useState<NodeType | null>(null);
  const [globalSearch, setGlobalSearch] = useState('');

  // Impact preview state
  const [impactNodeId, setImpactNodeId] = useState<string | null>(null);
  const [impactReport, setImpactReport] = useState<ImpactReport | null>(null);
  const [impactLoading, setImpactLoading] = useState(false);

  // Create node modal state
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [addEdgeModalOpen, setAddEdgeModalOpen] = useState(false);

  // Reconvergence state
  const [reconResult, setReconResult] = useState<ReconvergenceResult | null>(null);
  const [reconLoading, setReconLoading] = useState(false);
  const [reconVisible, setReconVisible] = useState(false);

  // Delete node dialog state
  const [deleteNodeId, setDeleteNodeId] = useState<string | null>(null);
  const [deleteNodeName, setDeleteNodeName] = useState<string | null>(null);

  // ─── Data fetching ──────────────────────────────────────────────────────

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

  // ─── Impact preview ─────────────────────────────────────────────────────

  const handleImpactPreview = useCallback(async (nodeId: string) => {
    setImpactNodeId(nodeId);
    setImpactLoading(true);
    setImpactReport(null);
    try {
      const report = await api.impactPreview(nodeId, 'Proposed change impact analysis');
      setImpactReport(report);
    } catch {
      // Still show modal, just without data
      setImpactReport(null);
    } finally {
      setImpactLoading(false);
    }
  }, [api]);

  const handleImpactClose = useCallback(() => {
    setImpactNodeId(null);
    setImpactReport(null);
    setImpactLoading(false);
  }, []);

  const handleImpactApply = useCallback(async () => {
    if (!impactNodeId || !impactReport) {
      handleImpactClose();
      return;
    }

    const request = {
      source_node_id: impactNodeId,
      impact_report: impactReport,
      auto_apply: true,
    };

    const seedResult = (): ReconvergenceResult => ({
      steps: [],
      summary: {
        total: 0,
        applied: 0,
        skipped: 0,
        errors: 0,
        needs_review: 0,
      },
      timestamp: new Date().toISOString(),
    });

    const summarize = (steps: ReconvergenceStep[]) => ({
      total: steps.length,
      applied: steps.filter(step => step.status === 'done').length,
      skipped: steps.filter(step => step.status === 'skipped').length,
      errors: steps.filter(step => step.status === 'error').length,
      needs_review: steps.filter(step => step.status === 'pending').length,
    });

    const runRestFallback = async () => {
      try {
        const result = await api.reconvergeBlueprint(request);
        setReconResult(result);
      } catch {
        setReconResult(null);
      } finally {
        setReconLoading(false);
      }
    };

    // Close the impact modal and open the reconvergence panel
    handleImpactClose();
    setReconVisible(true);
    setReconLoading(true);
    setReconResult(seedResult());

    try {
      let resolved = false;
      let fallbackStarted = false;
      const steps: ReconvergenceStep[] = [];

      const startFallback = () => {
        if (resolved || fallbackStarted) return;
        fallbackStarted = true;
        void runRestFallback();
      };

      const ws = await api.reconvergeBlueprintWs(request, {
        onStep: step => {
          steps.push(step);
          setReconResult(prev => ({
            ...(prev ?? seedResult()),
            steps: [...steps],
            summary: summarize(steps),
          }));
        },
        onComplete: summary => {
          resolved = true;
          setReconResult(prev => ({
            ...(prev ?? seedResult()),
            steps: prev?.steps ?? [...steps],
            summary,
          }));
          setReconLoading(false);
        },
        onError: () => {
          startFallback();
        },
      });

      ws.addEventListener('close', () => {
        startFallback();
      });
    } catch {
      await runRestFallback();
    }
  }, [impactNodeId, impactReport, api, handleImpactClose]);

  const handleReconClose = useCallback(() => {
    setReconVisible(false);
    setReconResult(null);
    setReconLoading(false);
    // Refresh blueprint data after reconvergence
    void loadBlueprint();
  }, [loadBlueprint]);

  // ─── Create node ────────────────────────────────────────────────────────

  const handleCreateNode = useCallback(async (node: BlueprintNode) => {
    await api.createBlueprintNode(node);
    await loadBlueprint();
  }, [api, loadBlueprint]);

  // ─── Create edge ────────────────────────────────────────────────────────

  const handleCreateEdge = useCallback(async (edge: { source: string; target: string; edge_type: EdgeType; metadata?: string }) => {
    await api.createBlueprintEdge(edge);
    await loadBlueprint();
  }, [api, loadBlueprint]);

  // ─── Delete node ────────────────────────────────────────────────────────

  const handleRequestDelete = useCallback((nodeId: string) => {
    const node = blueprint?.nodes.find(n => n.id === nodeId);
    setDeleteNodeId(nodeId);
    setDeleteNodeName(node?.name ?? nodeId);
  }, [blueprint]);

  const handleConfirmDelete = useCallback(async (nodeId: string) => {
    await api.deleteBlueprintNode(nodeId);
    if (selectedNodeId === nodeId) setSelectedNodeId(null);
    await loadBlueprint();
  }, [api, selectedNodeId, loadBlueprint]);

  const handleDeleteClose = useCallback(() => {
    setDeleteNodeId(null);
    setDeleteNodeName(null);
  }, []);

  // ─── Node selection / navigation ────────────────────────────────────────

  const handleSelectNode = useCallback((nodeId: string | null) => {
    setSelectedNodeId(nodeId);
  }, []);

  const handleNavigateNode = useCallback((nodeId: string) => {
    setSelectedNodeId(nodeId);
  }, []);

  // ─── Hover info ─────────────────────────────────────────────────────────

  const hoveredNode: NodeSummary | null = useMemo(() => {
    if (!hoveredNodeId || !blueprint) return null;
    return blueprint.nodes.find(n => n.id === hoveredNodeId) ?? null;
  }, [hoveredNodeId, blueprint]);

  const selectedNode: NodeSummary | null = useMemo(() => {
    if (!selectedNodeId || !blueprint) return null;
    return blueprint.nodes.find(n => n.id === selectedNodeId) ?? null;
  }, [blueprint, selectedNodeId]);

  const relatedKnowledgeLink = useMemo(() => {
    if (!selectedNode) return null;
    const projectId = selectedNode.project_id?.trim();
    if (!projectId) return null;

    const secondary = selectedNode.secondary_scope ?? {};
    const component = secondary.component?.trim()
      || (selectedNode.node_type === 'component' ? selectedNode.name.trim() : undefined);

    return buildKnowledgeDeepLink({
      projectId,
      feature: secondary.feature,
      widget: secondary.widget,
      artifact: secondary.artifact,
      component,
      originPath: '/blueprint',
      originLabel: 'Blueprint',
    });
  }, [selectedNode]);

  const createInitialScope = useMemo(() => {
    if (!selectedNode?.project_id) return undefined;
    const secondary = selectedNode.secondary_scope ?? {};
    const hasSecondary = Boolean(
      secondary.feature || secondary.widget || secondary.artifact || secondary.component,
    );
    const scopeClass: ScopeClass = hasSecondary ? 'project_contextual' : 'project';
    return {
      scopeClass,
      projectId: selectedNode.project_id,
      projectName: selectedNode.project_name ?? selectedNode.project_id,
      feature: secondary.feature,
      widget: secondary.widget,
      artifact: secondary.artifact,
      component: secondary.component,
    };
  }, [selectedNode]);

  // ─── Node counts for sidebar ────────────────────────────────────────────

  const nodeCounts = useMemo(() => {
    if (!blueprint) return {} as Record<string, number>;
    return blueprint.counts;
  }, [blueprint]);

  // ─── Global search filtering ───────────────────────────────────────────

  const filteredBlueprint = useMemo(() => {
    if (!blueprint) return null;
    const q = globalSearch.trim().toLowerCase();
    if (!q && !filterType) return blueprint;
    let nodes = blueprint.nodes;
    if (filterType) nodes = nodes.filter(n => n.node_type === filterType);
    if (q) {
      nodes = nodes.filter(n =>
        n.name.toLowerCase().includes(q) ||
        n.id.toLowerCase().includes(q) ||
        n.tags.some(t => t.toLowerCase().includes(q)) ||
        n.status.toLowerCase().includes(q) ||
        n.node_type.toLowerCase().includes(q)
      );
    }
    const nodeIds = new Set(nodes.map(n => n.id));
    const edges = blueprint.edges.filter(e => nodeIds.has(e.source) && nodeIds.has(e.target));
    return { ...blueprint, nodes, edges };
  }, [blueprint, filterType, globalSearch]);

  // ─── Render ─────────────────────────────────────────────────────────────

  return (
    <Layout>
      <div className="main">
        {/* Topbar */}
        <div className="topbar">
          <div className="topbar-left">
            <div className="topbar-title">Blueprint</div>
            {blueprint && (
              <div className="topbar-subtitle">
                {blueprint.total_nodes} nodes · {blueprint.total_edges} edges
              </div>
            )}
          </div>

          <div className="topbar-right">
            {/* Global search */}
            <div className="global-search">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--color-text-faint)" strokeWidth="2" strokeLinecap="round">
                <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
              </svg>
              <input
                type="text"
                placeholder="Search nodes…"
                value={globalSearch}
                onChange={e => setGlobalSearch(e.target.value)}
                className="global-search-input"
              />
              {globalSearch && (
                <button
                  className="global-search-clear"
                  onClick={() => setGlobalSearch('')}
                  aria-label="Clear search"
                >
                  ×
                </button>
              )}
            </div>

            {/* View tabs */}
            <div className="view-tabs">
              {(['graph', 'table', 'radar'] as ViewMode[]).map(v => (
                <button
                  key={v}
                  className={`view-tab${viewMode === v ? ' active' : ''}`}
                  onClick={() => setViewMode(v)}
                >
                  {v === 'graph' && (
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                      <circle cx="6" cy="6" r="3"/><circle cx="18" cy="18" r="3"/><circle cx="18" cy="6" r="3"/>
                      <path d="M8.5 8.5l7 7M8.5 6h7"/>
                    </svg>
                  )}
                  {v === 'table' && (
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                      <rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18M3 15h18M9 3v18"/>
                    </svg>
                  )}
                  {v === 'radar' && (
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                      <circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="6"/><circle cx="12" cy="12" r="2"/>
                      <path d="M12 2v4M12 18v4"/>
                    </svg>
                  )}
                  {v.charAt(0).toUpperCase() + v.slice(1)}
                </button>
              ))}
            </div>

            {/* Refresh */}
            <button
              className="btn btn-ghost"
              onClick={() => void loadBlueprint()}
              title="Refresh blueprint data"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                <path d="M21 2v6h-6M3 12a9 9 0 0 1 15-6.7L21 8M3 22v-6h6M21 12a9 9 0 0 1-15 6.7L3 16"/>
              </svg>
            </button>

            {/* Layout toggle (graph view only) */}
            {viewMode === 'graph' && (
              <button
                className={`btn btn-ghost${layoutMode === 'hierarchical' ? ' active' : ''}`}
                onClick={() => setLayoutMode(m => m === 'force' ? 'hierarchical' : 'force')}
                title={layoutMode === 'force' ? 'Switch to hierarchical layout' : 'Switch to force-directed layout'}
                style={{ fontSize: 'var(--text-xs)', gap: '4px' }}
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                  <rect x="8" y="2" width="8" height="4" rx="1"/>
                  <rect x="2" y="18" width="8" height="4" rx="1"/>
                  <rect x="14" y="18" width="8" height="4" rx="1"/>
                  <path d="M12 6v6M6 18v-6h12v6"/>
                </svg>
                {layoutMode === 'force' ? 'Tree' : 'Force'}
              </button>
            )}

            {/* Create node button */}
            <button
              className="btn btn-primary"
              onClick={() => setCreateModalOpen(true)}
              title="Create a new blueprint node"
              style={{ fontSize: 'var(--text-xs)', gap: '4px' }}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
              </svg>
              New Node
            </button>

            {/* Add edge button */}
            <button
              className="btn btn-outline"
              onClick={() => setAddEdgeModalOpen(true)}
              title="Add an edge between two nodes"
              style={{ fontSize: 'var(--text-xs)', gap: '4px' }}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                <path d="M5 12h14M12 5l7 7-7 7"/>
              </svg>
              Add Edge
            </button>

            {/* Delete node button */}
            <button
              className="btn btn-ghost"
              onClick={() => { if (selectedNodeId) handleRequestDelete(selectedNodeId); }}
              title="Delete selected node"
              disabled={!selectedNodeId}
              style={{ opacity: selectedNodeId ? 1 : 0.4, color: selectedNodeId ? 'var(--color-error)' : undefined }}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                <polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
              </svg>
            </button>

            {/* Impact button */}
            <button
              className="btn btn-ghost"
              onClick={() => {
                if (selectedNodeId) handleImpactPreview(selectedNodeId);
              }}
              title="Impact preview"
              disabled={!selectedNodeId}
              style={{ opacity: selectedNodeId ? 1 : 0.4 }}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>
              </svg>
              Impact
            </button>

            <button
              className="btn btn-outline"
              onClick={() => {
                if (relatedKnowledgeLink) {
                  void navigate(relatedKnowledgeLink);
                }
              }}
              title={relatedKnowledgeLink
                ? 'View related knowledge in project scope'
                : 'Select a project-scoped node to view related knowledge'}
              disabled={!relatedKnowledgeLink}
              style={{ opacity: relatedKnowledgeLink ? 1 : 0.4 }}
            >
              View related knowledge
            </button>
          </div>
        </div>

        {/* Content area */}
        <div className="content" style={{ display: 'flex' }}>
          {/* Sidebar filter (inside content, not app-level sidebar) */}
          <div style={{
            width: '200px', flexShrink: 0,
            borderRight: '1px solid var(--color-divider)',
            background: 'var(--color-surface)',
            display: 'flex', flexDirection: 'column',
            padding: 'var(--space-3) 0',
            overflowY: 'auto', overscrollBehavior: 'contain',
          }}>
            <div className="sidebar-label" style={{ padding: 'var(--space-1) var(--space-4)' }}>
              Node Types
            </div>
            {NODE_TYPES.map(t => {
              const isActive = filterType === t.value;
              const count = t.value === null
                ? (blueprint?.total_nodes ?? 0)
                : (nodeCounts[t.value] ?? 0);
              return (
                <button
                  key={t.label}
                  className={`sidebar-item${isActive ? ' active' : ''}`}
                  onClick={() => setFilterType(isActive && t.value !== null ? null : t.value)}
                  style={{ padding: 'var(--space-1) var(--space-4)' }}
                >
                  <span className="icon">{t.icon}</span>
                  {t.label}
                  <span className="count">{count}</span>
                </button>
              );
            })}

            {/* Edge legend (only when graph is active) */}
            {viewMode === 'graph' && (
              <>
                <div style={{ margin: 'var(--space-4) 0 var(--space-1)', borderTop: '1px solid var(--color-divider)' }} />
                <div className="sidebar-label" style={{ padding: 'var(--space-1) var(--space-4)' }}>
                  Edge Types
                </div>
                {EDGE_STYLES.map(e => (
                  <div
                    key={e.type}
                    style={{
                      display: 'flex', alignItems: 'center', gap: 'var(--space-2)',
                      padding: '2px var(--space-4)', fontSize: 'var(--text-xs)',
                      color: 'var(--color-text-faint)',
                    }}
                  >
                    <svg width="24" height="8" style={{ flexShrink: 0 }}>
                      <line
                        x1="0" y1="4" x2="24" y2="4"
                        stroke="var(--color-text-faint)" strokeWidth="1.2"
                        strokeDasharray={e.dash || 'none'}
                      />
                    </svg>
                    <span>{e.label}</span>
                  </div>
                ))}
              </>
            )}
          </div>

          {/* Main view area */}
          <div style={{ flex: 1, overflow: 'hidden', position: 'relative' }}>
            {/* Loading state */}
            {loading && (
              <div style={{
                position: 'absolute', inset: 0, display: 'flex',
                alignItems: 'center', justifyContent: 'center',
                background: 'var(--color-bg)', zIndex: 3,
              }}>
                <div className="skeleton-pulse" />
              </div>
            )}

            {/* Error state */}
            {fetchError && (
              <div style={{
                position: 'absolute', inset: 0, display: 'flex',
                flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
                gap: '12px', zIndex: 2,
              }}>
                <div style={{ color: 'var(--color-error)', fontSize: 'var(--text-sm)' }}>
                  failed to load blueprint
                </div>
                <div style={{
                  color: 'var(--color-text-faint)', fontSize: 'var(--text-xs)',
                  maxWidth: '400px', textAlign: 'center',
                }}>
                  {fetchError}
                </div>
                <button
                  className="btn btn-outline"
                  onClick={() => void loadBlueprint()}
                >
                  retry
                </button>
              </div>
            )}

            {/* Graph view */}
            {!loading && !fetchError && filteredBlueprint && (
              <div style={{
                width: '100%', height: '100%',
                display: viewMode === 'graph' ? 'block' : 'none',
                position: 'absolute', inset: 0,
              }}>
                <BlueprintGraph
                  nodes={filteredBlueprint.nodes}
                  edges={filteredBlueprint.edges}
                  selectedNodeId={selectedNodeId}
                  onSelectNode={handleSelectNode}
                  onHoverNode={setHoveredNodeId}
                  filterType={filterType}
                  layoutMode={layoutMode}
                />

                {/* Hover tooltip */}
                {hoveredNode && !selectedNodeId && (
                  <div style={{
                    position: 'absolute', bottom: '16px', left: '16px',
                    background: 'var(--color-surface)', border: '1px solid var(--color-border)',
                    borderRadius: 'var(--radius-md)', padding: 'var(--space-2) var(--space-3)',
                    fontSize: 'var(--text-xs)', color: 'var(--color-text)',
                    pointerEvents: 'none', zIndex: 10, maxWidth: '280px',
                    boxShadow: 'var(--shadow-md)',
                  }}>
                    <div style={{ fontWeight: 600, marginBottom: '2px' }}>{hoveredNode.name}</div>
                    <div style={{ color: 'var(--color-text-faint)', fontSize: '0.625rem' }}>
                      {hoveredNode.node_type}
                      {hoveredNode.tags.length > 0 && ` · ${hoveredNode.tags.join(', ')}`}
                    </div>
                  </div>
                )}
              </div>
            )}

            {/* Table view */}
            {!loading && !fetchError && filteredBlueprint && viewMode === 'table' && (
              <TableView
                nodes={filteredBlueprint.nodes}
                edges={filteredBlueprint.edges}
                filterType={filterType}
                onSelectNode={(id) => handleSelectNode(id)}
              />
            )}

            {/* Radar view */}
            {!loading && !fetchError && filteredBlueprint && viewMode === 'radar' && (
              <div style={{ width: '100%', height: '100%' }}>
                <RadarView
                  nodes={filteredBlueprint.nodes}
                  onSelectNode={(id) => handleSelectNode(id)}
                />
              </div>
            )}
          </div>
        </div>

        {/* Detail drawer */}
        <DetailDrawer
          nodeId={selectedNodeId}
          allNodes={blueprint?.nodes ?? []}
          edges={blueprint?.edges ?? []}
          api={api}
          onClose={() => handleSelectNode(null)}
          onNavigateNode={handleNavigateNode}
          onImpactPreview={handleImpactPreview}
          onRequestDelete={handleRequestDelete}
          onNodeUpdated={loadBlueprint}
        />

        {/* Impact preview modal */}
        <ImpactPreviewModal
          isOpen={impactNodeId !== null}
          report={impactReport}
          loading={impactLoading}
          onClose={handleImpactClose}
          onApply={handleImpactApply}
        />

        {/* Create node modal */}
        <CreateNodeModal
          isOpen={createModalOpen}
          onClose={() => setCreateModalOpen(false)}
          onCreate={handleCreateNode}
          initialScope={createInitialScope}
          requireExplicitScopeSelection
        />

        {/* Delete node confirmation */}
        <DeleteNodeDialog
          isOpen={deleteNodeId !== null}
          nodeId={deleteNodeId}
          nodeName={deleteNodeName}
          onClose={handleDeleteClose}
          onConfirm={handleConfirmDelete}
        />

        {/* Add edge modal */}
        <AddEdgeModal
          isOpen={addEdgeModalOpen}
          nodes={blueprint?.nodes ?? []}
          defaultSourceId={selectedNodeId}
          onClose={() => setAddEdgeModalOpen(false)}
          onCreate={handleCreateEdge}
        />

        {/* Reconvergence panel */}
        {reconVisible && (
          <ReconvergencePanel
            result={reconResult}
            loading={reconLoading}
            onClose={handleReconClose}
          />
        )}
      </div>
    </Layout>
  );
}
