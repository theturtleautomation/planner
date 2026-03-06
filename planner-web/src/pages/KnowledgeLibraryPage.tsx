import { useEffect, useState, useCallback, useMemo } from 'react';
import { Link, useLocation, useParams } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import NodeListPanel from '../components/NodeListPanel.tsx';
import DetailDrawer from '../components/DetailDrawer.tsx';
import DeleteNodeDialog from '../components/DeleteNodeDialog.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import type { BlueprintResponse, NodeSummary, NodeType, ScopeClass } from '../types/blueprint.ts';

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

const SCOPE_TABS: { key: ScopeClass | 'all'; label: string }[] = [
  { key: 'all', label: 'All Scope' },
  { key: 'project', label: 'Project' },
  { key: 'project_contextual', label: 'Project Contextual' },
  { key: 'global', label: 'Global' },
  { key: 'unscoped', label: 'Unscoped' },
];

const MAJOR_TYPES: NodeType[] = [
  'decision',
  'technology',
  'component',
  'constraint',
  'pattern',
  'quality_requirement',
];

const NODE_TYPE_LABELS: Record<NodeType, string> = {
  decision: 'Decisions',
  technology: 'Technologies',
  component: 'Components',
  constraint: 'Constraints',
  pattern: 'Patterns',
  quality_requirement: 'Quality',
};

const FAVORITES_STORAGE_KEY = 'knowledge-project-favorites';
const STALE_THRESHOLD_DAYS = 30;

type ProjectSortKey =
  | 'health_desc'
  | 'activity_desc'
  | 'knowledge_desc'
  | 'stale_desc'
  | 'name_asc';

interface ProjectSummary {
  id: string;
  name: string;
  description: string;
  totalKnowledge: number;
  localKnowledge: number;
  sharedKnowledge: number;
  typeCounts: Record<NodeType, number>;
  staleCount: number;
  lastActivityIso: string | null;
  lastActivityMs: number;
  healthScore: number;
  healthLabel: string;
  searchableText: string;
}

interface ProjectAccumulator {
  nodeIds: Set<string>;
  typeCounts: Record<NodeType, number>;
  staleCount: number;
  docsCount: number;
  newestActivityMs: number;
  tagCounts: Map<string, { label: string; count: number }>;
  localKnowledge: number;
  sharedKnowledge: number;
  projectNameCounts: Map<string, number>;
}

function isMajorType(value: string): value is NodeType {
  return MAJOR_TYPES.includes(value as NodeType);
}

function upsertCount(
  map: Map<string, { label: string; count: number }>,
  rawValue: string,
) {
  const label = rawValue.trim();
  if (!label) return;
  const key = label.toLowerCase();
  const existing = map.get(key);
  if (existing) {
    existing.count += 1;
  } else {
    map.set(key, { label, count: 1 });
  }
}

function pickTopTags(
  map: Map<string, { label: string; count: number }>,
  limit: number,
): string[] {
  return Array.from(map.values())
    .sort((a, b) => b.count - a.count || a.label.localeCompare(b.label))
    .slice(0, limit)
    .map(entry => entry.label);
}

function parseIsoTimeMs(iso: string): number {
  const timestamp = new Date(iso).getTime();
  return Number.isFinite(timestamp) ? timestamp : 0;
}

function healthLabel(score: number): string {
  if (score >= 80) return 'Healthy';
  if (score >= 55) return 'Needs Attention';
  return 'At Risk';
}

function formatLastActivity(iso: string | null): string {
  if (!iso) return 'No activity yet';
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return 'No activity yet';
  return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
}

function formatRelativeActivity(timestampMs: number): string {
  if (!timestampMs || Number.isNaN(timestampMs)) return 'No recent updates';
  const diffMs = Date.now() - timestampMs;
  if (diffMs < 0) return 'Updated recently';
  const dayMs = 24 * 60 * 60 * 1000;
  const hourMs = 60 * 60 * 1000;
  if (diffMs < hourMs) return 'Updated less than 1 hour ago';
  if (diffMs < dayMs) {
    const hours = Math.floor(diffMs / hourMs);
    return `Updated ${hours}h ago`;
  }
  const days = Math.floor(diffMs / dayMs);
  return days === 1 ? 'Updated 1 day ago' : `Updated ${days} days ago`;
}

function buildProjectDescription(args: {
  typeCounts: Record<NodeType, number>;
  topTags: string[];
  healthScore: number;
  staleCount: number;
  totalKnowledge: number;
  docsCount: number;
}): string {
  const dominantTypes = MAJOR_TYPES
    .filter(type => args.typeCounts[type] > 0)
    .sort((a, b) => args.typeCounts[b] - args.typeCounts[a] || a.localeCompare(b))
    .slice(0, 2)
    .map(type => NODE_TYPE_LABELS[type].toLowerCase());

  const focusPhrase = args.topTags.length > 0
    ? `Focuses on ${args.topTags.slice(0, 2).join(' and ')}`
    : dominantTypes.length > 0
      ? `Leans toward ${dominantTypes.join(' and ')}`
      : 'Has mixed knowledge coverage';

  const freshnessRatio = (args.totalKnowledge - args.staleCount) / Math.max(args.totalKnowledge, 1);
  const freshnessPhrase = freshnessRatio >= 0.75
    ? 'with mostly current documentation'
    : args.staleCount > 0
      ? `with ${args.staleCount} stale item${args.staleCount === 1 ? '' : 's'} to refresh`
      : 'with active updates';

  const docsRatio = args.docsCount / Math.max(args.totalKnowledge, 1);
  const docsPhrase = docsRatio >= 0.6
    ? 'and strong docs coverage.'
    : 'and room to improve docs coverage.';

  const healthTone = args.healthScore >= 80
    ? 'Health is strong'
    : args.healthScore >= 55
      ? 'Health is moderate'
      : 'Health is weak';

  return `${focusPhrase}; ${healthTone} ${freshnessPhrase} ${docsPhrase}`;
}

function readFavoriteProjectIds(): string[] {
  if (typeof window === 'undefined') return [];
  try {
    const raw = window.localStorage.getItem(FAVORITES_STORAGE_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter((value): value is string => typeof value === 'string')
      .map(value => value.trim())
      .filter(Boolean);
  } catch {
    return [];
  }
}

// ─── Page Component ─────────────────────────────────────────────────────────

export default function KnowledgeLibraryPage() {
  const location = useLocation();
  const { projectId: routeProjectId } = useParams<{ projectId: string }>();
  const projectId = routeProjectId?.trim() || undefined;
  const isProjectScoped = Boolean(projectId);
  const normalizedPath = location.pathname.replace(/\/+$/, '') || '/';
  const isGlobalView = !isProjectScoped && normalizedPath === '/knowledge/all';
  const isProjectLanding = !isProjectScoped && !isGlobalView;

  const getToken = useGetAccessToken();
  const api = useMemo(() => createApiClient(getToken), [getToken]);

  const [blueprint, setBlueprint] = useState<BlueprintResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [activeTab, setActiveTab] = useState<NodeType | 'all'>('all');
  const [activeScope, setActiveScope] = useState<ScopeClass | 'all'>('all');
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [projectSearch, setProjectSearch] = useState('');
  const [projectSort, setProjectSort] = useState<ProjectSortKey>('health_desc');
  const [favoritesOnly, setFavoritesOnly] = useState(false);
  const [favoriteProjectIds, setFavoriteProjectIds] = useState<string[]>(() => readFavoriteProjectIds());

  // Delete state
  const [deleteNodeId, setDeleteNodeId] = useState<string | null>(null);
  const [deleteNodeName, setDeleteNodeName] = useState<string | null>(null);

  // ─── Data loading ───────────────────────────────────────────────────────

  const loadBlueprint = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.getBlueprint(
        isProjectScoped
          ? {
              projectId,
              includeShared: true,
              includeGlobal: false,
            }
          : undefined
      );
      setBlueprint(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [api, isProjectScoped, projectId]);

  useEffect(() => {
    void loadBlueprint();
  }, [loadBlueprint]);

  useEffect(() => {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(FAVORITES_STORAGE_KEY, JSON.stringify(favoriteProjectIds));
  }, [favoriteProjectIds]);

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

  const toggleFavoriteProject = useCallback((projectIdToToggle: string) => {
    setFavoriteProjectIds(prev => (
      prev.includes(projectIdToToggle)
        ? prev.filter(id => id !== projectIdToToggle)
        : [...prev, projectIdToToggle]
    ));
  }, []);

  // ─── Derived data ─────────────────────────────────────────────────────

  const effectiveType: NodeType | null = activeTab === 'all' ? null : activeTab;
  const nodes = blueprint?.nodes ?? [];
  const edges = blueprint?.edges ?? [];
  const scopedNodes = useMemo(() => {
    if (activeScope === 'all') return nodes;
    return nodes.filter(n => (n.scope_class ?? 'unscoped') === activeScope);
  }, [nodes, activeScope]);

  // Counts per type for tab badges
  const typeCounts = useMemo(() => {
    const counts: Record<string, number> = { all: scopedNodes.length };
    for (const n of scopedNodes) {
      counts[n.node_type] = (counts[n.node_type] ?? 0) + 1;
    }
    return counts;
  }, [scopedNodes]);

  const scopeCounts = useMemo(() => {
    const counts: Record<string, number> = { all: nodes.length };
    for (const n of nodes) {
      const key = n.scope_class ?? 'unscoped';
      counts[key] = (counts[key] ?? 0) + 1;
    }
    return counts;
  }, [nodes]);

  const sharedCount = useMemo(() => nodes.filter(n => n.scope_visibility === 'shared' || n.is_shared).length, [nodes]);
  const unscopedCount = scopeCounts.unscoped ?? 0;
  const staleCount = useMemo(() => {
    const staleCutoff = Date.now() - STALE_THRESHOLD_DAYS * 24 * 60 * 60 * 1000;
    return nodes.filter(node => parseIsoTimeMs(node.updated_at) <= staleCutoff).length;
  }, [nodes]);

  const favoriteProjectSet = useMemo(() => new Set(favoriteProjectIds), [favoriteProjectIds]);

  const projectSummaries = useMemo<ProjectSummary[]>(() => {
    if (nodes.length === 0) return [];

    const staleCutoff = Date.now() - STALE_THRESHOLD_DAYS * 24 * 60 * 60 * 1000;
    const projects = new Map<string, ProjectAccumulator>();

    const ensureProject = (id: string): ProjectAccumulator => {
      const existing = projects.get(id);
      if (existing) return existing;
      const created: ProjectAccumulator = {
        nodeIds: new Set<string>(),
        typeCounts: {
          decision: 0,
          technology: 0,
          component: 0,
          constraint: 0,
          pattern: 0,
          quality_requirement: 0,
        },
        staleCount: 0,
        docsCount: 0,
        newestActivityMs: 0,
        tagCounts: new Map(),
        localKnowledge: 0,
        sharedKnowledge: 0,
        projectNameCounts: new Map(),
      };
      projects.set(id, created);
      return created;
    };

    const contributesToProjects = (node: NodeSummary): string[] => {
      const projectIds = new Set<string>();
      if (node.project_id?.trim()) {
        projectIds.add(node.project_id.trim());
      }
      const isSharedNode = node.scope_visibility === 'shared' || node.is_shared;
      if (isSharedNode) {
        for (const linkedProjectId of node.linked_project_ids) {
          const normalized = linkedProjectId.trim();
          if (normalized) projectIds.add(normalized);
        }
      }
      return Array.from(projectIds);
    };

    for (const node of nodes) {
      const projectIds = contributesToProjects(node);
      if (projectIds.length === 0) continue;

      for (const pid of projectIds) {
        const bucket = ensureProject(pid);
        if (bucket.nodeIds.has(node.id)) continue;
        bucket.nodeIds.add(node.id);

        if (isMajorType(node.node_type)) {
          bucket.typeCounts[node.node_type] += 1;
        }

        const updatedMs = parseIsoTimeMs(node.updated_at);
        if (updatedMs > 0) {
          bucket.newestActivityMs = Math.max(bucket.newestActivityMs, updatedMs);
          if (updatedMs <= staleCutoff) {
            bucket.staleCount += 1;
          }
        } else {
          bucket.staleCount += 1;
        }

        if (node.has_documentation) {
          bucket.docsCount += 1;
        }

        const isLocal = node.project_id?.trim() === pid;
        const isSharedForProject =
          (node.scope_visibility === 'shared' || node.is_shared)
          && node.linked_project_ids.some(linkedProjectId => linkedProjectId.trim() === pid);
        if (isLocal) bucket.localKnowledge += 1;
        if (!isLocal && isSharedForProject) bucket.sharedKnowledge += 1;

        if (isLocal && node.project_name?.trim()) {
          const projectName = node.project_name.trim();
          bucket.projectNameCounts.set(projectName, (bucket.projectNameCounts.get(projectName) ?? 0) + 1);
        }

        for (const tag of node.tags) {
          const normalizedTag = tag.trim();
          if (!normalizedTag) continue;
          upsertCount(bucket.tagCounts, normalizedTag);
        }
      }
    }

    const summaries: ProjectSummary[] = [];
    for (const [id, bucket] of projects.entries()) {
      const totalKnowledge = bucket.nodeIds.size;
      if (totalKnowledge === 0) continue;

      const categoriesPresent = MAJOR_TYPES.filter(type => bucket.typeCounts[type] > 0).length;
      const freshnessRatio = (totalKnowledge - bucket.staleCount) / totalKnowledge;
      const docsRatio = bucket.docsCount / totalKnowledge;
      const coverageRatio = categoriesPresent / MAJOR_TYPES.length;
      const healthScore = Math.round((0.4 * freshnessRatio + 0.3 * docsRatio + 0.3 * coverageRatio) * 100);

      const topTags = pickTopTags(bucket.tagCounts, 3);
      const description = buildProjectDescription({
        typeCounts: bucket.typeCounts,
        topTags,
        healthScore,
        staleCount: bucket.staleCount,
        totalKnowledge,
        docsCount: bucket.docsCount,
      });

      const resolvedName = (() => {
        if (bucket.projectNameCounts.size === 0) return id;
        const winner = Array.from(bucket.projectNameCounts.entries())
          .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))[0];
        return winner?.[0] ?? id;
      })();

      const searchableText = [
        id,
        resolvedName,
        description,
        ...topTags,
      ]
        .join(' ')
        .toLowerCase();

      summaries.push({
        id,
        name: resolvedName,
        description,
        totalKnowledge,
        localKnowledge: bucket.localKnowledge,
        sharedKnowledge: bucket.sharedKnowledge,
        typeCounts: bucket.typeCounts,
        staleCount: bucket.staleCount,
        lastActivityIso: bucket.newestActivityMs > 0 ? new Date(bucket.newestActivityMs).toISOString() : null,
        lastActivityMs: bucket.newestActivityMs,
        healthScore,
        healthLabel: healthLabel(healthScore),
        searchableText,
      });
    }

    return summaries;
  }, [nodes]);

  const visibleProjects = useMemo(() => {
    const query = projectSearch.trim().toLowerCase();
    let filtered = projectSummaries.filter(project => (
      (!favoritesOnly || favoriteProjectSet.has(project.id))
      && (!query || project.searchableText.includes(query))
    ));

    filtered = filtered.sort((a, b) => {
      const aFavorite = favoriteProjectSet.has(a.id);
      const bFavorite = favoriteProjectSet.has(b.id);
      if (aFavorite !== bFavorite) return aFavorite ? -1 : 1;

      switch (projectSort) {
        case 'activity_desc':
          return b.lastActivityMs - a.lastActivityMs || b.totalKnowledge - a.totalKnowledge;
        case 'knowledge_desc':
          return b.totalKnowledge - a.totalKnowledge || a.name.localeCompare(b.name);
        case 'stale_desc':
          return b.staleCount - a.staleCount || a.name.localeCompare(b.name);
        case 'name_asc':
          return a.name.localeCompare(b.name);
        case 'health_desc':
        default:
          return b.healthScore - a.healthScore || b.totalKnowledge - a.totalKnowledge;
      }
    });
    return filtered;
  }, [favoriteProjectSet, favoritesOnly, projectSearch, projectSort, projectSummaries]);

  const scopedProjectName = useMemo(() => {
    if (!projectId) return null;
    const name = nodes.find(node => node.project_id === projectId && node.project_name?.trim())?.project_name?.trim();
    return name ?? projectId;
  }, [nodes, projectId]);

  // ─── Render ───────────────────────────────────────────────────────────

  return (
    <Layout>
      <div className="knowledge-page">
        <div className="knowledge-header">
          <div style={{ flex: 1 }}>
            <h1 style={{ margin: 0, fontSize: 'var(--text-lg)', fontWeight: 600 }}>Knowledge Library</h1>
            <p style={{ margin: '4px 0 0', fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
              {isProjectLanding
                ? 'Choose a software project first, then drill into scoped knowledge.'
                : isProjectScoped
                  ? `Project-scoped view for ${scopedProjectName}. Shared knowledge linked to this project is included.`
                  : 'Global cross-project view for intentional broad exploration.'}
            </p>
            {isProjectLanding ? (
              <div style={{ marginTop: 'var(--space-2)' }}>
                <Link to="/knowledge/all" style={{ fontSize: 'var(--text-xs)' }}>
                  Open All Knowledge
                </Link>
              </div>
            ) : (
              <div style={{ marginTop: 'var(--space-2)', display: 'flex', gap: 'var(--space-3)', flexWrap: 'wrap' }}>
                <Link to="/knowledge" style={{ fontSize: 'var(--text-xs)' }}>
                  Back to project chooser
                </Link>
                {!isGlobalView && (
                  <Link to="/knowledge/all" style={{ fontSize: 'var(--text-xs)' }}>
                    Open global knowledge view
                  </Link>
                )}
              </div>
            )}
          </div>
          {blueprint && (
            <div className="knowledge-summary">
              <div className="knowledge-stat">
                <span className="knowledge-stat-value">{isProjectLanding ? projectSummaries.length : nodes.length}</span>
                <span className="knowledge-stat-label">{isProjectLanding ? 'Projects' : 'Nodes'}</span>
              </div>
              <div className="knowledge-stat">
                <span className="knowledge-stat-value">{isProjectLanding ? nodes.length : edges.length}</span>
                <span className="knowledge-stat-label">{isProjectLanding ? 'Knowledge' : 'Edges'}</span>
              </div>
              <div className="knowledge-stat">
                <span className="knowledge-stat-value">{staleCount}</span>
                <span className="knowledge-stat-label">Stale</span>
              </div>
              <div className="knowledge-stat">
                <span className="knowledge-stat-value">{sharedCount}</span>
                <span className="knowledge-stat-label">Shared</span>
              </div>
              <div className="knowledge-stat">
                <span className="knowledge-stat-value">{unscopedCount}</span>
                <span className="knowledge-stat-label">Unscoped</span>
              </div>
            </div>
          )}
        </div>

        {!isProjectLanding && (
          <>
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

            <div className="knowledge-tabs" style={{ paddingTop: 0 }}>
              {SCOPE_TABS.map(tab => (
                <button
                  key={tab.key}
                  className={`knowledge-tab${activeScope === tab.key ? ' active' : ''}`}
                  onClick={() => setActiveScope(tab.key)}
                >
                  {tab.label}
                  <span className="knowledge-tab-count">{scopeCounts[tab.key] ?? 0}</span>
                </button>
              ))}
            </div>
          </>
        )}

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

          {!loading && !error && blueprint && isProjectLanding && (
            <div className="project-landing">
              <div className="project-landing-toolbar">
                <div className="node-list-search">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--color-text-faint)" strokeWidth="2" strokeLinecap="round">
                    <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
                  </svg>
                  <input
                    type="text"
                    placeholder="Search projects by name or tag…"
                    value={projectSearch}
                    onChange={event => setProjectSearch(event.target.value)}
                    className="field-input"
                    style={{ border: 'none', background: 'transparent', padding: '4px 0', fontSize: 'var(--text-xs)' }}
                  />
                </div>
                <div className="project-landing-controls">
                  <select
                    className="field-input"
                    value={projectSort}
                    onChange={event => setProjectSort(event.target.value as ProjectSortKey)}
                    aria-label="Sort projects"
                  >
                    <option value="health_desc">Sort: Health</option>
                    <option value="activity_desc">Sort: Last activity</option>
                    <option value="knowledge_desc">Sort: Knowledge count</option>
                    <option value="stale_desc">Sort: Stale count</option>
                    <option value="name_asc">Sort: Name</option>
                  </select>
                  <label className="project-favorites-filter">
                    <input
                      type="checkbox"
                      checked={favoritesOnly}
                      onChange={event => setFavoritesOnly(event.target.checked)}
                    />
                    Favorites only
                  </label>
                </div>
              </div>

              <div className="project-landing-grid">
                <article className="project-card project-card-all-knowledge">
                  <div className="project-card-title-row">
                    <h2 className="project-card-title">All Knowledge</h2>
                  </div>
                  <p className="project-card-description">
                    Cross-project browsing for architecture-wide audits and discovery.
                  </p>
                  <div className="project-card-meta">
                    <span className="project-card-meta-item">Nodes: {nodes.length}</span>
                    <span className="project-card-meta-item">Edges: {edges.length}</span>
                    <span className="project-card-meta-item">Types: {Object.keys(blueprint.counts).length}</span>
                  </div>
                  <Link to="/knowledge/all" className="project-card-link">
                    Open All Knowledge
                  </Link>
                </article>

                {visibleProjects.map(project => {
                  const isFavorite = favoriteProjectSet.has(project.id);
                  return (
                    <article key={project.id} className="project-card">
                      <div className="project-card-title-row">
                        <div>
                          <h2 className="project-card-title">{project.name}</h2>
                          <p className="project-card-subtitle">{project.id}</p>
                        </div>
                        <button
                          type="button"
                          className={`project-favorite-btn${isFavorite ? ' active' : ''}`}
                          onClick={() => toggleFavoriteProject(project.id)}
                          aria-label={isFavorite ? `Remove ${project.name} from favorites` : `Add ${project.name} to favorites`}
                        >
                          ★
                        </button>
                      </div>

                      <p className="project-card-description">{project.description}</p>

                      <div className="project-card-meta">
                        <span className="project-card-meta-item">Knowledge: {project.totalKnowledge}</span>
                        <span className="project-card-meta-item">Local: {project.localKnowledge}</span>
                        <span className="project-card-meta-item">Shared: {project.sharedKnowledge}</span>
                        <span className="project-card-meta-item">Stale: {project.staleCount}</span>
                        <span className="project-card-meta-item">Last activity: {formatLastActivity(project.lastActivityIso)}</span>
                        <span className="project-card-meta-item">{formatRelativeActivity(project.lastActivityMs)}</span>
                        <span className={`project-health-badge health-${project.healthScore >= 80 ? 'healthy' : project.healthScore >= 55 ? 'attention' : 'risk'}`}>
                          {project.healthLabel} · {project.healthScore}%
                        </span>
                      </div>

                      <div className="project-type-counts">
                        {MAJOR_TYPES.map(type => (
                          <span key={type} className="project-type-chip">
                            {NODE_TYPE_LABELS[type]}: {project.typeCounts[type]}
                          </span>
                        ))}
                      </div>

                      <Link to={`/knowledge/projects/${encodeURIComponent(project.id)}`} className="project-card-link">
                        Open Project View
                      </Link>
                    </article>
                  );
                })}
              </div>

              {visibleProjects.length === 0 && (
                <div className="project-landing-empty">
                  No projects matched your filters. Clear search or disable favorites-only.
                </div>
              )}
            </div>
          )}

          {!loading && !error && blueprint && !isProjectLanding && (
            <NodeListPanel
              nodes={scopedNodes}
              edges={edges}
              nodeType={effectiveType}
              onSelectNode={handleSelectNode}
            />
          )}
        </div>

        {!isProjectLanding && (
          <DetailDrawer
            nodeId={selectedNodeId}
            allNodes={nodes}
            edges={edges}
            api={api}
            onClose={() => setSelectedNodeId(null)}
            onNavigateNode={handleNavigateNode}
            onImpactPreview={() => {}}
            onRequestDelete={handleRequestDelete}
            onNodeUpdated={loadBlueprint}
          />
        )}

        {!isProjectLanding && (
          <DeleteNodeDialog
            isOpen={deleteNodeId !== null}
            nodeId={deleteNodeId}
            nodeName={deleteNodeName}
            onClose={handleDeleteClose}
            onConfirm={handleConfirmDelete}
          />
        )}
      </div>
    </Layout>
  );
}
