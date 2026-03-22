import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import AddEdgeModal from '../AddEdgeModal.tsx';
import type { NodeSummary } from '../../types/blueprint.ts';

const nodes: NodeSummary[] = [
  {
    id: 'component-api',
    name: 'API Service',
    node_type: 'component',
    status: 'shipped',
    tags: [],
    has_documentation: false,
    updated_at: '2026-03-22T00:00:00Z',
  },
  {
    id: 'technology-pg',
    name: 'Postgres',
    node_type: 'technology',
    ring: 'adopt',
    category: 'database',
    tags: [],
    has_documentation: false,
    updated_at: '2026-03-22T00:00:00Z',
  },
];

describe('AddEdgeModal', () => {
  it('renders the shared modal structure and submits the selected relationship', async () => {
    const user = userEvent.setup();
    const onCreate = vi.fn().mockResolvedValue(undefined);
    const onClose = vi.fn();

    render(
      <AddEdgeModal
        isOpen={true}
        nodes={nodes}
        onCreate={onCreate}
        onClose={onClose}
      />,
    );

    expect(screen.getByText(/connect two blueprint nodes/i)).toBeInTheDocument();

    const selects = screen.getAllByRole('combobox');
    await user.selectOptions(selects[0], 'component-api');
    await user.selectOptions(selects[1], 'uses');
    await user.selectOptions(selects[2], 'technology-pg');
    await user.type(
      screen.getByRole('textbox', { name: /why this relationship/i }),
      'primary storage path',
    );

    await user.click(screen.getByRole('button', { name: /create edge/i }));

    expect(onCreate).toHaveBeenCalledWith({
      source: 'component-api',
      target: 'technology-pg',
      edge_type: 'uses',
      metadata: 'primary storage path',
    });
  });
});
