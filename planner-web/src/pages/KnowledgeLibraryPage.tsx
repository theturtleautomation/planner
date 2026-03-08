import { useEffect, useState, useCallback, useMemo, useRef } from 'react';
import { Link, useLocation, useNavigate, useParams } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import NodeListPanel from '../components/NodeListPanel.tsx';
import DetailDrawer from '../components/DetailDrawer.tsx';
import DeleteNodeDialog from '../components/DeleteNodeDialog.tsx';
import CreateNodeModal from '../components/CreateNodeModal.tsx';
import KnowledgeFilterBar, { type KnowledgeFilterDescriptor } from '../components/KnowledgeFilterBar.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import { uuidv4 } from '../lib/uuid.ts';
import { parseKnowledgeDeepLink } from '../lib/knowledgeDeepLinks.ts';
import { labelNodeType, labelScopeClass, labelScopeVisibility, labelSecondaryScopeField } from '../lib/taxonomy.ts';
import type { BlueprintNode, BlueprintResponse, NodeSummary, NodeType, ScopeClass, ScopeVisibility } from '../types/blueprint.ts';

const MAJOR_TYPES: NodeType[] = [
  'decision',
  'technology',
  'component',
  'constraint',
  'pattern',
  'quality_requirement',
];

const NODE_TYPE_LABELS: Record<NodeType, string> = {
  decision: labelNodeType('decision', 'plural'),
  technology: labelNodeType('technology', 'plural'),
  component: labelNodeType('component', 'plural'),
  constraint: labelNodeType('constraint', 'plural'),
  pattern: labelNodeType('pattern', 'plural'),
  quality_requirement: labelNodeType('quality_requirement', 'plural'),
};

const FAVORITES_STORAGE_KEY = 'knowledge-project-favorites';
const SCOPED_FILTERS_STORAGE_PREFIX = 'knowledge-scoped-filters';
const STALE_THRESHOLD_DAYS = 30;
const LEGACY_ARCHIVED_TAG = 'archived';
const LEGACY_OVERRIDE_PREFIX = 'overrides:';
const MAX_BRANCH_ACTION_NODES = 25;

type UpdatedDateFilter = 'all' | 'last_7d' | 'last_30d' | 'last_90d' | 'older_90d';
type StaleFilter = 'all' | 'stale' | 'fresh';
type OrphanFilter = 'all' | 'orphan' | 'connected';
type DocumentationFilter = 'all' | 'with_docs' | 'without_docs';
type LifecycleFilter = 'all' | 'active' | 'archived';
type ScopeVisibilityFilter = ScopeVisibility | 'all';
type ProjectSection = 'overview' | 'inventory' | 'architecture' | 'quality' | 'activity';

interface ScopedFiltersState {
  knowledgeType: NodeType | 'all';
  scopeClass: ScopeClass | 'all';
  scopeVisibility: ScopeVisibilityFilter;
  feature: string;
  widget: string;
  artifact: string;
  component: string;
  tag: string;
  owner: string;
  status: string;
  stale: StaleFilter;
  orphan: OrphanFilter;
  documentation: DocumentationFilter;
  lifecycle: LifecycleFilter;
  updatedDate: UpdatedDateFilter;
}

interface FilterValueOption {
  value: string;
  label: string;
}

interface ActiveFilterToken {
  key: keyof ScopedFiltersState;
  label: string;
  removeLabel: string;
}

interface FilterEvaluationContext {
  linkedNodeSet: Set<string>;
  staleCutoffMs: number;
  nowMs: number;
}

const DEFAULT_SCOPED_FILTERS: ScopedFiltersState = {
  knowledgeType: 'all',
  scopeClass: 'all',
  scopeVisibility: 'all',
  feature: 'all',
  widget: 'all',
  artifact: 'all',
  component: 'all',
  tag: 'all',
  owner: 'all',
  status: 'all',
  stale: 'all',
  orphan: 'all',
  documentation: 'all',
  lifecycle: 'active',
  updatedDate: 'all',
};

const KNOWLEDGE_TYPE_FILTERS: { value: NodeType | 'all'; label: string }[] = [
  { value: 'all', label: 'All Types' },
  { value: 'decision', label: labelNodeType('decision', 'plural') },
  { value: 'technology', label: labelNodeType('technology', 'plural') },
  { value: 'component', label: labelNodeType('component', 'plural') },
  { value: 'constraint', label: labelNodeType('constraint', 'plural') },
  { value: 'pattern', label: labelNodeType('pattern', 'plural') },
  { value: 'quality_requirement', label: labelNodeType('quality_requirement', 'plural') },
];

const SCOPE_CLASS_FILTERS: { value: ScopeClass | 'all'; label: string }[] = [
  { value: 'all', label: 'All Placements' },
  { value: 'project', label: labelScopeClass('project') },
  { value: 'project_contextual', label: labelScopeClass('project_contextual') },
  { value: 'global', label: labelScopeClass('global') },
  { value: 'unscoped', label: labelScopeClass('unscoped') },
];

const SCOPE_VISIBILITY_FILTERS: { value: ScopeVisibilityFilter; label: string }[] = [
  { value: 'all', label: 'Visible: All' },
  { value: 'project_local', label: labelScopeVisibility('project_local') },
  { value: 'shared', label: labelScopeVisibility('shared') },
  { value: 'unscoped', label: `${labelScopeClass('unscoped')} only` },
];

const STALE_FILTERS: { value: StaleFilter; label: string }[] = [
  { value: 'all', label: 'Stale: Any' },
  { value: 'stale', label: 'Stale only' },
  { value: 'fresh', label: 'Fresh only' },
];

const ORPHAN_FILTERS: { value: OrphanFilter; label: string }[] = [
  { value: 'all', label: 'Orphan: Any' },
  { value: 'orphan', label: 'Orphan only' },
  { value: 'connected', label: 'Connected only' },
];

const DOC_FILTERS: { value: DocumentationFilter; label: string }[] = [
  { value: 'all', label: 'Docs: Any' },
  { value: 'with_docs', label: 'With docs' },
  { value: 'without_docs', label: 'Without docs' },
];

const LIFECYCLE_FILTERS: { value: LifecycleFilter; label: string }[] = [
  { value: 'all', label: 'Lifecycle: Any' },
  { value: 'active', label: 'Active only' },
  { value: 'archived', label: 'Archived only' },
];

const PROJECT_SECTION_TABS: { value: ProjectSection; label: string }[] = [
  { value: 'overview', label: 'Overview' },
  { value: 'inventory', label: 'Inventory' },
  { value: 'architecture', label: 'Architecture' },
  { value: 'quality', label: 'Quality' },
  { value: 'activity', label: 'Activity' },
];

const UPDATED_FILTERS: { value: UpdatedDateFilter; label: string }[] = [
  { value: 'all', label: 'Updated: Any' },
  { value: 'last_7d', label: 'Last 7 days' },
  { value: 'last_30d', label: 'Last 30 days' },
  { value: 'last_90d', label: 'Last 90 days' },
  { value: 'older_90d', label: 'Older than 90 days' },
];

const EMPTY_NODES: NodeSummary[] = [];
const EMPTY_EDGES: BlueprintResponse['edges'] = [];

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
  ownerLabel: string | null;
  teamLabel: string | null;
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
  ownerCounts: Map<string, { label: string; count: number }>;
  teamCounts: Map<string, { label: string; count: number }>;
  localKnowledge: number;
  sharedKnowledge: number;
  projectNameCounts: Map<string, number>;
}

interface ProjectEventEntry {
  id: string;
  timestamp: string;
  kind: 'mutation' | 'export';
  summary: string;
  details: string;
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

function scopedFiltersStorageKey(projectId?: string): string {
  return `${SCOPED_FILTERS_STORAGE_PREFIX}:${projectId ?? 'global'}`;
}

function normalizeFilterString(value: unknown): string {
  if (typeof value !== 'string') return 'all';
  const normalized = value.trim();
  return normalized.length > 0 ? normalized : 'all';
}

function areScopedFiltersEqual(left: ScopedFiltersState, right: ScopedFiltersState): boolean {
  return left.knowledgeType === right.knowledgeType
    && left.scopeClass === right.scopeClass
    && left.scopeVisibility === right.scopeVisibility
    && left.feature === right.feature
    && left.widget === right.widget
    && left.artifact === right.artifact
    && left.component === right.component
    && left.tag === right.tag
    && left.owner === right.owner
    && left.status === right.status
    && left.stale === right.stale
    && left.orphan === right.orphan
    && left.documentation === right.documentation
    && left.lifecycle === right.lifecycle
    && left.updatedDate === right.updatedDate;
}

function readScopedFilters(projectId?: string): ScopedFiltersState {
  if (typeof window === 'undefined') return { ...DEFAULT_SCOPED_FILTERS };
  try {
    const raw = window.localStorage.getItem(scopedFiltersStorageKey(projectId));
    if (!raw) return { ...DEFAULT_SCOPED_FILTERS };
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object') return { ...DEFAULT_SCOPED_FILTERS };
    const candidate = parsed as Partial<ScopedFiltersState>;
    const stale = candidate.stale === 'stale' || candidate.stale === 'fresh' ? candidate.stale : 'all';
    const orphan = candidate.orphan === 'orphan' || candidate.orphan === 'connected' ? candidate.orphan : 'all';
    const documentation =
      candidate.documentation === 'with_docs' || candidate.documentation === 'without_docs'
        ? candidate.documentation
        : 'all';
    const updatedDate =
      candidate.updatedDate === 'last_7d'
      || candidate.updatedDate === 'last_30d'
      || candidate.updatedDate === 'last_90d'
      || candidate.updatedDate === 'older_90d'
        ? candidate.updatedDate
        : 'all';
    const lifecycle =
      candidate.lifecycle === 'active'
      || candidate.lifecycle === 'archived'
        ? candidate.lifecycle
        : 'active';
    const scopeVisibility =
      candidate.scopeVisibility === 'shared'
      || candidate.scopeVisibility === 'project_local'
      || candidate.scopeVisibility === 'unscoped'
        ? candidate.scopeVisibility
        : 'all';

    const scopeClass =
      candidate.scopeClass === 'project'
      || candidate.scopeClass === 'project_contextual'
      || candidate.scopeClass === 'global'
      || candidate.scopeClass === 'unscoped'
        ? candidate.scopeClass
        : 'all';

    const knowledgeType = MAJOR_TYPES.includes(candidate.knowledgeType as NodeType)
      ? (candidate.knowledgeType as NodeType)
      : 'all';

    return {
      knowledgeType,
      scopeClass,
      scopeVisibility,
      feature: normalizeFilterString(candidate.feature),
      widget: normalizeFilterString(candidate.widget),
      artifact: normalizeFilterString(candidate.artifact),
      component: normalizeFilterString(candidate.component),
      tag: normalizeFilterString(candidate.tag),
      owner: normalizeFilterString(candidate.owner),
      status: normalizeFilterString(candidate.status),
      stale,
      orphan,
      documentation,
      lifecycle,
      updatedDate,
    };
  } catch {
    return { ...DEFAULT_SCOPED_FILTERS };
  }
}

function buildFilterValueOptions(values: string[]): FilterValueOption[] {
  const counts = new Map<string, FilterValueOption & { seedCount: number }>();
  for (const rawValue of values) {
    const normalized = rawValue.trim();
    if (!normalized) continue;
    const key = normalized.toLowerCase();
    const existing = counts.get(key);
    if (existing) {
      existing.seedCount += 1;
      continue;
    }
    counts.set(key, { value: key, label: normalized, seedCount: 1 });
  }
  return Array.from(counts.values())
    .sort((a, b) => b.seedCount - a.seedCount || a.label.localeCompare(b.label))
    .map(({ value, label }) => ({ value, label }));
}

function extractTagValue(rawTag: string, prefixes: string[]): string | null {
  const tag = rawTag.trim();
  if (!tag) return null;
  const lower = tag.toLowerCase();

  for (const prefix of prefixes) {
    if (lower.startsWith(`${prefix}:`) || lower.startsWith(`${prefix}=`)) {
      const separatorIndex = tag.indexOf(':') >= 0 ? tag.indexOf(':') : tag.indexOf('=');
      const value = tag.slice(separatorIndex + 1).trim();
      if (value) return value;
    }
  }

  return null;
}

function extractOwnerLabel(node: NodeSummary): string | null {
  for (const rawTag of node.tags) {
    const fromPrefix = extractTagValue(rawTag, ['owner']);
    if (fromPrefix) return fromPrefix;
    const tag = rawTag.trim();
    if (tag.startsWith('@') && tag.length > 1) {
      return tag.slice(1);
    }
  }
  return null;
}

function extractTeamLabel(node: NodeSummary): string | null {
  for (const rawTag of node.tags) {
    const value = extractTagValue(rawTag, ['team', 'owning-team', 'owning_team', 'squad']);
    if (value) return value;
  }
  return null;
}

function isProjectSignalTag(rawTag: string): boolean {
  const tag = rawTag.trim().toLowerCase();
  return Boolean(tag)
    && !tag.startsWith('owner:')
    && !tag.startsWith('owner=')
    && !tag.startsWith('team:')
    && !tag.startsWith('team=')
    && !tag.startsWith('owning-team:')
    && !tag.startsWith('owning-team=')
    && !tag.startsWith('owning_team:')
    && !tag.startsWith('owning_team=')
    && tag !== LEGACY_ARCHIVED_TAG
    && tag !== 'branch'
    && !tag.startsWith('lineage:branch-of:')
    && !tag.startsWith(LEGACY_OVERRIDE_PREFIX);
}

function pickTopLabel(map: Map<string, { label: string; count: number }>): string | null {
  const winner = Array.from(map.values())
    .sort((a, b) => b.count - a.count || a.label.localeCompare(b.label))[0];
  return winner?.label ?? null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function readString(value: unknown): string | null {
  return typeof value === 'string' && value.trim() ? value.trim() : null;
}

function readStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value
    .filter((entry): entry is string => typeof entry === 'string')
    .map(entry => entry.trim())
    .filter(Boolean);
}

function nodeTypeLabel(nodeType: string | null): string {
  if (!nodeType) return 'record';
  if (nodeType in NODE_TYPE_LABELS) {
    const pluralLabel = NODE_TYPE_LABELS[nodeType as NodeType];
    return pluralLabel.endsWith('s') ? pluralLabel.slice(0, -1).toLowerCase() : pluralLabel.toLowerCase();
  }
  return nodeType.replace(/_/g, ' ');
}

function eventNodeName(node: Record<string, unknown> | null): string {
  if (!node) return 'record';
  return readString(node.name)
    ?? readString(node.title)
    ?? readString(node.scenario)
    ?? readString(node.id)
    ?? 'record';
}

function eventNodeTags(node: Record<string, unknown> | null): string[] {
  if (!node) return [];
  return readStringArray(node.tags);
}

function eventNodeLifecycle(node: Record<string, unknown> | null): 'active' | 'archived' {
  if (!node) return 'active';
  const scope = isRecord(node.scope) ? node.scope : null;
  const lifecycle = readString(scope?.lifecycle);
  if (lifecycle === 'archived') return 'archived';
  if (lifecycle === 'active') return 'active';
  const hasLegacyArchivedTag = eventNodeTags(node)
    .some(tag => tag.trim().toLowerCase() === LEGACY_ARCHIVED_TAG);
  return hasLegacyArchivedTag ? 'archived' : 'active';
}

function eventNodeMatchesProject(node: Record<string, unknown> | null, projectId: string): boolean {
  if (!node) return false;
  const scope = isRecord(node.scope) ? node.scope : null;
  if (!scope) return false;
  const project = isRecord(scope.project) ? scope.project : null;
  if (readString(project?.project_id) === projectId) {
    return true;
  }
  const shared = isRecord(scope.shared) ? scope.shared : null;
  return readStringArray(shared?.linked_project_ids).includes(projectId);
}

function summarizeProjectEvent(
  event: { event_type: string; summary: string; timestamp: string; data: Record<string, unknown> },
  projectId: string,
  projectNodeIds: Set<string>,
): ProjectEventEntry | null {
  switch (event.event_type) {
    case 'node_created': {
      const node = isRecord(event.data.node) ? event.data.node : null;
      if (!eventNodeMatchesProject(node, projectId)) return null;
      const tags = eventNodeTags(node);
      const branched = tags.some(tag => tag.trim().toLowerCase() === 'branch')
        || tags.some(tag => tag.trim().toLowerCase().startsWith('lineage:branch-of:'));
      const name = eventNodeName(node);
      const type = nodeTypeLabel(readString(node?.node_type));
      return {
        id: `event:${event.timestamp}:${event.summary}`,
        timestamp: event.timestamp,
        kind: 'mutation',
        summary: branched ? `Branched ${type} '${name}'` : `Created ${type} '${name}'`,
        details: event.summary,
      };
    }
    case 'node_updated': {
      const before = isRecord(event.data.before) ? event.data.before : null;
      const after = isRecord(event.data.after) ? event.data.after : null;
      if (!eventNodeMatchesProject(after, projectId) && !eventNodeMatchesProject(before, projectId)) return null;
      const beforeArchived = eventNodeLifecycle(before) === 'archived';
      const afterArchived = eventNodeLifecycle(after) === 'archived';
      const name = eventNodeName(after ?? before);
      const type = nodeTypeLabel(readString(after?.node_type) ?? readString(before?.node_type));
      const summary = afterArchived && !beforeArchived
        ? `Archived ${type} '${name}'`
        : !afterArchived && beforeArchived
          ? `Restored ${type} '${name}'`
          : `Updated ${type} '${name}'`;
      return {
        id: `event:${event.timestamp}:${event.summary}`,
        timestamp: event.timestamp,
        kind: 'mutation',
        summary,
        details: event.summary,
      };
    }
    case 'node_deleted': {
      const node = isRecord(event.data.node) ? event.data.node : null;
      if (!eventNodeMatchesProject(node, projectId)) return null;
      const name = eventNodeName(node);
      const type = nodeTypeLabel(readString(node?.node_type));
      return {
        id: `event:${event.timestamp}:${event.summary}`,
        timestamp: event.timestamp,
        kind: 'mutation',
        summary: `Deleted ${type} '${name}'`,
        details: event.summary,
      };
    }
    case 'edge_created': {
      const edge = isRecord(event.data.edge) ? event.data.edge : null;
      const source = readString(edge?.source);
      const target = readString(edge?.target);
      if (!source || !target) return null;
      if (!projectNodeIds.has(source) && !projectNodeIds.has(target)) return null;
      return {
        id: `event:${event.timestamp}:${event.summary}`,
        timestamp: event.timestamp,
        kind: 'mutation',
        summary: 'Created relationship',
        details: event.summary,
      };
    }
    case 'edges_deleted': {
      const edges = Array.isArray(event.data.edges) ? event.data.edges : [];
      const relevant = edges.some(edge => {
        if (!isRecord(edge)) return false;
        const source = readString(edge.source);
        const target = readString(edge.target);
        return Boolean(source && projectNodeIds.has(source)) || Boolean(target && projectNodeIds.has(target));
      });
      if (!relevant) return null;
      return {
        id: `event:${event.timestamp}:${event.summary}`,
        timestamp: event.timestamp,
        kind: 'mutation',
        summary: 'Removed relationship',
        details: event.summary,
      };
    }
    case 'export_recorded': {
      const exportProjectId = readString(event.data.project_id);
      if (exportProjectId !== projectId) return null;
      const kind = readString(event.data.kind) ?? 'scoped_view';
      const nodeCount = typeof event.data.node_count === 'number' ? event.data.node_count : 0;
      const edgeCount = typeof event.data.edge_count === 'number' ? event.data.edge_count : 0;
      const nodeId = readString(event.data.node_id);
      const summary = kind === 'single_record'
        ? `Exported single record${nodeId ? ` (${nodeId})` : ''}`
        : `Exported scoped view (${nodeCount} records)`;
      return {
        id: `event:${event.timestamp}:${event.summary}`,
        timestamp: event.timestamp,
        kind: 'export',
        summary,
        details: `${event.summary} · ${nodeCount} nodes · ${edgeCount} edges`,
      };
    }
    default:
      return null;
  }
}

function isArchivedNode(node: NodeSummary): boolean {
  if (node.lifecycle === 'archived') return true;
  return node.tags.some(tag => tag.trim().toLowerCase() === LEGACY_ARCHIVED_TAG);
}

function normalizeTags(tags: string[]): string[] {
  const deduped = new Map<string, string>();
  for (const rawTag of tags) {
    const trimmed = rawTag.trim();
    if (!trimmed) continue;
    const key = trimmed.toLowerCase();
    if (!deduped.has(key)) {
      deduped.set(key, trimmed);
    }
  }
  return Array.from(deduped.values());
}

function withBranchSuffix(label: string): string {
  return label.endsWith(' (branch)') ? label : `${label} (branch)`;
}

function nodeDisplayName(node: BlueprintNode): string {
  switch (node.node_type) {
    case 'decision':
    case 'constraint':
      return node.title;
    case 'quality_requirement':
      return node.scenario;
    default:
      return node.name;
  }
}

function toBranchNode(base: BlueprintNode): BlueprintNode {
  const now = new Date().toISOString();
  const clone = structuredClone(base) as BlueprintNode & {
    id: string;
    tags: string[];
    name?: string;
    title?: string;
    created_at: string;
    updated_at: string;
  };

  clone.id = `${base.id}-branch-${uuidv4().slice(0, 8)}`;
  if (typeof clone.name === 'string') clone.name = withBranchSuffix(clone.name);
  if (typeof clone.title === 'string') clone.title = withBranchSuffix(clone.title);
  clone.tags = normalizeTags([...(clone.tags ?? []), 'branch', `lineage:branch-of:${base.id}`]);
  clone.created_at = now;
  clone.updated_at = now;
  return clone;
}

function nodeMatchesScopedFilters(
  node: NodeSummary,
  scopedFilters: ScopedFiltersState,
  context: FilterEvaluationContext,
): boolean {
  const knowledgeType = scopedFilters.knowledgeType;
  const scopeClass = scopedFilters.scopeClass;
  const scopeVisibility = scopedFilters.scopeVisibility;
  const feature = scopedFilters.feature.toLowerCase();
  const widget = scopedFilters.widget.toLowerCase();
  const artifact = scopedFilters.artifact.toLowerCase();
  const component = scopedFilters.component.toLowerCase();
  const tag = scopedFilters.tag.toLowerCase();
  const owner = scopedFilters.owner.toLowerCase();
  const status = scopedFilters.status.toLowerCase();
  const lifecycle = scopedFilters.lifecycle;

  if (knowledgeType !== 'all' && node.node_type !== knowledgeType) return false;
  if (scopeClass !== 'all' && (node.scope_class ?? 'unscoped') !== scopeClass) return false;
  const nodeScopeVisibility = node.scope_visibility ?? (node.is_shared ? 'shared' : 'unscoped');
  if (scopeVisibility !== 'all' && nodeScopeVisibility !== scopeVisibility) return false;

  const secondaryScope = node.secondary_scope ?? {};
  if (feature !== 'all' && (secondaryScope.feature ?? '').trim().toLowerCase() !== feature) return false;
  if (widget !== 'all' && (secondaryScope.widget ?? '').trim().toLowerCase() !== widget) return false;
  if (artifact !== 'all' && (secondaryScope.artifact ?? '').trim().toLowerCase() !== artifact) return false;
  if (component !== 'all' && (secondaryScope.component ?? '').trim().toLowerCase() !== component) return false;

  if (tag !== 'all' && !node.tags.some(nodeTag => nodeTag.trim().toLowerCase() === tag)) return false;
  if (owner !== 'all' && (extractOwnerLabel(node)?.toLowerCase() ?? '') !== owner) return false;
  if (status !== 'all' && node.status.trim().toLowerCase() !== status) return false;

  const archived = isArchivedNode(node);
  if (lifecycle === 'active' && archived) return false;
  if (lifecycle === 'archived' && !archived) return false;

  const updatedMs = parseIsoTimeMs(node.updated_at);
  const nodeIsStale = updatedMs <= context.staleCutoffMs;
  if (scopedFilters.stale === 'stale' && !nodeIsStale) return false;
  if (scopedFilters.stale === 'fresh' && nodeIsStale) return false;

  const orphan = !context.linkedNodeSet.has(node.id);
  if (scopedFilters.orphan === 'orphan' && !orphan) return false;
  if (scopedFilters.orphan === 'connected' && orphan) return false;

  if (scopedFilters.documentation === 'with_docs' && !node.has_documentation) return false;
  if (scopedFilters.documentation === 'without_docs' && node.has_documentation) return false;

  switch (scopedFilters.updatedDate) {
    case 'last_7d':
      if (updatedMs < context.nowMs - 7 * 24 * 60 * 60 * 1000) return false;
      break;
    case 'last_30d':
      if (updatedMs < context.nowMs - 30 * 24 * 60 * 60 * 1000) return false;
      break;
    case 'last_90d':
      if (updatedMs < context.nowMs - 90 * 24 * 60 * 60 * 1000) return false;
      break;
    case 'older_90d':
      if (updatedMs >= context.nowMs - 90 * 24 * 60 * 60 * 1000) return false;
      break;
    case 'all':
    default:
      break;
  }

  return true;
}

// ─── Page Component ─────────────────────────────────────────────────────────

export default function KnowledgeLibraryPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const { projectId: routeProjectId } = useParams<{ projectId: string }>();
  const deepLink = useMemo(() => parseKnowledgeDeepLink(location.search), [location.search]);
  const projectId = routeProjectId?.trim() || undefined;
  const isProjectScoped = Boolean(projectId);
  const normalizedPath = location.pathname.replace(/\/+$/, '') || '/';
  const isGlobalView = !isProjectScoped && normalizedPath === '/knowledge/all';
  const isProjectLanding = !isProjectScoped && !isGlobalView;

  const getToken = useGetAccessToken();
  const getTokenRef = useRef(getToken);
  useEffect(() => {
    getTokenRef.current = getToken;
  }, [getToken]);
  const api = useMemo(
    () => createApiClient(() => getTokenRef.current()),
    [],
  );

  const [blueprint, setBlueprint] = useState<BlueprintResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [selectedNodeIds, setSelectedNodeIds] = useState<string[]>([]);
  const [projectSearch, setProjectSearch] = useState('');
  const [projectSort, setProjectSort] = useState<ProjectSortKey>('health_desc');
  const [favoritesOnly, setFavoritesOnly] = useState(false);
  const [favoriteProjectIds, setFavoriteProjectIds] = useState<string[]>(() => readFavoriteProjectIds());
  const [scopedFilters, setScopedFilters] = useState<ScopedFiltersState>(() => readScopedFilters(projectId));
  const [actionBusy, setActionBusy] = useState<'archive' | 'restore' | 'export' | 'branch' | 'create' | null>(null);
  const [actionNotice, setActionNotice] = useState<string | null>(null);
  const [projectSection, setProjectSection] = useState<ProjectSection>('overview');
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [projectEvents, setProjectEvents] = useState<ProjectEventEntry[]>([]);
  const [reviewBusyNodeId, setReviewBusyNodeId] = useState<string | null>(null);

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
      if (isProjectScoped && projectId) {
        try {
          const eventResponse = await api.listBlueprintEvents({ limit: 250 });
          const projectNodeIds = new Set(data.nodes.map(node => node.id));
          const nextProjectEvents = eventResponse.events
            .map(event => summarizeProjectEvent(event, projectId, projectNodeIds))
            .filter((entry): entry is ProjectEventEntry => entry !== null)
            .slice(0, 40);
          setProjectEvents(nextProjectEvents);
        } catch {
          setProjectEvents([]);
        }
      } else {
        setProjectEvents([]);
      }
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

  useEffect(() => {
    if (isProjectScoped) return;
    if (!deepLink.projectId) return;
    const query = location.search ? `?${location.search.replace(/^\?/, '')}` : '';
    navigate(`/knowledge/projects/${encodeURIComponent(deepLink.projectId)}${query}`, { replace: true });
  }, [deepLink.projectId, isProjectScoped, location.search, navigate]);

  useEffect(() => {
    const nextScopedFilters = readScopedFilters(projectId);
    setScopedFilters(previous => (
      areScopedFiltersEqual(previous, nextScopedFilters) ? previous : nextScopedFilters
    ));
    setSelectedNodeIds(previous => (previous.length === 0 ? previous : []));
    setActionNotice(previous => (previous === null ? previous : null));
    setProjectEvents([]);
    setProjectSection('overview');
  }, [projectId]);

  useEffect(() => {
    if (typeof window === 'undefined' || isProjectLanding) return;
    window.localStorage.setItem(scopedFiltersStorageKey(projectId), JSON.stringify(scopedFilters));
  }, [isProjectLanding, projectId, scopedFilters]);

  useEffect(() => {
    if (!isProjectScoped || !projectId) return;
    if (!deepLink.hasContextFilters) return;
    if (deepLink.projectId && deepLink.projectId !== projectId) return;

    const nextScopedFilters: ScopedFiltersState = {
      ...DEFAULT_SCOPED_FILTERS,
      feature: normalizeFilterString(deepLink.filters.feature),
      widget: normalizeFilterString(deepLink.filters.widget),
      artifact: normalizeFilterString(deepLink.filters.artifact),
      component: normalizeFilterString(deepLink.filters.component),
    };

    setScopedFilters(previous => (
      areScopedFiltersEqual(previous, nextScopedFilters) ? previous : nextScopedFilters
    ));
    setSelectedNodeIds(previous => (previous.length === 0 ? previous : []));
    setActionNotice(previous => (previous === null ? previous : null));
  }, [
    deepLink.filters.artifact,
    deepLink.filters.component,
    deepLink.filters.feature,
    deepLink.filters.widget,
    deepLink.hasContextFilters,
    deepLink.projectId,
    isProjectScoped,
    projectId,
  ]);

  useEffect(() => {
    if (!actionNotice) return;
    const timeout = window.setTimeout(() => setActionNotice(null), 4000);
    return () => window.clearTimeout(timeout);
  }, [actionNotice]);

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

  const nodes = blueprint?.nodes ?? EMPTY_NODES;
  const edges = blueprint?.edges ?? EMPTY_EDGES;
  const scopedProjectName = useMemo(() => {
    if (!projectId) return null;
    const matching = nodes.find(node => node.project_id === projectId && node.project_name?.trim());
    return matching?.project_name?.trim() ?? projectId;
  }, [nodes, projectId]);
  const selectedNodeSet = useMemo(() => new Set(selectedNodeIds), [selectedNodeIds]);
  const linkedNodeSet = useMemo(() => {
    const linked = new Set<string>();
    for (const edge of edges) {
      linked.add(edge.source);
      linked.add(edge.target);
    }
    return linked;
  }, [edges]);

  const staleCutoffMs = Date.now() - STALE_THRESHOLD_DAYS * 24 * 60 * 60 * 1000;
  const filterEvaluationContext = useMemo(
    () => ({
      linkedNodeSet,
      staleCutoffMs,
      nowMs: Date.now(),
    }),
    [linkedNodeSet, staleCutoffMs],
  );
  const filteredNodes = useMemo(() => {
    return nodes.filter(node => nodeMatchesScopedFilters(node, scopedFilters, filterEvaluationContext));
  }, [filterEvaluationContext, nodes, scopedFilters]);

  const filteredNodeIdSet = useMemo(() => new Set(filteredNodes.map(node => node.id)), [filteredNodes]);
  const filteredEdges = useMemo(
    () => edges.filter(edge => filteredNodeIdSet.has(edge.source) && filteredNodeIdSet.has(edge.target)),
    [edges, filteredNodeIdSet],
  );

  const sectionFilteredNodes = useMemo(() => {
    if (!isProjectScoped) return filteredNodes;
    switch (projectSection) {
      case 'inventory':
        return filteredNodes.filter(node => node.node_type === 'component' || node.node_type === 'technology');
      case 'architecture':
        return filteredNodes.filter(node => (
          node.node_type === 'decision'
          || node.node_type === 'constraint'
          || node.node_type === 'pattern'
        ));
      case 'quality':
        return filteredNodes.filter(node => node.node_type === 'quality_requirement');
      case 'activity':
      case 'overview':
      default:
        return filteredNodes;
    }
  }, [filteredNodes, isProjectScoped, projectSection]);

  const sectionNodeIdSet = useMemo(() => new Set(sectionFilteredNodes.map(node => node.id)), [sectionFilteredNodes]);
  const sectionFilteredEdges = useMemo(
    () => edges.filter(edge => sectionNodeIdSet.has(edge.source) && sectionNodeIdSet.has(edge.target)),
    [edges, sectionNodeIdSet],
  );

  const lifecycleCounts = useMemo(() => {
    const archived = nodes.filter(isArchivedNode).length;
    const active = nodes.length - archived;
    return {
      all: nodes.length,
      active,
      archived,
    };
  }, [nodes]);

  const sharedCount = useMemo(() => nodes.filter(n => n.scope_visibility === 'shared' || n.is_shared).length, [nodes]);
  const unscopedCount = useMemo(
    () => nodes.filter(node => (node.scope_class ?? 'unscoped') === 'unscoped').length,
    [nodes],
  );
  const staleCount = useMemo(() => {
    return nodes.filter(node => parseIsoTimeMs(node.updated_at) <= staleCutoffMs).length;
  }, [nodes, staleCutoffMs]);
  const orphanCount = useMemo(() => nodes.filter(node => !linkedNodeSet.has(node.id)).length, [linkedNodeSet, nodes]);
  const missingScopeCount = useMemo(() => nodes.filter(node => (node.scope_class ?? 'unscoped') === 'unscoped').length, [nodes]);
  const missingDocsCount = useMemo(() => nodes.filter(node => !node.has_documentation).length, [nodes]);
  const archivedCount = lifecycleCounts.archived;
  const recentlyChangedCount = useMemo(() => {
    const cutoff = Date.now() - 7 * 24 * 60 * 60 * 1000;
    return nodes.filter(node => parseIsoTimeMs(node.updated_at) >= cutoff).length;
  }, [nodes]);

  const featureValueOptions = useMemo(
    () => buildFilterValueOptions(nodes.map(node => node.secondary_scope?.feature ?? '')),
    [nodes],
  );
  const widgetValueOptions = useMemo(
    () => buildFilterValueOptions(nodes.map(node => node.secondary_scope?.widget ?? '')),
    [nodes],
  );
  const artifactValueOptions = useMemo(
    () => buildFilterValueOptions(nodes.map(node => node.secondary_scope?.artifact ?? '')),
    [nodes],
  );
  const componentValueOptions = useMemo(
    () => buildFilterValueOptions(nodes.map(node => node.secondary_scope?.component ?? '')),
    [nodes],
  );
  const tagValueOptions = useMemo(
    () => buildFilterValueOptions(nodes.flatMap(node => node.tags)),
    [nodes],
  );
  const ownerValueOptions = useMemo(
    () => buildFilterValueOptions(nodes.map(node => extractOwnerLabel(node) ?? '')),
    [nodes],
  );
  const statusValueOptions = useMemo(
    () => buildFilterValueOptions(nodes.map(node => node.status)),
    [nodes],
  );

  const countNodesForFilters = useCallback((candidateFilters: ScopedFiltersState): number => {
    let count = 0;
    for (const node of nodes) {
      if (nodeMatchesScopedFilters(node, candidateFilters, filterEvaluationContext)) {
        count += 1;
      }
    }
    return count;
  }, [filterEvaluationContext, nodes]);

  const buildDescriptorOptions = useCallback(<K extends keyof ScopedFiltersState>(
    key: K,
    baseOptions: Array<{ value: string; label: string }>,
  ) => {
    const normalizedOptions = [...baseOptions];
    const currentValue = String(scopedFilters[key]);
    if (!normalizedOptions.some(option => option.value === currentValue)) {
      normalizedOptions.push({
        value: currentValue,
        label: currentValue === 'all' ? 'All' : currentValue,
      });
    }

    return normalizedOptions.map(option => {
      const nextFilters = {
        ...scopedFilters,
        [key]: option.value,
      } as ScopedFiltersState;
      return {
        value: option.value,
        label: option.label,
        count: countNodesForFilters(nextFilters),
      };
    });
  }, [countNodesForFilters, scopedFilters]);

  const knowledgeFilterDescriptors = useMemo<KnowledgeFilterDescriptor[]>(() => {
    return [
      {
        key: 'knowledgeType',
        label: 'Type',
        shortLabel: 'Type',
        placement: 'primary',
        value: scopedFilters.knowledgeType,
        options: buildDescriptorOptions(
          'knowledgeType',
          KNOWLEDGE_TYPE_FILTERS.map(option => ({ value: option.value, label: option.label })),
        ),
        onChange: (value) => {
          setScopedFilters(previous => ({ ...previous, knowledgeType: value as ScopedFiltersState['knowledgeType'] }));
        },
      },
      {
        key: 'feature',
        label: labelSecondaryScopeField('feature'),
        shortLabel: labelSecondaryScopeField('feature'),
        placement: 'primary',
        value: scopedFilters.feature,
        options: buildDescriptorOptions('feature', [
          { value: 'all', label: 'All Feature Areas' },
          ...featureValueOptions,
        ]),
        onChange: (value) => {
          setScopedFilters(previous => ({ ...previous, feature: value }));
        },
      },
      {
        key: 'widget',
        label: labelSecondaryScopeField('widget'),
        shortLabel: labelSecondaryScopeField('widget'),
        placement: 'primary',
        value: scopedFilters.widget,
        options: buildDescriptorOptions('widget', [
          { value: 'all', label: 'All Surfaces' },
          ...widgetValueOptions,
        ]),
        onChange: (value) => {
          setScopedFilters(previous => ({ ...previous, widget: value }));
        },
      },
      {
        key: 'artifact',
        label: 'Artifact',
        shortLabel: 'Artifact',
        placement: 'primary',
        value: scopedFilters.artifact,
        options: buildDescriptorOptions('artifact', [
          { value: 'all', label: 'All Artifacts' },
          ...artifactValueOptions,
        ]),
        onChange: (value) => {
          setScopedFilters(previous => ({ ...previous, artifact: value }));
        },
      },
      {
        key: 'component',
        label: labelSecondaryScopeField('component'),
        shortLabel: labelSecondaryScopeField('component'),
        placement: 'primary',
        value: scopedFilters.component,
        options: buildDescriptorOptions('component', [
          { value: 'all', label: 'All Related Components' },
          ...componentValueOptions,
        ]),
        onChange: (value) => {
          setScopedFilters(previous => ({ ...previous, component: value }));
        },
      },
      {
        key: 'scopeClass',
        label: 'Placement',
        shortLabel: 'Placement',
        placement: 'overflow',
        value: scopedFilters.scopeClass,
        options: buildDescriptorOptions(
          'scopeClass',
          SCOPE_CLASS_FILTERS.map(option => ({ value: option.value, label: option.label })),
        ),
        onChange: (value) => {
          setScopedFilters(previous => ({ ...previous, scopeClass: value as ScopedFiltersState['scopeClass'] }));
        },
      },
      {
        key: 'scopeVisibility',
        label: 'Availability',
        shortLabel: 'Availability',
        placement: 'overflow',
        value: scopedFilters.scopeVisibility,
        options: buildDescriptorOptions(
          'scopeVisibility',
          SCOPE_VISIBILITY_FILTERS.map(option => ({ value: option.value, label: option.label })),
        ),
        onChange: (value) => {
          setScopedFilters(previous => ({ ...previous, scopeVisibility: value as ScopedFiltersState['scopeVisibility'] }));
        },
      },
      {
        key: 'owner',
        label: 'Owner',
        shortLabel: 'Owner',
        placement: 'overflow',
        value: scopedFilters.owner,
        options: buildDescriptorOptions('owner', [
          { value: 'all', label: 'All Owners' },
          ...ownerValueOptions,
        ]),
        onChange: (value) => {
          setScopedFilters(previous => ({ ...previous, owner: value }));
        },
      },
      {
        key: 'tag',
        label: 'Tag',
        shortLabel: 'Tag',
        placement: 'overflow',
        value: scopedFilters.tag,
        options: buildDescriptorOptions('tag', [
          { value: 'all', label: 'All Tags' },
          ...tagValueOptions,
        ]),
        onChange: (value) => {
          setScopedFilters(previous => ({ ...previous, tag: value }));
        },
      },
      {
        key: 'status',
        label: 'Status',
        shortLabel: 'Status',
        placement: 'overflow',
        value: scopedFilters.status,
        options: buildDescriptorOptions('status', [
          { value: 'all', label: 'Any Status' },
          ...statusValueOptions,
        ]),
        onChange: (value) => {
          setScopedFilters(previous => ({ ...previous, status: value }));
        },
      },
      {
        key: 'stale',
        label: 'Freshness',
        shortLabel: 'Freshness',
        placement: 'overflow',
        value: scopedFilters.stale,
        options: buildDescriptorOptions(
          'stale',
          STALE_FILTERS.map(option => ({ value: option.value, label: option.label })),
        ),
        onChange: (value) => {
          setScopedFilters(previous => ({ ...previous, stale: value as ScopedFiltersState['stale'] }));
        },
      },
      {
        key: 'orphan',
        label: 'Connectivity',
        shortLabel: 'Connectivity',
        placement: 'overflow',
        value: scopedFilters.orphan,
        options: buildDescriptorOptions(
          'orphan',
          ORPHAN_FILTERS.map(option => ({ value: option.value, label: option.label })),
        ),
        onChange: (value) => {
          setScopedFilters(previous => ({ ...previous, orphan: value as ScopedFiltersState['orphan'] }));
        },
      },
      {
        key: 'documentation',
        label: 'Docs',
        shortLabel: 'Docs',
        placement: 'overflow',
        value: scopedFilters.documentation,
        options: buildDescriptorOptions(
          'documentation',
          DOC_FILTERS.map(option => ({ value: option.value, label: option.label })),
        ),
        onChange: (value) => {
          setScopedFilters(previous => ({ ...previous, documentation: value as ScopedFiltersState['documentation'] }));
        },
      },
      {
        key: 'lifecycle',
        label: 'Lifecycle',
        shortLabel: 'Lifecycle',
        placement: 'overflow',
        value: scopedFilters.lifecycle,
        options: buildDescriptorOptions(
          'lifecycle',
          LIFECYCLE_FILTERS.map(option => ({ value: option.value, label: option.label })),
        ),
        onChange: (value) => {
          setScopedFilters(previous => ({ ...previous, lifecycle: value as ScopedFiltersState['lifecycle'] }));
        },
      },
      {
        key: 'updatedDate',
        label: 'Updated',
        shortLabel: 'Updated',
        placement: 'overflow',
        value: scopedFilters.updatedDate,
        options: buildDescriptorOptions(
          'updatedDate',
          UPDATED_FILTERS.map(option => ({ value: option.value, label: option.label })),
        ),
        onChange: (value) => {
          setScopedFilters(previous => ({ ...previous, updatedDate: value as ScopedFiltersState['updatedDate'] }));
        },
      },
    ];
  }, [
    artifactValueOptions,
    buildDescriptorOptions,
    componentValueOptions,
    featureValueOptions,
    ownerValueOptions,
    scopedFilters.artifact,
    scopedFilters.component,
    scopedFilters.documentation,
    scopedFilters.feature,
    scopedFilters.knowledgeType,
    scopedFilters.lifecycle,
    scopedFilters.orphan,
    scopedFilters.owner,
    scopedFilters.scopeClass,
    scopedFilters.scopeVisibility,
    scopedFilters.stale,
    scopedFilters.status,
    scopedFilters.tag,
    scopedFilters.updatedDate,
    scopedFilters.widget,
    statusValueOptions,
    tagValueOptions,
    widgetValueOptions,
  ]);

  const selectedNodes = useMemo(() => nodes.filter(node => selectedNodeSet.has(node.id)), [nodes, selectedNodeSet]);
  const selectedArchivedCount = useMemo(() => selectedNodes.filter(isArchivedNode).length, [selectedNodes]);
  const exportTargetNodeId = useMemo(() => {
    if (selectedNodeIds.length === 1) return selectedNodeIds[0];
    return selectedNodeId;
  }, [selectedNodeId, selectedNodeIds]);
  const exportTargetLabel = useMemo(
    () => nodes.find(node => node.id === exportTargetNodeId)?.name ?? exportTargetNodeId,
    [exportTargetNodeId, nodes],
  );

  const activeFilterTokens = useMemo(() => {
    const optionLabel = (key: keyof ScopedFiltersState, value: string): string => {
      const descriptor = knowledgeFilterDescriptors.find(entry => entry.key === key);
      const matched = descriptor?.options.find(option => option.value === value);
      return matched?.label ?? value;
    };

    const tokens: ActiveFilterToken[] = [];
    if (scopedFilters.knowledgeType !== DEFAULT_SCOPED_FILTERS.knowledgeType) {
      tokens.push({
        key: 'knowledgeType',
        label: `Type: ${optionLabel('knowledgeType', scopedFilters.knowledgeType)}`,
        removeLabel: `Remove filter: Type ${optionLabel('knowledgeType', scopedFilters.knowledgeType)}`,
      });
    }
    if (scopedFilters.scopeClass !== DEFAULT_SCOPED_FILTERS.scopeClass) {
      const label = optionLabel('scopeClass', scopedFilters.scopeClass);
      tokens.push({
        key: 'scopeClass',
        label: `Placement: ${label}`,
        removeLabel: `Remove filter: Placement ${label}`,
      });
    }
    if (scopedFilters.scopeVisibility !== DEFAULT_SCOPED_FILTERS.scopeVisibility) {
      const label = optionLabel('scopeVisibility', scopedFilters.scopeVisibility);
      tokens.push({
        key: 'scopeVisibility',
        label: `Availability: ${label}`,
        removeLabel: `Remove filter: Availability ${label}`,
      });
    }
    if (scopedFilters.feature !== DEFAULT_SCOPED_FILTERS.feature) {
      const label = optionLabel('feature', scopedFilters.feature);
      tokens.push({
        key: 'feature',
        label: `${labelSecondaryScopeField('feature')}: ${label}`,
        removeLabel: `Remove filter: ${labelSecondaryScopeField('feature')} ${label}`,
      });
    }
    if (scopedFilters.widget !== DEFAULT_SCOPED_FILTERS.widget) {
      const label = optionLabel('widget', scopedFilters.widget);
      tokens.push({
        key: 'widget',
        label: `${labelSecondaryScopeField('widget')}: ${label}`,
        removeLabel: `Remove filter: ${labelSecondaryScopeField('widget')} ${label}`,
      });
    }
    if (scopedFilters.artifact !== DEFAULT_SCOPED_FILTERS.artifact) {
      const label = optionLabel('artifact', scopedFilters.artifact);
      tokens.push({
        key: 'artifact',
        label: `Artifact: ${label}`,
        removeLabel: `Remove filter: Artifact ${label}`,
      });
    }
    if (scopedFilters.component !== DEFAULT_SCOPED_FILTERS.component) {
      const label = optionLabel('component', scopedFilters.component);
      tokens.push({
        key: 'component',
        label: `${labelSecondaryScopeField('component')}: ${label}`,
        removeLabel: `Remove filter: ${labelSecondaryScopeField('component')} ${label}`,
      });
    }
    if (scopedFilters.tag !== DEFAULT_SCOPED_FILTERS.tag) {
      const label = optionLabel('tag', scopedFilters.tag);
      tokens.push({
        key: 'tag',
        label: `Tag: ${label}`,
        removeLabel: `Remove filter: Tag ${label}`,
      });
    }
    if (scopedFilters.owner !== DEFAULT_SCOPED_FILTERS.owner) {
      const label = optionLabel('owner', scopedFilters.owner);
      tokens.push({
        key: 'owner',
        label: `Owner: ${label}`,
        removeLabel: `Remove filter: Owner ${label}`,
      });
    }
    if (scopedFilters.status !== DEFAULT_SCOPED_FILTERS.status) {
      const label = optionLabel('status', scopedFilters.status);
      tokens.push({
        key: 'status',
        label: `Status: ${label}`,
        removeLabel: `Remove filter: Status ${label}`,
      });
    }
    if (scopedFilters.stale !== DEFAULT_SCOPED_FILTERS.stale) {
      const label = optionLabel('stale', scopedFilters.stale);
      tokens.push({
        key: 'stale',
        label,
        removeLabel: `Remove filter: Freshness ${label}`,
      });
    }
    if (scopedFilters.orphan !== DEFAULT_SCOPED_FILTERS.orphan) {
      const label = optionLabel('orphan', scopedFilters.orphan);
      tokens.push({
        key: 'orphan',
        label,
        removeLabel: `Remove filter: Connectivity ${label}`,
      });
    }
    if (scopedFilters.documentation !== DEFAULT_SCOPED_FILTERS.documentation) {
      const label = optionLabel('documentation', scopedFilters.documentation);
      tokens.push({
        key: 'documentation',
        label,
        removeLabel: `Remove filter: Docs ${label}`,
      });
    }
    if (scopedFilters.lifecycle !== DEFAULT_SCOPED_FILTERS.lifecycle) {
      const label = optionLabel('lifecycle', scopedFilters.lifecycle);
      tokens.push({
        key: 'lifecycle',
        label,
        removeLabel: `Remove filter: Lifecycle ${label}`,
      });
    }
    if (scopedFilters.updatedDate !== DEFAULT_SCOPED_FILTERS.updatedDate) {
      const label = optionLabel('updatedDate', scopedFilters.updatedDate);
      tokens.push({
        key: 'updatedDate',
        label,
        removeLabel: `Remove filter: Updated ${label}`,
      });
    }
    return tokens;
  }, [knowledgeFilterDescriptors, scopedFilters]);

  const originBackLink = useMemo(() => {
    if (!deepLink.originPath) return null;
    return {
      path: deepLink.originPath,
      label: deepLink.originLabel ?? 'origin surface',
    };
  }, [deepLink.originLabel, deepLink.originPath]);

  useEffect(() => {
    const nodeIds = new Set(nodes.map(node => node.id));
    setSelectedNodeIds(previous => {
      const next = previous.filter(nodeId => nodeIds.has(nodeId));
      return next.length === previous.length ? previous : next;
    });
    if (selectedNodeId && !nodeIds.has(selectedNodeId)) {
      setSelectedNodeId(null);
    }
  }, [nodes, selectedNodeId]);

  const setScopedFilter = useCallback(<K extends keyof ScopedFiltersState>(key: K, value: ScopedFiltersState[K]) => {
    setScopedFilters(previous => ({ ...previous, [key]: value }));
  }, []);

  const clearScopedFilters = useCallback(() => {
    setScopedFilters({ ...DEFAULT_SCOPED_FILTERS });
  }, []);

  const resetToProjectScope = useCallback(() => {
    if (!projectId) return;
    setScopedFilters({ ...DEFAULT_SCOPED_FILTERS });
    setSelectedNodeIds([]);
    navigate(`/knowledge/projects/${encodeURIComponent(projectId)}`);
  }, [navigate, projectId]);

  const resolveUnscopedNode = useCallback(async (
    node: NodeSummary,
    target: 'project' | 'global',
  ) => {
    const lifecycle = node.lifecycle === 'archived' ? 'archived' : 'active';
    setReviewBusyNodeId(node.id);
    try {
      if (target === 'project') {
        if (!projectId) {
          setActionNotice('Open a project-scoped view to assign unscoped records to a project.');
          return;
        }
        await api.updateBlueprintNode(node.id, {
          scope: {
            scope_class: 'project',
            project: {
              project_id: projectId,
              project_name: scopedProjectName ?? projectId,
            },
            secondary: {},
            is_shared: false,
            shared: null,
            lifecycle,
            override_scope: null,
          },
        } as unknown as Partial<BlueprintNode>);
        setActionNotice(`Assigned '${node.name}' to project scope.`);
      } else {
        await api.updateBlueprintNode(node.id, {
          scope: {
            scope_class: 'global',
            project: null,
            secondary: {},
            is_shared: false,
            shared: null,
            lifecycle,
            override_scope: null,
          },
        } as unknown as Partial<BlueprintNode>);
        setActionNotice(`Marked '${node.name}' as intentionally global.`);
      }
      await loadBlueprint();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setReviewBusyNodeId(previous => (previous === node.id ? null : previous));
    }
  }, [api, loadBlueprint, projectId, scopedProjectName]);

  const toggleSelectedNode = useCallback((nodeId: string, selected: boolean) => {
    setSelectedNodeIds(previous => {
      if (selected) {
        return previous.includes(nodeId) ? previous : [...previous, nodeId];
      }
      return previous.filter(id => id !== nodeId);
    });
  }, []);

  const toggleSelectAllVisible = useCallback((nodeIds: string[], selected: boolean) => {
    setSelectedNodeIds(previous => {
      const next = new Set(previous);
      for (const nodeId of nodeIds) {
        if (selected) next.add(nodeId);
        else next.delete(nodeId);
      }
      return Array.from(next);
    });
  }, []);

  const archiveSelected = useCallback(async () => {
    if (selectedNodeIds.length === 0) return;
    setActionBusy('archive');
    try {
      await Promise.all(selectedNodeIds.map(async nodeId => {
        const node = nodes.find(entry => entry.id === nodeId);
        if (!node || isArchivedNode(node)) return;
        await api.updateBlueprintNode(
          nodeId,
          { scope: { lifecycle: 'archived' } } as unknown as Partial<BlueprintNode>,
        );
      }));
      await loadBlueprint();
      setActionNotice(`Archived ${selectedNodeIds.length} record${selectedNodeIds.length === 1 ? '' : 's'} in current scope.`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setActionBusy(null);
    }
  }, [api, loadBlueprint, nodes, selectedNodeIds]);

  const restoreSelected = useCallback(async () => {
    if (selectedNodeIds.length === 0) return;
    const confirmed = window.confirm(
      `Restore archived state for ${selectedNodeIds.length} selected record${selectedNodeIds.length === 1 ? '' : 's'}?`,
    );
    if (!confirmed) return;
    setActionBusy('restore');
    try {
      await Promise.all(selectedNodeIds.map(async nodeId => {
        const node = nodes.find(entry => entry.id === nodeId);
        if (!node || !isArchivedNode(node)) return;
        await api.updateBlueprintNode(
          nodeId,
          { scope: { lifecycle: 'active' } } as unknown as Partial<BlueprintNode>,
        );
      }));
      await loadBlueprint();
      setActionNotice(`Restored ${selectedNodeIds.length} archived record${selectedNodeIds.length === 1 ? '' : 's'}.`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setActionBusy(null);
    }
  }, [api, loadBlueprint, nodes, selectedNodeIds]);

  const exportSingleRecord = useCallback(async () => {
    if (!exportTargetNodeId) return;
    setActionBusy('export');
    try {
      const record = await api.getBlueprintNode(exportTargetNodeId);
      const payload = {
        exported_at: new Date().toISOString(),
        scope: {
          project_id: projectId ?? null,
          project_name: isProjectScoped ? scopedProjectName : 'All Projects',
        },
        node: record,
      };
      const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      const fileStem = exportTargetNodeId.replace(/[^a-z0-9_-]+/gi, '-').toLowerCase();
      link.href = url;
      link.download = `knowledge-record-${fileStem || 'export'}.json`;
      document.body.appendChild(link);
      link.click();
      link.remove();
      URL.revokeObjectURL(url);
      await api.recordBlueprintExport({
        kind: 'single_record',
        nodeId: exportTargetNodeId,
        nodeCount: 1,
        edgeCount: 0,
        projectId,
        projectName: isProjectScoped ? scopedProjectName ?? projectId : 'All Projects',
        scopeSnapshot: {
          filters: scopedFilters,
          selected_node_id: exportTargetNodeId,
        },
      });
      if (isProjectScoped) {
        await loadBlueprint();
      }
      setActionNotice(`Exported ${exportTargetLabel ?? 'selected record'} as JSON.`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setActionBusy(null);
    }
  }, [
    api,
    exportTargetLabel,
    exportTargetNodeId,
    isProjectScoped,
    loadBlueprint,
    projectId,
    scopedFilters,
    scopedProjectName,
  ]);

  const exportScopedView = useCallback(async () => {
    setActionBusy('export');
    try {
      const payload = {
        exported_at: new Date().toISOString(),
        scope: {
          project_id: projectId ?? null,
          project_name: isProjectScoped ? scopedProjectName : 'All Projects',
        },
        filters: scopedFilters,
        counts: {
          nodes: sectionFilteredNodes.length,
          edges: sectionFilteredEdges.length,
        },
        nodes: sectionFilteredNodes,
        edges: sectionFilteredEdges,
      };
      const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      const fileStem = projectId ? `knowledge-${projectId}` : 'knowledge-all-projects';
      link.href = url;
      link.download = `${fileStem}-${Date.now()}.json`;
      document.body.appendChild(link);
      link.click();
      link.remove();
      URL.revokeObjectURL(url);
      await api.recordBlueprintExport({
        kind: 'scoped_view',
        nodeCount: sectionFilteredNodes.length,
        edgeCount: sectionFilteredEdges.length,
        projectId,
        projectName: isProjectScoped ? scopedProjectName ?? projectId : 'All Projects',
        scopeSnapshot: {
          filters: scopedFilters,
          section: projectSection,
        },
      });
      setActionNotice(`Exported ${sectionFilteredNodes.length} scoped record${sectionFilteredNodes.length === 1 ? '' : 's'} as JSON.`);
      if (isProjectScoped) {
        await loadBlueprint();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setActionBusy(null);
    }
  }, [
    isProjectScoped,
    loadBlueprint,
    projectId,
    projectSection,
    scopedFilters,
    scopedProjectName,
    sectionFilteredEdges,
    sectionFilteredNodes,
  ]);

  const branchSelection = useCallback(async () => {
    const sourceIds = selectedNodeIds.length > 0
      ? selectedNodeIds
      : sectionFilteredNodes.slice(0, MAX_BRANCH_ACTION_NODES).map(node => node.id);
    if (sourceIds.length === 0) return;

    setActionBusy('branch');
    try {
      let created = 0;
      for (const sourceId of sourceIds) {
        const sourceNode = await api.getBlueprintNode(sourceId);
        await api.createBlueprintNode(toBranchNode(sourceNode));
        created += 1;
      }
      await loadBlueprint();
      setActionNotice(
        selectedNodeIds.length > 0
          ? `Branched ${created} selected record${created === 1 ? '' : 's'}.`
          : `Branched ${created} scoped record${created === 1 ? '' : 's'} (max ${MAX_BRANCH_ACTION_NODES} per action).`,
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setActionBusy(null);
    }
  }, [api, loadBlueprint, sectionFilteredNodes, selectedNodeIds]);

  const handleCreateNode = useCallback(async (node: BlueprintNode) => {
    setActionBusy('create');
    try {
      await api.createBlueprintNode(node);
      await loadBlueprint();
      const display = nodeDisplayName(node);
      setActionNotice(`Created ${display} in current scoped context.`);
      setCreateModalOpen(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setActionBusy(null);
    }
  }, [api, loadBlueprint]);

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
        ownerCounts: new Map(),
        teamCounts: new Map(),
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

        if (isLocal) {
          const owner = extractOwnerLabel(node);
          if (owner) upsertCount(bucket.ownerCounts, owner);
          const team = extractTeamLabel(node);
          if (team) upsertCount(bucket.teamCounts, team);
        }

        for (const tag of node.tags) {
          const normalizedTag = tag.trim();
          if (!normalizedTag || !isProjectSignalTag(normalizedTag)) continue;
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
        pickTopLabel(bucket.ownerCounts) ?? '',
        pickTopLabel(bucket.teamCounts) ?? '',
        ...topTags,
      ]
        .join(' ')
        .toLowerCase();

      summaries.push({
        id,
        name: resolvedName,
        description,
        ownerLabel: pickTopLabel(bucket.ownerCounts),
        teamLabel: pickTopLabel(bucket.teamCounts),
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

  const recentNodeActivity = useMemo(() => {
    return [...filteredNodes]
      .sort((left, right) => parseIsoTimeMs(right.updated_at) - parseIsoTimeMs(left.updated_at))
      .slice(0, 12);
  }, [filteredNodes]);
  const projectMutationEvents = useMemo(
    () => projectEvents.filter(entry => entry.kind === 'mutation'),
    [projectEvents],
  );
  const durableExportEvents = useMemo(
    () => projectEvents.filter(entry => entry.kind === 'export'),
    [projectEvents],
  );

  const lineageEntries = useMemo(() => {
    return nodes
      .map(node => {
        const lineageTag = node.tags.find(tag => tag.trim().toLowerCase().startsWith('lineage:branch-of:'));
        if (!lineageTag) return null;
        const source = lineageTag.slice('lineage:branch-of:'.length).trim();
        return source ? { node, source } : null;
      })
      .filter((entry): entry is { node: NodeSummary; source: string } => entry !== null)
      .slice(0, 20);
  }, [nodes]);

  const reviewQueues = useMemo(() => ([
    {
      key: 'unscoped',
      label: 'Needs scope',
      count: missingScopeCount,
      onOpen: () => {
        setScopedFilter('scopeClass', 'unscoped');
        setProjectSection('quality');
      },
    },
    {
      key: 'stale',
      label: 'Stale records',
      count: staleCount,
      onOpen: () => {
        setScopedFilter('stale', 'stale');
        setProjectSection('quality');
      },
    },
    {
      key: 'orphan',
      label: 'Orphan records',
      count: orphanCount,
      onOpen: () => {
        setScopedFilter('orphan', 'orphan');
        setProjectSection('quality');
      },
    },
    {
      key: 'archived',
      label: 'Archived pending review',
      count: archivedCount,
      onOpen: () => {
        setScopedFilter('lifecycle', 'archived');
        setProjectSection('quality');
      },
    },
  ]), [archivedCount, missingScopeCount, orphanCount, setScopedFilter, staleCount]);
  const unscopedReviewNodes = useMemo(
    () => nodes.filter(node => (node.scope_class ?? 'unscoped') === 'unscoped').slice(0, 20),
    [nodes],
  );

  const initialCreateScope = useMemo(() => {
    const contextualFeature = scopedFilters.feature !== 'all' ? scopedFilters.feature : (deepLink.filters.feature ?? '');
    const contextualWidget = scopedFilters.widget !== 'all' ? scopedFilters.widget : (deepLink.filters.widget ?? '');
    const contextualArtifact = scopedFilters.artifact !== 'all' ? scopedFilters.artifact : (deepLink.filters.artifact ?? '');
    const contextualComponent = scopedFilters.component !== 'all' ? scopedFilters.component : (deepLink.filters.component ?? '');
    const hasContextualSecondary = Boolean(
      contextualFeature || contextualWidget || contextualArtifact || contextualComponent,
    );
    if (isProjectScoped && projectId) {
      return {
        scopeClass: (hasContextualSecondary ? 'project_contextual' : 'project') as ScopeClass,
        projectId,
        projectName: scopedProjectName ?? projectId,
        feature: contextualFeature,
        widget: contextualWidget,
        artifact: contextualArtifact,
        component: contextualComponent,
      };
    }
    return undefined;
  }, [
    deepLink.filters.artifact,
    deepLink.filters.component,
    deepLink.filters.feature,
    deepLink.filters.widget,
    isProjectScoped,
    projectId,
    scopedFilters.artifact,
    scopedFilters.component,
    scopedFilters.feature,
    scopedFilters.widget,
    scopedProjectName,
  ]);

  const visibleNodesForView = isProjectScoped ? sectionFilteredNodes : filteredNodes;
  const visibleEdgesForView = isProjectScoped ? sectionFilteredEdges : filteredEdges;

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
                  ? `Project-scoped view for ${scopedProjectName}. Shared across projects records linked to this project are included.`
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
                {originBackLink && (
                  <Link to={originBackLink.path} style={{ fontSize: 'var(--text-xs)' }}>
                    Back to {originBackLink.label}
                  </Link>
                )}
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
                <span className="knowledge-stat-label">Needs Scope</span>
              </div>
            </div>
          )}
        </div>

        {!isProjectLanding && (
          <div className="knowledge-scope-shell">
            <div className="knowledge-scope-header">
              <div className="knowledge-scope-title-row">
                <div className="knowledge-scope-context">
                  <span className="knowledge-scope-pill">
                    {isProjectScoped ? `Project: ${scopedProjectName}` : 'Global: All Projects'}
                  </span>
                  {isProjectScoped && <span className="knowledge-scope-subtle">{projectId}</span>}
                  <span className="knowledge-scope-subtle">{visibleNodesForView.length} visible</span>
                  <span className="knowledge-scope-subtle">{selectedNodeIds.length} selected</span>
                </div>
                <div className="knowledge-scope-actions-inline">
                  <button
                    type="button"
                    className="scope-action-btn"
                    onClick={() => setCreateModalOpen(true)}
                    disabled={actionBusy !== null}
                  >
                    Create knowledge
                  </button>
                  <button type="button" className="scope-action-btn" onClick={clearScopedFilters}>
                    Clear all
                  </button>
                  {isProjectScoped && (
                    <button type="button" className="scope-action-btn" onClick={resetToProjectScope}>
                      Reset to project scope
                    </button>
                  )}
                  {!isGlobalView ? (
                    <Link to="/knowledge/all" className="scope-action-link">Open global view</Link>
                  ) : (
                    <span className="knowledge-scope-subtle">Global view active</span>
                  )}
                </div>
              </div>

              <KnowledgeFilterBar descriptors={knowledgeFilterDescriptors} />

              <div className="knowledge-active-filter-row">
                <span className="knowledge-active-filter-label">Active filters</span>
                {activeFilterTokens.length === 0 && (
                  <span className="knowledge-active-filter-empty">none</span>
                )}
                {activeFilterTokens.map(token => (
                  <button
                    key={`${token.key}:${token.label}`}
                    type="button"
                    className="knowledge-active-filter-chip"
                    aria-label={token.removeLabel}
                    onClick={() => setScopedFilter(token.key, DEFAULT_SCOPED_FILTERS[token.key])}
                  >
                    <span>{token.label}</span>
                    <span className="knowledge-active-filter-chip-dismiss" aria-hidden="true">×</span>
                  </button>
                ))}
              </div>

              {actionNotice && (
                <div className="knowledge-scope-notice" role="status">
                  {actionNotice}
                </div>
              )}
            </div>

            <div className="knowledge-action-row">
              <button
                type="button"
                className="scope-action-btn"
                onClick={() => void archiveSelected()}
                disabled={selectedNodeIds.length === 0 || actionBusy !== null || selectedNodes.every(isArchivedNode)}
              >
                Archive selected knowledge
              </button>
              <button
                type="button"
                className="scope-action-btn"
                onClick={() => void restoreSelected()}
                disabled={selectedArchivedCount === 0 || actionBusy !== null}
              >
                Restore archived knowledge
              </button>
              <button
                type="button"
                className="scope-action-btn"
                onClick={() => void exportSingleRecord()}
                disabled={!exportTargetNodeId || actionBusy !== null}
              >
                Export selected record
              </button>
              <button
                type="button"
                className="scope-action-btn"
                onClick={() => void exportScopedView()}
                disabled={visibleNodesForView.length === 0 || actionBusy !== null}
              >
                Export current scoped view
              </button>
              <button
                type="button"
                className="scope-action-btn"
                onClick={() => void branchSelection()}
                disabled={(selectedNodeIds.length === 0 && visibleNodesForView.length === 0) || actionBusy !== null}
              >
                Duplicate / branch scoped subset
              </button>
            </div>
          </div>
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

          {!loading && !error && blueprint && isProjectScoped && (
            <>
              <div className="knowledge-section-tabs">
                {PROJECT_SECTION_TABS.map(section => (
                  <button
                    key={section.value}
                    type="button"
                    className={`knowledge-section-tab${projectSection === section.value ? ' active' : ''}`}
                    onClick={() => setProjectSection(section.value)}
                  >
                    {section.label}
                  </button>
                ))}
              </div>

              {projectSection === 'overview' && (
                <div className="knowledge-section-panel">
                  <div className="knowledge-overview-grid">
                    <div className="knowledge-overview-card">
                      <span className="knowledge-overview-label">Inventory</span>
                      <span className="knowledge-overview-value">
                        {nodes.filter(node => node.node_type === 'component' || node.node_type === 'technology').length}
                      </span>
                    </div>
                    <div className="knowledge-overview-card">
                      <span className="knowledge-overview-label">Architecture</span>
                      <span className="knowledge-overview-value">
                        {nodes.filter(node => node.node_type === 'decision' || node.node_type === 'constraint' || node.node_type === 'pattern').length}
                      </span>
                    </div>
                    <div className="knowledge-overview-card">
                      <span className="knowledge-overview-label">{labelNodeType('quality_requirement', 'short')}</span>
                      <span className="knowledge-overview-value">
                        {nodes.filter(node => node.node_type === 'quality_requirement').length}
                      </span>
                    </div>
                    <div className="knowledge-overview-card">
                      <span className="knowledge-overview-label">Active</span>
                      <span className="knowledge-overview-value">{lifecycleCounts.active}</span>
                    </div>
                  </div>
                </div>
              )}

              {projectSection === 'quality' && (
                <div className="knowledge-section-panel">
                  <div className="knowledge-overview-grid">
                    <div className="knowledge-overview-card">
                      <span className="knowledge-overview-label">Stale</span>
                      <span className="knowledge-overview-value">{staleCount}</span>
                    </div>
                    <div className="knowledge-overview-card">
                      <span className="knowledge-overview-label">Orphaned</span>
                      <span className="knowledge-overview-value">{orphanCount}</span>
                    </div>
                    <div className="knowledge-overview-card">
                      <span className="knowledge-overview-label">Needs Scope</span>
                      <span className="knowledge-overview-value">{missingScopeCount}</span>
                    </div>
                    <div className="knowledge-overview-card">
                      <span className="knowledge-overview-label">Missing Docs</span>
                      <span className="knowledge-overview-value">{missingDocsCount}</span>
                    </div>
                    <div className="knowledge-overview-card">
                      <span className="knowledge-overview-label">Archived</span>
                      <span className="knowledge-overview-value">{archivedCount}</span>
                    </div>
                    <div className="knowledge-overview-card">
                      <span className="knowledge-overview-label">Changed (7d)</span>
                      <span className="knowledge-overview-value">{recentlyChangedCount}</span>
                    </div>
                  </div>

                  <p className="knowledge-section-muted" style={{ marginTop: 'var(--space-3)' }}>
                    Shared guidance overrides are represented as first-class scope relations on
                    project-local records.
                  </p>

                  <div className="knowledge-review-queue">
                    {reviewQueues.map(queue => (
                      <button
                        key={queue.key}
                        type="button"
                        className="knowledge-review-queue-item"
                        onClick={queue.onOpen}
                      >
                        <span>{queue.label}</span>
                        <span>{queue.count}</span>
                      </button>
                    ))}
                  </div>

                  {unscopedReviewNodes.length > 0 && (
                    <div style={{ marginTop: 'var(--space-4)' }}>
                      <h3 className="knowledge-section-subtitle">Needs Scope Review Workflow</h3>
                      <p className="knowledge-section-muted">
                        Resolve ambiguous records by assigning project scope or marking intentionally global.
                      </p>
                      <div className="knowledge-review-queue" style={{ marginTop: 'var(--space-2)' }}>
                        {unscopedReviewNodes.map(node => {
                          const busy = reviewBusyNodeId === node.id;
                          return (
                            <div key={`unscoped-${node.id}`} className="knowledge-review-queue-item" style={{ alignItems: 'stretch', gap: 'var(--space-2)' }}>
                              <span style={{ fontWeight: 600 }}>{node.name}</span>
                              <span style={{ color: 'var(--color-text-faint)', fontSize: 'var(--text-xs)' }}>{node.node_type}</span>
                              <div style={{ display: 'flex', gap: 'var(--space-2)' }}>
                                <button
                                  type="button"
                                  className="scope-action-btn"
                                  disabled={busy || !isProjectScoped}
                                  onClick={() => void resolveUnscopedNode(node, 'project')}
                                  title={isProjectScoped ? 'Assign to current project scope' : 'Open a project view to assign project scope'}
                                >
                                  {busy ? 'Saving…' : 'Assign to project'}
                                </button>
                                <button
                                  type="button"
                                  className="scope-action-btn"
                                  disabled={busy}
                                  onClick={() => void resolveUnscopedNode(node, 'global')}
                                >
                                  Mark global
                                </button>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  )}
                </div>
              )}

              {projectSection === 'activity' && (
                <div className="knowledge-section-panel">
                  <div className="knowledge-activity-columns">
                    <div>
                      <h3 className="knowledge-section-subtitle">Project Event History</h3>
                      {projectMutationEvents.length === 0 && (
                        <p className="knowledge-section-muted">No durable project activity captured yet.</p>
                      )}
                      {projectMutationEvents.map(entry => (
                        <div key={entry.id} className="knowledge-activity-item">
                          <span>{entry.summary}</span>
                          <span>durable</span>
                          <span>{new Date(entry.timestamp).toLocaleString()}</span>
                          <p>{entry.details}</p>
                        </div>
                      ))}

                      <h3 className="knowledge-section-subtitle">Durable Export History</h3>
                      {durableExportEvents.length === 0 && (
                        <p className="knowledge-section-muted">No durable export activity recorded yet.</p>
                      )}
                      {durableExportEvents.map(entry => (
                        <div key={entry.id} className="knowledge-activity-item">
                          <span>{entry.summary}</span>
                          <span>durable</span>
                          <span>{new Date(entry.timestamp).toLocaleString()}</span>
                          <p>{entry.details}</p>
                        </div>
                      ))}

                      <h3 className="knowledge-section-subtitle">Review Queue</h3>
                      {reviewQueues.every(queue => queue.count === 0) && (
                        <p className="knowledge-section-muted">No queued records for review.</p>
                      )}
                      <div className="knowledge-review-queue">
                        {reviewQueues.map(queue => (
                          <button
                            key={`activity-${queue.key}`}
                            type="button"
                            className="knowledge-review-queue-item"
                            onClick={queue.onOpen}
                          >
                            <span>{queue.label}</span>
                            <span>{queue.count}</span>
                          </button>
                        ))}
                      </div>
                    </div>
                    <div>
                      <h3 className="knowledge-section-subtitle">Recent Node Changes</h3>
                      {recentNodeActivity.length === 0 && (
                        <p className="knowledge-section-muted">No recent node updates in this scope.</p>
                      )}
                      {recentNodeActivity.map(node => (
                        <div key={node.id} className="knowledge-activity-item">
                          <span>{node.name}</span>
                          <span>{node.node_type}</span>
                          <span>{new Date(node.updated_at).toLocaleString()}</span>
                        </div>
                      ))}

                      <h3 className="knowledge-section-subtitle">Branch Lineage</h3>
                      {lineageEntries.length === 0 && (
                        <p className="knowledge-section-muted">No branched records with lineage tags yet.</p>
                      )}
                      {lineageEntries.map(entry => (
                        <div key={entry.node.id} className="knowledge-activity-item">
                          <span>{entry.node.name}</span>
                          <span>branch</span>
                          <span>{entry.source}</span>
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              )}
            </>
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
                        {project.ownerLabel && (
                          <span className="project-card-meta-item">Owner: {project.ownerLabel}</span>
                        )}
                        {project.teamLabel && (
                          <span className="project-card-meta-item">Team: {project.teamLabel}</span>
                        )}
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

          {!loading && !error && blueprint && !isProjectLanding && (!isProjectScoped || projectSection !== 'activity') && (
            <NodeListPanel
              nodes={visibleNodesForView}
              edges={visibleEdgesForView}
              nodeType={null}
              onSelectNode={handleSelectNode}
              selectedNodeIds={selectedNodeIds}
              onToggleSelectNode={toggleSelectedNode}
              onToggleSelectAllVisible={toggleSelectAllVisible}
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

        {!isProjectLanding && (
          <CreateNodeModal
            isOpen={createModalOpen}
            onClose={() => setCreateModalOpen(false)}
            onCreate={handleCreateNode}
            initialScope={initialCreateScope}
            requireExplicitScopeSelection={!isProjectScoped}
          />
        )}
      </div>
    </Layout>
  );
}
