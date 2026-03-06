import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import DetailDrawer from '../DetailDrawer.tsx';
import EditNodeForm from '../EditNodeForm.tsx';
import NodeListPanel from '../NodeListPanel.tsx';
import type { ApiClient } from '../../api/client.ts';
import type { BlueprintEventPayload, DecisionNode, NodeSummary } from '../../types/blueprint.ts';

const decisionNode: DecisionNode = {
  node_type: 'decision',
  id: 'dec-docs',
  title: 'Document the merge patch flow',
  status: 'accepted',
  context: 'Partial updates should merge rather than replace.',
  options: [],
  consequences: [],
  assumptions: [],
  tags: ['docs'],
  documentation: '## Architecture Notes\n\nUse merge patch.',
  created_at: '2026-03-06T00:00:00Z',
  updated_at: '2026-03-06T00:00:00Z',
};

describe('Blueprint documentation UI', () => {
  it('shows the documentation textarea in EditNodeForm', () => {
    render(
      <EditNodeForm
        node={decisionNode}
        onSave={vi.fn().mockResolvedValue(undefined)}
        onCancel={vi.fn()}
        saving={false}
      />,
    );

    expect(screen.getByText(/Documentation \(markdown\)/i)).toBeInTheDocument();
    const textareas = screen.getAllByRole('textbox');
    expect(textareas[textareas.length - 1]).toHaveValue('## Architecture Notes\n\nUse merge patch.');
  });

  it('shows a docs badge in NodeListPanel when documentation exists', () => {
    const summaries: NodeSummary[] = [
      {
        id: 'dec-docs',
        name: 'Document the merge patch flow',
        node_type: 'decision',
        status: 'Accepted',
        tags: ['docs'],
        has_documentation: true,
        updated_at: '2026-03-06T00:00:00Z',
      },
    ];

    render(
      <NodeListPanel
        nodes={summaries}
        edges={[]}
        nodeType={null}
        onSelectNode={vi.fn()}
      />,
    );

    expect(screen.getByTitle('Documentation attached')).toBeInTheDocument();
  });

  it('renders markdown in the DetailDrawer docs tab', async () => {
    const api = {
      getBlueprintNode: vi.fn().mockResolvedValue(decisionNode),
      listBlueprintEvents: vi.fn().mockResolvedValue({
        events: [] as BlueprintEventPayload[],
        total: 0,
      }),
      updateBlueprintNode: vi.fn().mockResolvedValue(decisionNode),
    } as unknown as ApiClient;

    render(
      <DetailDrawer
        nodeId="dec-docs"
        allNodes={[{ id: 'dec-docs', name: decisionNode.title, node_type: 'decision' }]}
        edges={[]}
        api={api}
        onClose={vi.fn()}
        onNavigateNode={vi.fn()}
        onImpactPreview={vi.fn()}
      />,
    );

    const docsTab = await screen.findByRole('button', { name: 'Docs' });
    fireEvent.click(docsTab);

    expect(await screen.findByText('Architecture Notes')).toBeInTheDocument();
    expect(screen.getByText('Use merge patch.')).toBeInTheDocument();
  });
});
