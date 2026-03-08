import { useState, useCallback, useEffect } from 'react';
import type {
  NodeType,
  BlueprintNode,
  DecisionStatus,
  TechnologyCategory,
  AdoptionRing,
  ComponentType,
  ComponentStatus,
  ConstraintType,
  QualityAttribute,
  QualityPriority,
  ScopeClass,
  NodeScope,
} from '../types/blueprint.ts';
import { labelComponentType, labelScopeClass, labelScopeField, labelSecondaryScopeField } from '../lib/taxonomy.ts';

// ─── Types ────────────────────────────────────────────────────────────────────

interface CreateNodeModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (node: BlueprintNode) => Promise<void>;
  initialScope?: {
    scopeClass?: ScopeClass;
    projectId?: string;
    projectName?: string;
    feature?: string;
    widget?: string;
    artifact?: string;
    component?: string;
    isShared?: boolean;
    linkedProjectIds?: string[];
    inheritToLinkedProjects?: boolean;
  };
  requireExplicitScopeSelection?: boolean;
}

const NODE_TYPE_OPTIONS: { value: NodeType; label: string }[] = [
  { value: 'decision', label: 'Decision' },
  { value: 'technology', label: 'Technology' },
  { value: 'component', label: 'Component' },
  { value: 'constraint', label: 'Constraint' },
  { value: 'pattern', label: 'Pattern' },
  { value: 'quality_requirement', label: 'Quality Scenario' },
];

// ─── Helpers ──────────────────────────────────────────────────────────────────

function generateId(type: NodeType, name: string): string {
  const prefix = type === 'quality_requirement' ? 'qr' : type.slice(0, 3);
  const slug = name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 30);
  const uuid8 = crypto.randomUUID().replace(/-/g, '').slice(0, 8);
  return `${prefix}-${slug}-${uuid8}`;
}

function nowISO(): string {
  return new Date().toISOString();
}

function buildScope(params: {
  scopeClass: ScopeClass;
  projectId: string;
  projectName: string;
  feature: string;
  widget: string;
  artifact: string;
  component: string;
  isShared: boolean;
  linkedProjectIds: string;
  inheritToLinkedProjects: boolean;
}): NodeScope {
  const parsedLinkedProjectIds = params.linkedProjectIds
    .split(',')
    .map(v => v.trim())
    .filter(Boolean);

  return {
    scope_class: params.scopeClass,
    project: params.projectId.trim()
      ? {
          project_id: params.projectId.trim(),
          project_name: params.projectName.trim() || undefined,
        }
      : undefined,
    secondary: {
      feature: params.feature.trim() || undefined,
      widget: params.widget.trim() || undefined,
      artifact: params.artifact.trim() || undefined,
      component: params.component.trim() || undefined,
    },
    is_shared: params.isShared,
    shared: params.isShared
      ? {
          linked_project_ids: parsedLinkedProjectIds,
          inherit_to_linked_projects: params.inheritToLinkedProjects,
        }
      : undefined,
    lifecycle: 'active',
    override_scope: undefined,
  };
}

// ─── Component ────────────────────────────────────────────────────────────────

export default function CreateNodeModal({
  isOpen,
  onClose,
  onCreate,
  initialScope,
  requireExplicitScopeSelection = false,
}: CreateNodeModalProps) {
  const [nodeType, setNodeType] = useState<NodeType>('decision');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Common fields
  const [name, setName] = useState('');
  const [tags, setTags] = useState('');
  const [scopeClass, setScopeClass] = useState<ScopeClass>('unscoped');
  const [projectId, setProjectId] = useState('');
  const [projectName, setProjectName] = useState('');
  const [scopeFeature, setScopeFeature] = useState('');
  const [scopeWidget, setScopeWidget] = useState('');
  const [scopeArtifact, setScopeArtifact] = useState('');
  const [scopeComponent, setScopeComponent] = useState('');
  const [isShared, setIsShared] = useState(false);
  const [linkedProjectIds, setLinkedProjectIds] = useState('');
  const [inheritToLinkedProjects, setInheritToLinkedProjects] = useState(true);
  const [scopeSelectionAcknowledged, setScopeSelectionAcknowledged] = useState(false);

  // Decision fields
  const [context, setContext] = useState('');
  const [decisionStatus, setDecisionStatus] = useState<DecisionStatus>('proposed');

  // Technology fields
  const [version, setVersion] = useState('');
  const [techCategory, setTechCategory] = useState<TechnologyCategory>('library');
  const [ring, setRing] = useState<AdoptionRing>('assess');
  const [rationale, setRationale] = useState('');
  const [license, setLicense] = useState('');

  // Component fields
  const [componentType, setComponentType] = useState<ComponentType>('module');
  const [description, setDescription] = useState('');
  const [componentStatus, setComponentStatus] = useState<ComponentStatus>('planned');

  // Constraint fields
  const [constraintType, setConstraintType] = useState<ConstraintType>('technical');
  const [constraintSource, setConstraintSource] = useState('');

  // Pattern fields
  const [patternRationale, setPatternRationale] = useState('');

  // Quality requirement fields
  const [attribute, setAttribute] = useState<QualityAttribute>('performance');
  const [scenario, setScenario] = useState('');
  const [priority, setPriority] = useState<QualityPriority>('medium');

  const resetForm = useCallback(() => {
    setNodeType('decision');
    setName('');
    setTags('');
    setScopeClass('unscoped');
    setProjectId('');
    setProjectName('');
    setScopeFeature('');
    setScopeWidget('');
    setScopeArtifact('');
    setScopeComponent('');
    setIsShared(false);
    setLinkedProjectIds('');
    setInheritToLinkedProjects(true);
    setContext('');
    setDecisionStatus('proposed');
    setVersion('');
    setTechCategory('library');
    setRing('assess');
    setRationale('');
    setLicense('');
    setComponentType('module');
    setDescription('');
    setComponentStatus('planned');
    setConstraintType('technical');
    setConstraintSource('');
    setPatternRationale('');
    setAttribute('performance');
    setScenario('');
    setPriority('medium');
    setError(null);
    setSaving(false);
    setScopeSelectionAcknowledged(false);
  }, []);

  useEffect(() => {
    if (!isOpen) return;
    setError(null);
    setSaving(false);

    const nextScopeClass = initialScope?.scopeClass ?? 'unscoped';
    setScopeClass(nextScopeClass);
    setProjectId(initialScope?.projectId ?? '');
    setProjectName(initialScope?.projectName ?? '');
    setScopeFeature(initialScope?.feature ?? '');
    setScopeWidget(initialScope?.widget ?? '');
    setScopeArtifact(initialScope?.artifact ?? '');
    setScopeComponent(initialScope?.component ?? '');
    setIsShared(initialScope?.isShared ?? false);
    setLinkedProjectIds((initialScope?.linkedProjectIds ?? []).join(', '));
    setInheritToLinkedProjects(initialScope?.inheritToLinkedProjects ?? true);

    const hasInitialScope =
      nextScopeClass !== 'unscoped'
      || Boolean(initialScope?.projectId?.trim())
      || Boolean(initialScope?.projectName?.trim())
      || Boolean(initialScope?.feature?.trim())
      || Boolean(initialScope?.widget?.trim())
      || Boolean(initialScope?.artifact?.trim())
      || Boolean(initialScope?.component?.trim())
      || Boolean(initialScope?.isShared);
    setScopeSelectionAcknowledged(hasInitialScope);
  }, [initialScope, isOpen]);

  const handleClose = useCallback(() => {
    resetForm();
    onClose();
  }, [onClose, resetForm]);

  const handleSubmit = useCallback(async () => {
    if (!name.trim()) {
      setError('Name is required');
      return;
    }

    setSaving(true);
    setError(null);

    const now = nowISO();
    const parsedTags = tags
      .split(',')
      .map(t => t.trim())
      .filter(Boolean);
    const parsedLinkedProjects = linkedProjectIds
      .split(',')
      .map(v => v.trim())
      .filter(Boolean);

    if ((scopeClass === 'project' || scopeClass === 'project_contextual') && !projectId.trim()) {
      setError('Project ID is required for project-scoped records');
      setSaving(false);
      return;
    }

    if (requireExplicitScopeSelection && !isShared && scopeClass === 'unscoped' && !scopeSelectionAcknowledged) {
      setError('Select scope explicitly for global creation, or mark this record as shared.');
      setSaving(false);
      return;
    }

    if (isShared && parsedLinkedProjects.length === 0) {
      setError('Shared records require at least one linked project ID');
      setSaving(false);
      return;
    }

    const scope = buildScope({
      scopeClass,
      projectId,
      projectName,
      feature: scopeFeature,
      widget: scopeWidget,
      artifact: scopeArtifact,
      component: scopeComponent,
      isShared,
      linkedProjectIds,
      inheritToLinkedProjects,
    });

    let node: BlueprintNode;

    try {
      switch (nodeType) {
        case 'decision':
          node = {
            node_type: 'decision',
            id: generateId('decision', name),
            title: name.trim(),
            status: decisionStatus,
            context: context.trim() || 'No context provided',
            options: [],
            consequences: [],
            assumptions: [],
            tags: parsedTags,
            scope,
            created_at: now,
            updated_at: now,
          };
          break;

        case 'technology':
          node = {
            node_type: 'technology',
            id: generateId('technology', name),
            name: name.trim(),
            version: version.trim() || undefined,
            category: techCategory,
            ring,
            rationale: rationale.trim() || 'No rationale provided',
            license: license.trim() || undefined,
            tags: parsedTags,
            scope,
            created_at: now,
            updated_at: now,
          };
          break;

        case 'component':
          {
            const nodeId = generateId('component', name);
            const trimmedName = name.trim();
            node = {
              node_type: 'component',
              id: nodeId,
              name: trimmedName,
              component_type: componentType,
              naming: {
                origin_key: `manual:${nodeId}`,
                source: 'manual',
                strategy: 'manual_create',
                generated_name: trimmedName,
                naming_version: 1,
                last_generated_at: now,
              },
              description: description.trim() || 'No description provided',
              provides: [],
              consumes: [],
              status: componentStatus,
              tags: parsedTags,
              scope,
              created_at: now,
              updated_at: now,
            };
          }
          break;

        case 'constraint':
          node = {
            node_type: 'constraint',
            id: generateId('constraint', name),
            title: name.trim(),
            constraint_type: constraintType,
            description: description.trim() || 'No description provided',
            source: constraintSource.trim() || 'Unknown',
            tags: parsedTags,
            scope,
            created_at: now,
            updated_at: now,
          };
          break;

        case 'pattern':
          node = {
            node_type: 'pattern',
            id: generateId('pattern', name),
            name: name.trim(),
            description: description.trim() || 'No description provided',
            rationale: patternRationale.trim() || 'No rationale provided',
            tags: parsedTags,
            scope,
            created_at: now,
            updated_at: now,
          };
          break;

        case 'quality_requirement':
          node = {
            node_type: 'quality_requirement',
            id: generateId('quality_requirement', name),
            attribute,
            scenario: scenario.trim() || name.trim(),
            priority,
            tags: parsedTags,
            scope,
            created_at: now,
            updated_at: now,
          };
          break;
      }

      await onCreate(node);
      handleClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }, [
    nodeType, name, tags, scopeClass, projectId, projectName,
    scopeFeature, scopeWidget, scopeArtifact, scopeComponent,
    isShared, linkedProjectIds, inheritToLinkedProjects,
    context, decisionStatus,
    version, techCategory, ring, rationale, license,
    componentType, description, componentStatus,
    constraintType, constraintSource,
    patternRationale,
    attribute, scenario, priority,
    requireExplicitScopeSelection, scopeSelectionAcknowledged,
    onCreate, handleClose,
  ]);

  if (!isOpen) return null;

  return (
    <div className="modal-backdrop" onClick={handleClose}>
      <div
        className="modal"
        onClick={e => e.stopPropagation()}
        style={{ maxWidth: '520px', maxHeight: '80vh', overflow: 'auto' }}
      >
        <div className="modal-header">
          <div className="modal-title">Create Node</div>
          <button className="modal-close" onClick={handleClose}>&times;</button>
        </div>

        <div className="modal-body" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-3)' }}>
          {/* Node type selector */}
          <label className="field-label">
            Type
            <select
              value={nodeType}
              onChange={e => setNodeType(e.target.value as NodeType)}
              className="field-input"
            >
              {NODE_TYPE_OPTIONS.map(o => (
                <option key={o.value} value={o.value}>{o.label}</option>
              ))}
            </select>
          </label>

          {/* Name / Title (all types) */}
          <label className="field-label">
            {nodeType === 'decision' || nodeType === 'constraint' ? 'Title' : 'Name'}
            <input
              className="field-input"
              value={name}
              onChange={e => setName(e.target.value)}
              placeholder={`Enter ${nodeType} name…`}
              autoFocus
            />
          </label>

          <div style={{
            padding: 'var(--space-3)',
            borderRadius: 'var(--radius-md)',
            border: '1px solid var(--color-border)',
            background: 'var(--color-surface-offset)',
            display: 'grid',
            gap: 'var(--space-3)',
          }}>
            <label className="field-label">
              {labelScopeField('scope_class')}
              <select
                value={scopeClass}
                onChange={e => {
                  setScopeClass(e.target.value as ScopeClass);
                  setScopeSelectionAcknowledged(true);
                }}
                className="field-input"
              >
                <option value="unscoped">{labelScopeClass('unscoped')}</option>
                <option value="global">{labelScopeClass('global')}</option>
                <option value="project">{labelScopeClass('project')}</option>
                <option value="project_contextual">{labelScopeClass('project_contextual')}</option>
              </select>
            </label>

            {(scopeClass === 'project' || scopeClass === 'project_contextual') && (
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 'var(--space-3)' }}>
                <label className="field-label">
                  {labelScopeField('project_id')}
                  <input
                    className="field-input"
                    value={projectId}
                    onChange={e => setProjectId(e.target.value)}
                    placeholder="proj-task-tracker"
                  />
                </label>
                <label className="field-label">
                  {labelScopeField('project_name')}
                  <input
                    className="field-input"
                    value={projectName}
                    onChange={e => setProjectName(e.target.value)}
                    placeholder="Task Tracker"
                  />
                </label>
              </div>
            )}

            {scopeClass === 'project_contextual' && (
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 'var(--space-3)' }}>
                <label className="field-label">
                  {labelSecondaryScopeField('feature')}
                  <input className="field-input" value={scopeFeature} onChange={e => setScopeFeature(e.target.value)} />
                </label>
                <label className="field-label">
                  {labelSecondaryScopeField('widget')}
                  <input className="field-input" value={scopeWidget} onChange={e => setScopeWidget(e.target.value)} />
                </label>
                <label className="field-label">
                  {labelSecondaryScopeField('artifact')}
                  <input className="field-input" value={scopeArtifact} onChange={e => setScopeArtifact(e.target.value)} />
                </label>
                <label className="field-label">
                  {labelSecondaryScopeField('component')}
                  <input className="field-input" value={scopeComponent} onChange={e => setScopeComponent(e.target.value)} />
                </label>
              </div>
            )}

            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
              <input
                type="checkbox"
                checked={isShared}
                onChange={e => {
                  setIsShared(e.target.checked);
                  setScopeSelectionAcknowledged(true);
                }}
              />
              {labelScopeField('is_shared')}
            </label>

            {requireExplicitScopeSelection && scopeClass === 'unscoped' && !scopeSelectionAcknowledged && (
              <div style={{ fontSize: '0.625rem', color: 'var(--color-warning)' }}>
                Global creation requires explicit scope selection, unless this record is marked shared.
              </div>
            )}

            {isShared && (
              <>
                <label className="field-label">
                  {labelScopeField('linked_project_ids')}
                  <input
                    className="field-input"
                    value={linkedProjectIds}
                    onChange={e => setLinkedProjectIds(e.target.value)}
                    placeholder="proj-task-tracker, proj-shared-ui"
                  />
                </label>
                <label style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
                  <input
                    type="checkbox"
                    checked={inheritToLinkedProjects}
                    onChange={e => setInheritToLinkedProjects(e.target.checked)}
                  />
                  {labelScopeField('inherit_to_linked_projects')}
                </label>
              </>
            )}
          </div>

          {/* ── Decision-specific ── */}
          {nodeType === 'decision' && (
            <>
              <label className="field-label">
                Status
                <select
                  value={decisionStatus}
                  onChange={e => setDecisionStatus(e.target.value as DecisionStatus)}
                  className="field-input"
                >
                  <option value="proposed">Proposed</option>
                  <option value="accepted">Accepted</option>
                  <option value="deprecated">Deprecated</option>
                  <option value="superseded">Superseded</option>
                </select>
              </label>
              <label className="field-label">
                Context
                <textarea
                  className="field-input"
                  value={context}
                  onChange={e => setContext(e.target.value)}
                  placeholder="Why was this decision needed?"
                  rows={3}
                />
              </label>
            </>
          )}

          {/* ── Technology-specific ── */}
          {nodeType === 'technology' && (
            <>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 'var(--space-3)' }}>
                <label className="field-label">
                  Category
                  <select
                    value={techCategory}
                    onChange={e => setTechCategory(e.target.value as TechnologyCategory)}
                    className="field-input"
                  >
                    <option value="language">Language</option>
                    <option value="framework">Framework</option>
                    <option value="library">Library</option>
                    <option value="runtime">Runtime</option>
                    <option value="tool">Tool</option>
                    <option value="platform">Platform</option>
                    <option value="protocol">Protocol</option>
                  </select>
                </label>
                <label className="field-label">
                  Ring
                  <select
                    value={ring}
                    onChange={e => setRing(e.target.value as AdoptionRing)}
                    className="field-input"
                  >
                    <option value="adopt">Adopt</option>
                    <option value="trial">Trial</option>
                    <option value="assess">Assess</option>
                    <option value="hold">Hold</option>
                  </select>
                </label>
              </div>
              <label className="field-label">
                Version
                <input
                  className="field-input"
                  value={version}
                  onChange={e => setVersion(e.target.value)}
                  placeholder="e.g. 1.84.0"
                />
              </label>
              <label className="field-label">
                Rationale
                <textarea
                  className="field-input"
                  value={rationale}
                  onChange={e => setRationale(e.target.value)}
                  placeholder="Why this technology?"
                  rows={2}
                />
              </label>
              <label className="field-label">
                License
                <input
                  className="field-input"
                  value={license}
                  onChange={e => setLicense(e.target.value)}
                  placeholder="e.g. MIT, Apache-2.0"
                />
              </label>
            </>
          )}

          {/* ── Component-specific ── */}
          {nodeType === 'component' && (
            <>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 'var(--space-3)' }}>
                <label className="field-label">
                  Component Type
                  <select
                    value={componentType}
                    onChange={e => setComponentType(e.target.value as ComponentType)}
                    className="field-input"
                  >
                    <option value="module">{labelComponentType('module')}</option>
                    <option value="service">Service</option>
                    <option value="library">Library</option>
                    <option value="store">{labelComponentType('store')}</option>
                    <option value="interface">{labelComponentType('interface')}</option>
                    <option value="pipeline">{labelComponentType('pipeline')}</option>
                  </select>
                </label>
                <label className="field-label">
                  Status
                  <select
                    value={componentStatus}
                    onChange={e => setComponentStatus(e.target.value as ComponentStatus)}
                    className="field-input"
                  >
                    <option value="planned">Planned</option>
                    <option value="in_progress">In Progress</option>
                    <option value="shipped">Shipped</option>
                    <option value="deprecated">Deprecated</option>
                  </select>
                </label>
              </div>
              <label className="field-label">
                Description
                <textarea
                  className="field-input"
                  value={description}
                  onChange={e => setDescription(e.target.value)}
                  placeholder="What does this component do?"
                  rows={3}
                />
              </label>
            </>
          )}

          {/* ── Constraint-specific ── */}
          {nodeType === 'constraint' && (
            <>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 'var(--space-3)' }}>
                <label className="field-label">
                  Constraint Type
                  <select
                    value={constraintType}
                    onChange={e => setConstraintType(e.target.value as ConstraintType)}
                    className="field-input"
                  >
                    <option value="technical">Technical</option>
                    <option value="organizational">Organizational</option>
                    <option value="philosophical">Philosophical</option>
                    <option value="regulatory">Regulatory</option>
                  </select>
                </label>
                <label className="field-label">
                  Source
                  <input
                    className="field-input"
                    value={constraintSource}
                    onChange={e => setConstraintSource(e.target.value)}
                    placeholder="Who imposed this?"
                  />
                </label>
              </div>
              <label className="field-label">
                Description
                <textarea
                  className="field-input"
                  value={description}
                  onChange={e => setDescription(e.target.value)}
                  placeholder="What is the constraint?"
                  rows={3}
                />
              </label>
            </>
          )}

          {/* ── Pattern-specific ── */}
          {nodeType === 'pattern' && (
            <>
              <label className="field-label">
                Description
                <textarea
                  className="field-input"
                  value={description}
                  onChange={e => setDescription(e.target.value)}
                  placeholder="Describe the pattern…"
                  rows={3}
                />
              </label>
              <label className="field-label">
                Rationale
                <textarea
                  className="field-input"
                  value={patternRationale}
                  onChange={e => setPatternRationale(e.target.value)}
                  placeholder="Why use this pattern?"
                  rows={2}
                />
              </label>
            </>
          )}

          {/* ── Quality Requirement-specific ── */}
          {nodeType === 'quality_requirement' && (
            <>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 'var(--space-3)' }}>
                <label className="field-label">
                  Quality Attribute
                  <select
                    value={attribute}
                    onChange={e => setAttribute(e.target.value as QualityAttribute)}
                    className="field-input"
                  >
                    <option value="performance">Performance</option>
                    <option value="reliability">Reliability</option>
                    <option value="security">Security</option>
                    <option value="usability">Usability</option>
                    <option value="maintainability">Maintainability</option>
                  </select>
                </label>
                <label className="field-label">
                  Priority
                  <select
                    value={priority}
                    onChange={e => setPriority(e.target.value as QualityPriority)}
                    className="field-input"
                  >
                    <option value="critical">Critical</option>
                    <option value="high">High</option>
                    <option value="medium">Medium</option>
                    <option value="low">Low</option>
                  </select>
                </label>
              </div>
              <label className="field-label">
                Scenario
                <textarea
                  className="field-input"
                  value={scenario}
                  onChange={e => setScenario(e.target.value)}
                  placeholder="Describe the quality scenario…"
                  rows={3}
                />
              </label>
            </>
          )}

          {/* Tags (all types) */}
          <label className="field-label">
            Tags
            <input
              className="field-input"
              value={tags}
              onChange={e => setTags(e.target.value)}
              placeholder="comma, separated, tags"
            />
          </label>

          {error && (
            <div style={{
              padding: 'var(--space-2) var(--space-3)',
              background: 'var(--color-error-bg, rgba(255,59,48,0.1))',
              color: 'var(--color-error)',
              borderRadius: 'var(--radius-sm)',
              fontSize: 'var(--text-xs)',
            }}>
              {error}
            </div>
          )}
        </div>

        <div className="modal-footer">
          <button className="btn btn-ghost" onClick={handleClose} disabled={saving}>
            Cancel
          </button>
          <button className="btn btn-primary" onClick={handleSubmit} disabled={saving}>
            {saving ? 'Creating…' : 'Create Node'}
          </button>
        </div>
      </div>
    </div>
  );
}
