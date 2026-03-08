import { describe, expect, it } from 'vitest';
import {
  labelComponentType,
  labelNodeType,
  labelScopeClass,
  labelScopeField,
  labelScopeVisibility,
  labelSecondaryScopeField,
  labelSessionField,
} from '../taxonomy';

describe('taxonomy labels', () => {
  it('maps node type labels with singular/plural/short variants', () => {
    expect(labelNodeType('quality_requirement')).toBe('Quality Scenario');
    expect(labelNodeType('quality_requirement', 'plural')).toBe('Quality Scenarios');
    expect(labelNodeType('quality_requirement', 'short')).toBe('Quality');
    expect(labelNodeType('decision', 'plural')).toBe('Decisions');
  });

  it('maps scope class labels including intervention context', () => {
    expect(labelScopeClass('project_contextual')).toBe('Project Context');
    expect(labelScopeClass('unscoped')).toBe('Unscoped');
    expect(labelScopeClass('unscoped', 'intervention')).toBe('Needs Scope');
  });

  it('maps scope visibility labels', () => {
    expect(labelScopeVisibility('shared')).toBe('Shared across projects');
    expect(labelScopeVisibility('shared', 'short')).toBe('Shared');
    expect(labelScopeVisibility('project_local')).toBe('Project Only');
  });

  it('maps component subtype labels', () => {
    expect(labelComponentType('module')).toBe('Application Module');
    expect(labelComponentType('store')).toBe('Data Store');
    expect(labelComponentType('interface')).toBe('Interface Surface');
    expect(labelComponentType('pipeline')).toBe('Automation Pipeline');
  });

  it('maps shared field labels and session brief labels', () => {
    expect(labelSecondaryScopeField('feature')).toBe('Feature Area');
    expect(labelSecondaryScopeField('widget')).toBe('Surface');
    expect(labelSecondaryScopeField('component')).toBe('Related Component');
    expect(labelScopeField('scope_class')).toBe('Placement');
    expect(labelScopeField('scope_visibility')).toBe('Availability');
    expect(labelScopeField('linked_project_ids')).toBe('Shared with projects');
    expect(labelScopeField('shared_source_id')).toBe('Overrides shared record');
    expect(labelSessionField('project_description')).toBe('Planning brief');
  });
});
