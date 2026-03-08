import type { ComponentType, NodeType, ScopeClass, ScopeVisibility } from '../types/blueprint.ts';

type NodeTypeVariant = 'singular' | 'plural' | 'short';

const NODE_TYPE_LABELS: Record<NodeType, { singular: string; plural: string; short: string }> = {
  decision: {
    singular: 'Decision',
    plural: 'Decisions',
    short: 'Decision',
  },
  technology: {
    singular: 'Technology',
    plural: 'Technologies',
    short: 'Technology',
  },
  component: {
    singular: 'Component',
    plural: 'Components',
    short: 'Component',
  },
  constraint: {
    singular: 'Constraint',
    plural: 'Constraints',
    short: 'Constraint',
  },
  pattern: {
    singular: 'Pattern',
    plural: 'Patterns',
    short: 'Pattern',
  },
  quality_requirement: {
    singular: 'Quality Scenario',
    plural: 'Quality Scenarios',
    short: 'Quality',
  },
};

export function labelNodeType(value: string, variant: NodeTypeVariant = 'singular'): string {
  const entry = NODE_TYPE_LABELS[value as NodeType];
  if (!entry) return value.replace(/_/g, ' ');
  return entry[variant];
}

export function labelScopeClass(value: ScopeClass | string, context: 'default' | 'intervention' = 'default'): string {
  if (value === 'project_contextual') return 'Project Context';
  if (value === 'project') return 'Project';
  if (value === 'global') return 'Global';
  if (value === 'unscoped') {
    return context === 'intervention' ? 'Needs Scope' : 'Unscoped';
  }
  return String(value).replace(/_/g, ' ');
}

export function labelScopeVisibility(value: ScopeVisibility | 'unscoped' | string, variant: 'default' | 'short' = 'default'): string {
  if (value === 'shared') return variant === 'short' ? 'Shared' : 'Shared across projects';
  if (value === 'project_local') return 'Project Only';
  if (value === 'unscoped') return 'Unscoped';
  return String(value).replace(/_/g, ' ');
}

export function labelComponentType(value: ComponentType | string): string {
  switch (value) {
    case 'module':
      return 'Application Module';
    case 'store':
      return 'Data Store';
    case 'interface':
      return 'Interface Surface';
    case 'pipeline':
      return 'Automation Pipeline';
    case 'service':
      return 'Service';
    case 'library':
      return 'Library';
    default:
      return String(value).replace(/_/g, ' ');
  }
}

export function labelSecondaryScopeField(value: 'feature' | 'widget' | 'artifact' | 'component'): string {
  if (value === 'feature') return 'Feature Area';
  if (value === 'widget') return 'Surface';
  if (value === 'artifact') return 'Artifact';
  return 'Related Component';
}

export function labelScopeField(value: 'scope_class' | 'scope_visibility' | 'project_id' | 'project_name' | 'linked_project_ids' | 'shared_source_id' | 'inherit_to_linked_projects' | 'is_shared'): string {
  switch (value) {
    case 'scope_class':
      return 'Placement';
    case 'scope_visibility':
      return 'Availability';
    case 'project_id':
      return 'Project reference';
    case 'project_name':
      return 'Project name';
    case 'linked_project_ids':
      return 'Shared with projects';
    case 'shared_source_id':
      return 'Overrides shared record';
    case 'inherit_to_linked_projects':
      return 'Show in linked projects';
    case 'is_shared':
      return 'Shared across projects';
    default:
      return value;
  }
}

export function labelSessionField(value: 'project_description'): string {
  if (value === 'project_description') return 'Planning brief';
  return value;
}
