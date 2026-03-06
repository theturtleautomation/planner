import { useState, useCallback } from 'react';
import type {
  BlueprintNode,
  DecisionNode,
  TechnologyNode,
  ComponentNode,
  ConstraintNode,
  PatternNode,
  QualityRequirementNode,
  DecisionStatus,
  AdoptionRing,
  TechnologyCategory,
  ComponentType,
  ComponentStatus,
  ConstraintType,
  QualityAttribute,
  QualityPriority,
  DecisionOption,
  Consequence,
  Assumption,
} from '../types/blueprint.ts';

// ─── Props ──────────────────────────────────────────────────────────────────

interface EditNodeFormProps {
  node: BlueprintNode;
  onSave: (updated: BlueprintNode) => Promise<void>;
  onCancel: () => void;
  saving: boolean;
}

// ─── Helpers ────────────────────────────────────────────────────────────────

function nowIso(): string {
  return new Date().toISOString();
}

function DocumentationField({
  value,
  onChange,
}: {
  value?: string;
  onChange: (value?: string) => void;
}) {
  return (
    <>
      <label className="field-label">Documentation (markdown)</label>
      <textarea
        className="field-input"
        rows={6}
        value={value ?? ''}
        onChange={e => onChange(e.target.value.trim() ? e.target.value : undefined)}
      />
    </>
  );
}

// ─── Decision form ──────────────────────────────────────────────────────────

function EditDecision({ node, onChange }: { node: DecisionNode; onChange: (n: DecisionNode) => void }) {
  const setField = <K extends keyof DecisionNode>(key: K, value: DecisionNode[K]) =>
    onChange({ ...node, [key]: value, updated_at: nowIso() });

  const updateOption = (idx: number, opt: DecisionOption) => {
    const next = [...node.options];
    next[idx] = opt;
    setField('options', next);
  };

  const addOption = () => {
    setField('options', [...node.options, { name: '', pros: [], cons: [], chosen: false }]);
  };

  const removeOption = (idx: number) => {
    setField('options', node.options.filter((_, i) => i !== idx));
  };

  const updateConsequence = (idx: number, c: Consequence) => {
    const next = [...node.consequences];
    next[idx] = c;
    setField('consequences', next);
  };

  const addConsequence = () => {
    setField('consequences', [...node.consequences, { description: '', positive: true }]);
  };

  const removeConsequence = (idx: number) => {
    setField('consequences', node.consequences.filter((_, i) => i !== idx));
  };

  const updateAssumption = (idx: number, a: Assumption) => {
    const next = [...node.assumptions];
    next[idx] = a;
    setField('assumptions', next);
  };

  const addAssumption = () => {
    setField('assumptions', [...node.assumptions, { description: '', confidence: 'medium' }]);
  };

  const removeAssumption = (idx: number) => {
    setField('assumptions', node.assumptions.filter((_, i) => i !== idx));
  };

  return (
    <>
      <label className="field-label">Title</label>
      <input className="field-input" value={node.title} onChange={e => setField('title', e.target.value)} />

      <label className="field-label">Status</label>
      <select className="field-input" value={node.status} onChange={e => setField('status', e.target.value as DecisionStatus)}>
        <option value="proposed">Proposed</option>
        <option value="accepted">Accepted</option>
        <option value="superseded">Superseded</option>
        <option value="deprecated">Deprecated</option>
      </select>

      <label className="field-label">Context</label>
      <textarea className="field-input" rows={3} value={node.context} onChange={e => setField('context', e.target.value)} />

      {/* Options */}
      <div className="field-label" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        Options
        <button className="btn btn-outline" style={{ fontSize: '0.625rem', padding: '2px 8px' }} onClick={addOption}>+ Add</button>
      </div>
      {node.options.map((opt, i) => (
        <div key={i} style={{ border: '1px solid var(--color-border)', borderRadius: 'var(--radius-md)', padding: 'var(--space-3)', marginBottom: 'var(--space-2)' }}>
          <div style={{ display: 'flex', gap: 'var(--space-2)', alignItems: 'center', marginBottom: 'var(--space-2)' }}>
            <input className="field-input" placeholder="Option name" value={opt.name} onChange={e => updateOption(i, { ...opt, name: e.target.value })} style={{ flex: 1 }} />
            <label style={{ display: 'flex', alignItems: 'center', gap: '4px', fontSize: '0.6875rem', whiteSpace: 'nowrap' }}>
              <input type="checkbox" checked={opt.chosen} onChange={e => updateOption(i, { ...opt, chosen: e.target.checked })} />
              Chosen
            </label>
            <button className="btn btn-outline" style={{ fontSize: '0.625rem', padding: '2px 6px', color: 'var(--color-error)' }} onClick={() => removeOption(i)}>✕</button>
          </div>
          <input className="field-input" placeholder="Pros (comma-separated)" value={opt.pros.join(', ')}
            onChange={e => updateOption(i, { ...opt, pros: e.target.value.split(',').map(s => s.trim()).filter(Boolean) })}
            style={{ fontSize: '0.6875rem', marginBottom: '4px' }} />
          <input className="field-input" placeholder="Cons (comma-separated)" value={opt.cons.join(', ')}
            onChange={e => updateOption(i, { ...opt, cons: e.target.value.split(',').map(s => s.trim()).filter(Boolean) })}
            style={{ fontSize: '0.6875rem' }} />
        </div>
      ))}

      {/* Consequences */}
      <div className="field-label" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        Consequences
        <button className="btn btn-outline" style={{ fontSize: '0.625rem', padding: '2px 8px' }} onClick={addConsequence}>+ Add</button>
      </div>
      {node.consequences.map((c, i) => (
        <div key={i} style={{ display: 'flex', gap: 'var(--space-2)', alignItems: 'center', marginBottom: 'var(--space-2)' }}>
          <input className="field-input" value={c.description} onChange={e => updateConsequence(i, { ...c, description: e.target.value })} style={{ flex: 1 }} />
          <label style={{ display: 'flex', alignItems: 'center', gap: '4px', fontSize: '0.6875rem', whiteSpace: 'nowrap' }}>
            <input type="checkbox" checked={c.positive} onChange={e => updateConsequence(i, { ...c, positive: e.target.checked })} />
            +
          </label>
          <button className="btn btn-outline" style={{ fontSize: '0.625rem', padding: '2px 6px', color: 'var(--color-error)' }} onClick={() => removeConsequence(i)}>✕</button>
        </div>
      ))}

      {/* Assumptions */}
      <div className="field-label" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        Assumptions
        <button className="btn btn-outline" style={{ fontSize: '0.625rem', padding: '2px 8px' }} onClick={addAssumption}>+ Add</button>
      </div>
      {node.assumptions.map((a, i) => (
        <div key={i} style={{ display: 'flex', gap: 'var(--space-2)', alignItems: 'center', marginBottom: 'var(--space-2)' }}>
          <input className="field-input" value={a.description} onChange={e => updateAssumption(i, { ...a, description: e.target.value })} style={{ flex: 1 }} />
          <select className="field-input" value={a.confidence} onChange={e => updateAssumption(i, { ...a, confidence: e.target.value })} style={{ width: '90px' }}>
            <option value="high">High</option>
            <option value="medium">Medium</option>
            <option value="low">Low</option>
          </select>
          <button className="btn btn-outline" style={{ fontSize: '0.625rem', padding: '2px 6px', color: 'var(--color-error)' }} onClick={() => removeAssumption(i)}>✕</button>
        </div>
      ))}

      <label className="field-label">Tags (comma-separated)</label>
      <input className="field-input" value={node.tags.join(', ')} onChange={e => setField('tags', e.target.value.split(',').map(s => s.trim()).filter(Boolean))} />

      <DocumentationField value={node.documentation} onChange={value => setField('documentation', value)} />
    </>
  );
}

// ─── Technology form ────────────────────────────────────────────────────────

function EditTechnology({ node, onChange }: { node: TechnologyNode; onChange: (n: TechnologyNode) => void }) {
  const setField = <K extends keyof TechnologyNode>(key: K, value: TechnologyNode[K]) =>
    onChange({ ...node, [key]: value, updated_at: nowIso() });

  return (
    <>
      <label className="field-label">Name</label>
      <input className="field-input" value={node.name} onChange={e => setField('name', e.target.value)} />

      <label className="field-label">Category</label>
      <select className="field-input" value={node.category} onChange={e => setField('category', e.target.value as TechnologyCategory)}>
        {(['language', 'framework', 'library', 'runtime', 'tool', 'platform', 'protocol'] as const).map(c => (
          <option key={c} value={c}>{c.charAt(0).toUpperCase() + c.slice(1)}</option>
        ))}
      </select>

      <label className="field-label">Adoption Ring</label>
      <select className="field-input" value={node.ring} onChange={e => setField('ring', e.target.value as AdoptionRing)}>
        {(['adopt', 'trial', 'assess', 'hold'] as const).map(r => (
          <option key={r} value={r}>{r.charAt(0).toUpperCase() + r.slice(1)}</option>
        ))}
      </select>

      <label className="field-label">Version</label>
      <input className="field-input" value={node.version ?? ''} onChange={e => setField('version', e.target.value || undefined)} />

      <label className="field-label">Rationale</label>
      <textarea className="field-input" rows={3} value={node.rationale} onChange={e => setField('rationale', e.target.value)} />

      <label className="field-label">License</label>
      <input className="field-input" value={node.license ?? ''} onChange={e => setField('license', e.target.value || undefined)} />

      <label className="field-label">Tags (comma-separated)</label>
      <input className="field-input" value={node.tags.join(', ')} onChange={e => setField('tags', e.target.value.split(',').map(s => s.trim()).filter(Boolean))} />

      <DocumentationField value={node.documentation} onChange={value => setField('documentation', value)} />
    </>
  );
}

// ─── Component form ─────────────────────────────────────────────────────────

function EditComponent({ node, onChange }: { node: ComponentNode; onChange: (n: ComponentNode) => void }) {
  const setField = <K extends keyof ComponentNode>(key: K, value: ComponentNode[K]) =>
    onChange({ ...node, [key]: value, updated_at: nowIso() });

  return (
    <>
      <label className="field-label">Name</label>
      <input className="field-input" value={node.name} onChange={e => setField('name', e.target.value)} />

      <label className="field-label">Component Type</label>
      <select className="field-input" value={node.component_type} onChange={e => setField('component_type', e.target.value as ComponentType)}>
        {(['module', 'service', 'library', 'store', 'interface', 'pipeline'] as const).map(t => (
          <option key={t} value={t}>{t.charAt(0).toUpperCase() + t.slice(1)}</option>
        ))}
      </select>

      <label className="field-label">Status</label>
      <select className="field-input" value={node.status} onChange={e => setField('status', e.target.value as ComponentStatus)}>
        <option value="planned">Planned</option>
        <option value="in_progress">In Progress</option>
        <option value="shipped">Shipped</option>
        <option value="deprecated">Deprecated</option>
      </select>

      <label className="field-label">Description</label>
      <textarea className="field-input" rows={3} value={node.description} onChange={e => setField('description', e.target.value)} />

      <label className="field-label">Provides (comma-separated)</label>
      <input className="field-input" value={node.provides.join(', ')} onChange={e => setField('provides', e.target.value.split(',').map(s => s.trim()).filter(Boolean))} />

      <label className="field-label">Consumes (comma-separated)</label>
      <input className="field-input" value={node.consumes.join(', ')} onChange={e => setField('consumes', e.target.value.split(',').map(s => s.trim()).filter(Boolean))} />

      <label className="field-label">Tags (comma-separated)</label>
      <input className="field-input" value={node.tags.join(', ')} onChange={e => setField('tags', e.target.value.split(',').map(s => s.trim()).filter(Boolean))} />

      <DocumentationField value={node.documentation} onChange={value => setField('documentation', value)} />
    </>
  );
}

// ─── Constraint form ────────────────────────────────────────────────────────

function EditConstraint({ node, onChange }: { node: ConstraintNode; onChange: (n: ConstraintNode) => void }) {
  const setField = <K extends keyof ConstraintNode>(key: K, value: ConstraintNode[K]) =>
    onChange({ ...node, [key]: value, updated_at: nowIso() });

  return (
    <>
      <label className="field-label">Title</label>
      <input className="field-input" value={node.title} onChange={e => setField('title', e.target.value)} />

      <label className="field-label">Constraint Type</label>
      <select className="field-input" value={node.constraint_type} onChange={e => setField('constraint_type', e.target.value as ConstraintType)}>
        {(['technical', 'organizational', 'philosophical', 'regulatory'] as const).map(t => (
          <option key={t} value={t}>{t.charAt(0).toUpperCase() + t.slice(1)}</option>
        ))}
      </select>

      <label className="field-label">Description</label>
      <textarea className="field-input" rows={3} value={node.description} onChange={e => setField('description', e.target.value)} />

      <label className="field-label">Source</label>
      <input className="field-input" value={node.source} onChange={e => setField('source', e.target.value)} />

      <label className="field-label">Tags (comma-separated)</label>
      <input className="field-input" value={node.tags.join(', ')} onChange={e => setField('tags', e.target.value.split(',').map(s => s.trim()).filter(Boolean))} />

      <DocumentationField value={node.documentation} onChange={value => setField('documentation', value)} />
    </>
  );
}

// ─── Pattern form ───────────────────────────────────────────────────────────

function EditPattern({ node, onChange }: { node: PatternNode; onChange: (n: PatternNode) => void }) {
  const setField = <K extends keyof PatternNode>(key: K, value: PatternNode[K]) =>
    onChange({ ...node, [key]: value, updated_at: nowIso() });

  return (
    <>
      <label className="field-label">Name</label>
      <input className="field-input" value={node.name} onChange={e => setField('name', e.target.value)} />

      <label className="field-label">Description</label>
      <textarea className="field-input" rows={3} value={node.description} onChange={e => setField('description', e.target.value)} />

      <label className="field-label">Rationale</label>
      <textarea className="field-input" rows={3} value={node.rationale} onChange={e => setField('rationale', e.target.value)} />

      <label className="field-label">Tags (comma-separated)</label>
      <input className="field-input" value={node.tags.join(', ')} onChange={e => setField('tags', e.target.value.split(',').map(s => s.trim()).filter(Boolean))} />

      <DocumentationField value={node.documentation} onChange={value => setField('documentation', value)} />
    </>
  );
}

// ─── Quality Requirement form ───────────────────────────────────────────────

function EditQualityRequirement({ node, onChange }: { node: QualityRequirementNode; onChange: (n: QualityRequirementNode) => void }) {
  const setField = <K extends keyof QualityRequirementNode>(key: K, value: QualityRequirementNode[K]) =>
    onChange({ ...node, [key]: value, updated_at: nowIso() });

  return (
    <>
      <label className="field-label">Quality Attribute</label>
      <select className="field-input" value={node.attribute} onChange={e => setField('attribute', e.target.value as QualityAttribute)}>
        {(['performance', 'reliability', 'security', 'usability', 'maintainability'] as const).map(a => (
          <option key={a} value={a}>{a.charAt(0).toUpperCase() + a.slice(1)}</option>
        ))}
      </select>

      <label className="field-label">Scenario</label>
      <textarea className="field-input" rows={3} value={node.scenario} onChange={e => setField('scenario', e.target.value)} />

      <label className="field-label">Priority</label>
      <select className="field-input" value={node.priority} onChange={e => setField('priority', e.target.value as QualityPriority)}>
        {(['critical', 'high', 'medium', 'low'] as const).map(p => (
          <option key={p} value={p}>{p.charAt(0).toUpperCase() + p.slice(1)}</option>
        ))}
      </select>

      <label className="field-label">Tags (comma-separated)</label>
      <input className="field-input" value={node.tags.join(', ')} onChange={e => setField('tags', e.target.value.split(',').map(s => s.trim()).filter(Boolean))} />

      <DocumentationField value={node.documentation} onChange={value => setField('documentation', value)} />
    </>
  );
}

// ─── Main EditNodeForm ──────────────────────────────────────────────────────

export default function EditNodeForm({ node, onSave, onCancel, saving }: EditNodeFormProps) {
  const [draft, setDraft] = useState<BlueprintNode>(() => structuredClone(node));
  const [error, setError] = useState<string | null>(null);

  const handleSave = useCallback(async () => {
    setError(null);
    try {
      await onSave(draft);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Save failed');
    }
  }, [draft, onSave]);

  const renderForm = () => {
    switch (draft.node_type) {
      case 'decision':
        return <EditDecision node={draft as DecisionNode} onChange={n => setDraft(n)} />;
      case 'technology':
        return <EditTechnology node={draft as TechnologyNode} onChange={n => setDraft(n)} />;
      case 'component':
        return <EditComponent node={draft as ComponentNode} onChange={n => setDraft(n)} />;
      case 'constraint':
        return <EditConstraint node={draft as ConstraintNode} onChange={n => setDraft(n)} />;
      case 'pattern':
        return <EditPattern node={draft as PatternNode} onChange={n => setDraft(n)} />;
      case 'quality_requirement':
        return <EditQualityRequirement node={draft as QualityRequirementNode} onChange={n => setDraft(n)} />;
      default:
        return <div style={{ color: 'var(--color-error)' }}>Unknown node type</div>;
    }
  };

  return (
    <div className="edit-node-form">
      <div className="edit-node-form-body">
        {renderForm()}
      </div>

      {error && (
        <div style={{ color: 'var(--color-error)', fontSize: 'var(--text-xs)', padding: 'var(--space-2) 0' }}>
          {error}
        </div>
      )}

      <div className="edit-node-form-actions">
        <button className="btn btn-outline" onClick={onCancel} disabled={saving}>
          Cancel
        </button>
        <button className="btn btn-primary" onClick={handleSave} disabled={saving}>
          {saving ? 'Saving…' : 'Save Changes'}
        </button>
      </div>
    </div>
  );
}
