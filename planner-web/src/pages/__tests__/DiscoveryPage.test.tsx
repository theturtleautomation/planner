import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import DiscoveryPage from '../DiscoveryPage.tsx';
import type { ProposedNode } from '../../types/blueprint.ts';

const mockListProposedNodes = vi.fn();
const mockRunDiscoveryScan = vi.fn();
const mockAcceptProposal = vi.fn();
const mockRejectProposal = vi.fn();

vi.mock('../../api/client.ts', () => ({
  createApiClient: vi.fn(() => ({
    listProposedNodes: mockListProposedNodes,
    runDiscoveryScan: mockRunDiscoveryScan,
    acceptProposal: mockAcceptProposal,
    rejectProposal: mockRejectProposal,
  })),
}));

vi.mock('../../auth/useAuthenticatedFetch.ts', () => ({
  useGetAccessToken: vi.fn(() => vi.fn().mockResolvedValue('mock-token')),
}));

vi.mock('../../components/Layout.tsx', () => ({
  default: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

function makeProposal(): ProposedNode {
  return {
    id: 'proposal-task-widget',
    node: {
      node_type: 'component',
      id: 'comp-task-widget',
      name: 'Task Tracker Widget',
      component_type: 'interface',
      description: 'Widget component for task tracking.',
      provides: [],
      consumes: [],
      status: 'shipped',
      tags: ['task-tracker'],
      scope: {
        scope_class: 'project_contextual',
        project: {
          project_id: 'proj-task-tracker',
          project_name: 'Task Tracker',
        },
        secondary: {
          feature: 'task_management',
          widget: 'task-tracker',
          artifact: 'widgets/task-tracker.tsx',
          component: 'task-widget',
        },
        is_shared: false,
      },
      created_at: '2026-03-07T00:00:00Z',
      updated_at: '2026-03-07T00:00:00Z',
    },
    source: 'directory_scan',
    reason: 'Component detected from task tracker widget files.',
    status: 'pending',
    proposed_at: '2026-03-07T00:00:00Z',
    confidence: 0.9,
    source_artifact: 'widgets/task-tracker.tsx',
  };
}

function LocationSnapshot() {
  const location = useLocation();
  const params = Object.fromEntries(new URLSearchParams(location.search).entries());
  return (
    <div>
      <div data-testid="location-path">{location.pathname}</div>
      <pre data-testid="location-params">{JSON.stringify(params)}</pre>
    </div>
  );
}

describe('DiscoveryPage Phase 4 contextual links', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockRunDiscoveryScan.mockResolvedValue({ results: [], total_proposed: 0 });
    mockAcceptProposal.mockResolvedValue({});
    mockRejectProposal.mockResolvedValue({});
    mockListProposedNodes.mockResolvedValue({
      proposals: [makeProposal()],
      total: 1,
    });
  });

  it('opens related knowledge from discovery proposal context', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={['/discovery']}>
        <Routes>
          <Route path="/discovery" element={<DiscoveryPage />} />
          <Route path="/knowledge/projects/:projectId" element={<LocationSnapshot />} />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(mockListProposedNodes).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByRole('button', { name: /view related knowledge/i }));

    expect(await screen.findByTestId('location-path')).toHaveTextContent('/knowledge/projects/proj-task-tracker');
    const params = JSON.parse(screen.getByTestId('location-params').textContent ?? '{}') as Record<string, string>;
    expect(params.project).toBe('proj-task-tracker');
    expect(params.feature).toBe('task_management');
    expect(params.widget).toBe('task-tracker');
    expect(params.artifact).toBe('widgets/task-tracker.tsx');
    expect(params.component).toBe('task-widget');
    expect(params.from).toBe('/discovery');
    expect(params.from_label).toBe('Discovery');
  });

  it('disables related knowledge action when proposal scope identity is unavailable', async () => {
    const proposal = makeProposal();
    mockListProposedNodes.mockResolvedValueOnce({
      proposals: [
        {
          ...proposal,
          id: 'proposal-unscoped',
          node: {
            ...proposal.node,
            scope: {
              ...proposal.node.scope,
              project: undefined,
            },
          },
        },
      ],
      total: 1,
    });

    render(
      <MemoryRouter initialEntries={['/discovery']}>
        <Routes>
          <Route path="/discovery" element={<DiscoveryPage />} />
        </Routes>
      </MemoryRouter>,
    );

    const action = await screen.findByRole('button', { name: /view related knowledge/i });
    expect(action).toBeDisabled();
    expect(action).toHaveAttribute('title', 'Scope unavailable for this proposal');
  });
});
