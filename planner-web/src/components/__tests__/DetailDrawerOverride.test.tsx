import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import DetailDrawer from '../DetailDrawer.tsx';
import type { ApiClient } from '../../api/client.ts';
import type { BlueprintEventPayload, DecisionNode } from '../../types/blueprint.ts';

function makeDecisionNode(overrides: Partial<DecisionNode> = {}): DecisionNode {
  return {
    node_type: 'decision',
    id: 'dec-local-override',
    title: 'Local Checkout Policy',
    status: 'accepted',
    context: 'Project-specific checkout flow.',
    options: [],
    consequences: [],
    assumptions: [],
    tags: [],
    scope: {
      scope_class: 'project',
      project: {
        project_id: 'proj-alpha',
        project_name: 'Alpha Project',
      },
      secondary: {},
      is_shared: false,
      lifecycle: 'active',
      override_scope: {
        shared_source_id: 'shared-guidance',
        override_reason: 'Project-specific checkout flow',
        effective_from: '2026-03-10',
      },
    },
    created_at: '2026-03-01T00:00:00Z',
    updated_at: '2026-03-10T00:00:00Z',
    ...overrides,
  };
}

function makeApi(node: DecisionNode): ApiClient {
  return {
    getBlueprintNode: vi.fn().mockResolvedValue(node),
    listBlueprintEvents: vi.fn().mockResolvedValue({
      events: [] as BlueprintEventPayload[],
      total: 0,
    }),
    updateBlueprintNode: vi.fn().mockResolvedValue(node),
  } as unknown as ApiClient;
}

describe('DetailDrawer override visibility', () => {
  it('shows shared-source lineage and precedence for local overrides', async () => {
    const node = makeDecisionNode();
    const onNavigateNode = vi.fn();
    const user = userEvent.setup();

    render(
      <DetailDrawer
        nodeId={node.id}
        allNodes={[
          { id: node.id, name: node.title, node_type: node.node_type, project_id: 'proj-alpha', project_name: 'Alpha Project' },
          { id: 'shared-guidance', name: 'Shared Guidance', node_type: 'decision', scope_visibility: 'shared' },
        ]}
        edges={[]}
        api={makeApi(node)}
        onClose={vi.fn()}
        onNavigateNode={onNavigateNode}
        onImpactPreview={vi.fn()}
      />,
    );

    expect(await screen.findByText(/shared guidance \(shared-guidance\)/i)).toBeInTheDocument();
    expect(screen.getByText(/this project-local record is the effective version in alpha project/i)).toBeInTheDocument();
    expect(screen.getByText(/override_reason/i)).toBeInTheDocument();
    expect(screen.getAllByText(/project-specific checkout flow/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/effective_from/i)).toBeInTheDocument();
    expect(screen.getAllByText(/2026-03-10/i).length).toBeGreaterThan(0);

    await user.click(screen.getByRole('button', { name: /view shared source/i }));
    expect(onNavigateNode).toHaveBeenCalledWith('shared-guidance');
  });

  it('shows inbound local overrides for shared records', async () => {
    const sharedNode = makeDecisionNode({
      id: 'shared-guidance',
      title: 'Shared Guidance',
      scope: {
        scope_class: 'global',
        secondary: {},
        is_shared: true,
        shared: {
          linked_project_ids: ['proj-alpha'],
          inherit_to_linked_projects: true,
        },
        lifecycle: 'active',
      },
    });
    const onNavigateNode = vi.fn();
    const user = userEvent.setup();

    render(
      <DetailDrawer
        nodeId={sharedNode.id}
        allNodes={[
          { id: sharedNode.id, name: sharedNode.title, node_type: sharedNode.node_type, scope_visibility: 'shared' },
          {
            id: 'dec-local-override',
            name: 'Local Checkout Policy',
            node_type: 'decision',
            override_source_id: 'shared-guidance',
            project_id: 'proj-alpha',
            project_name: 'Alpha Project',
          },
        ]}
        edges={[]}
        api={makeApi(sharedNode)}
        onClose={vi.fn()}
        onNavigateNode={onNavigateNode}
        onImpactPreview={vi.fn()}
      />,
    );

    expect(await screen.findByText(/local overrides:/i)).toBeInTheDocument();
    expect(screen.getByText(/project-local overrides supersede this shared record inside their project views/i)).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /open local checkout policy \(alpha project\)/i }));
    expect(onNavigateNode).toHaveBeenCalledWith('dec-local-override');
  });
});
