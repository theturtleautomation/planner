export interface KnowledgeContextFilters {
  feature?: string;
  widget?: string;
  artifact?: string;
  component?: string;
}

export interface BuildKnowledgeDeepLinkArgs extends KnowledgeContextFilters {
  projectId: string;
  originPath?: string;
  originLabel?: string;
}

export interface ParsedKnowledgeDeepLink {
  projectId?: string;
  filters: KnowledgeContextFilters;
  hasContextFilters: boolean;
  originPath: string | null;
  originLabel: string | null;
}

function normalizeValue(value: string | null | undefined): string | undefined {
  if (typeof value !== 'string') return undefined;
  const normalized = value.trim();
  return normalized.length > 0 ? normalized : undefined;
}

function sanitizeOriginPath(rawPath: string | null | undefined): string | null {
  const candidate = normalizeValue(rawPath);
  if (!candidate) return null;
  if (!candidate.startsWith('/')) return null;
  if (candidate.startsWith('//')) return null;
  if (candidate.includes('://')) return null;
  return candidate;
}

function sanitizeOriginLabel(rawLabel: string | null | undefined): string | null {
  const candidate = normalizeValue(rawLabel);
  if (!candidate) return null;
  return candidate.slice(0, 60);
}

export function buildKnowledgeDeepLink(args: BuildKnowledgeDeepLinkArgs): string {
  const projectId = args.projectId.trim();
  const params = new URLSearchParams();
  params.set('project', projectId);

  const feature = normalizeValue(args.feature);
  const widget = normalizeValue(args.widget);
  const artifact = normalizeValue(args.artifact);
  const component = normalizeValue(args.component);
  if (feature) params.set('feature', feature);
  if (widget) params.set('widget', widget);
  if (artifact) params.set('artifact', artifact);
  if (component) params.set('component', component);

  const originPath = sanitizeOriginPath(args.originPath);
  const originLabel = sanitizeOriginLabel(args.originLabel);
  if (originPath) params.set('from', originPath);
  if (originLabel) params.set('from_label', originLabel);

  const query = params.toString();
  const path = `/knowledge/projects/${encodeURIComponent(projectId)}`;
  return query ? `${path}?${query}` : path;
}

export function parseKnowledgeDeepLink(search: string): ParsedKnowledgeDeepLink {
  const params = new URLSearchParams(search);
  const projectId = normalizeValue(params.get('project'));
  const filters: KnowledgeContextFilters = {
    feature: normalizeValue(params.get('feature')),
    widget: normalizeValue(params.get('widget')),
    artifact: normalizeValue(params.get('artifact')),
    component: normalizeValue(params.get('component')),
  };

  return {
    projectId,
    filters,
    hasContextFilters: Boolean(filters.feature || filters.widget || filters.artifact || filters.component),
    originPath: sanitizeOriginPath(params.get('from')),
    originLabel: sanitizeOriginLabel(params.get('from_label')),
  };
}
