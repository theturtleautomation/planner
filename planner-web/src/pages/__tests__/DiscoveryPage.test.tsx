import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import DiscoveryPage from '../DiscoveryPage.tsx';
import type { ProposedNode, ProposedEdge } from '../../types/blueprint.ts';

const mockListProposedNodes = vi.fn();
const mockListProposedEdges = vi.fn();
const mockRunDiscoveryScan = vi.fn();
const mockAcceptProposal = vi.fn();
const mockRejectProposal = vi.fn();
const mockAcceptEdgeProposal = vi.fn();
const mockRejectEdgeProposal = vi.fn();
const mockGetAccessToken = vi.fn().mockResolvedValue('mock-token');

vi.mock('../../api/client.ts', () => ({
  createApiClient: vi.fn(() => ({
    listProposedNodes: mockListProposedNodes,
    listProposedEdges: mockListProposedEdges,
    runDiscoveryScan: mockRunDiscoveryScan,
    acceptProposal: mockAcceptProposal,
    rejectProposal: mockRejectProposal,
    acceptEdgeProposal: mockAcceptEdgeProposal,
    rejectEdgeProposal: mockRejectEdgeProposal,
  })),
}));

vi.mock('../../auth/useAuthenticatedFetch.ts', () => ({
  useGetAccessToken: vi.fn(() => mockGetAccessToken),
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

function makeEdgeProposal(): ProposedEdge {
  return {
    id: 'edge-proposal-1',
    edge: {
      source: 'comp-review-controls',
      target: 'comp-task-list',
      edge_type: 'depends_on',
      metadata: 'import graph',
    },
    source: 'code_graph_context',
    reason: 'Review controls imports task-list reordering helpers.',
    status: 'pending',
    proposed_at: '2026-03-07T00:00:00Z',
    confidence: 0.87,
    source_artifact: 'codegraph:task-tracker',
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
    mockGetAccessToken.mockResolvedValue('mock-token');
    mockRunDiscoveryScan.mockResolvedValue({ results: [], total_proposed: 0, total_edge_proposed: 0 });
    mockAcceptProposal.mockResolvedValue({});
    mockRejectProposal.mockResolvedValue({});
    mockAcceptEdgeProposal.mockResolvedValue({});
    mockRejectEdgeProposal.mockResolvedValue({});
    mockListProposedNodes.mockResolvedValue({
      proposals: [makeProposal()],
      total: 1,
    });
    mockListProposedEdges.mockResolvedValue({
      proposals: [makeEdgeProposal()],
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

  it('allows inline component rename before accept and sends manual node_patch', async () => {
    const user = userEvent.setup();
    mockAcceptProposal.mockResolvedValueOnce({ node_id: 'comp-task-widget', message: 'ok' });

    render(
      <MemoryRouter initialEntries={['/discovery']}>
        <Routes>
          <Route path="/discovery" element={<DiscoveryPage />} />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(mockListProposedNodes).toHaveBeenCalledTimes(1);
    });

    await user.click(await screen.findByText('Task Tracker Widget'));
    const input = await screen.findByLabelText(/suggested component name/i);
    expect(input).toHaveValue('Task Tracker Widget');
    fireEvent.change(input, { target: { value: 'Identity Widget' } });
    expect(input).toHaveValue('Identity Widget');

    await user.click(await screen.findByRole('button', { name: /✓\s*accept/i }));

    expect(mockAcceptProposal).toHaveBeenCalledWith('proposal-task-widget', {
      node_patch: {
        name: 'Identity Widget',
        naming: {
          source: 'manual',
        },
      },
    });
  });

  it('loads edge proposals and accepts a pending edge', async () => {
    const user = userEvent.setup();

    render(
      <MemoryRouter initialEntries={['/discovery']}>
        <Routes>
          <Route path="/discovery" element={<DiscoveryPage />} />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(mockListProposedNodes).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByRole('button', { name: /edge proposals/i }));

    await waitFor(() => {
      expect(mockListProposedEdges).toHaveBeenCalledTimes(1);
    });

    await user.click(await screen.findByText('comp-review-controls -> comp-task-list'));
    await user.click(await screen.findByRole('button', { name: /accept edge/i }));

    expect(mockAcceptEdgeProposal).toHaveBeenCalledWith('edge-proposal-1');
  });

  it('groups pending and reviewed proposals when all statuses are visible', async () => {
    const user = userEvent.setup();
    const reviewedProposal = {
      ...makeProposal(),
      id: 'proposal-reviewed',
      status: 'accepted' as const,
      reviewed_at: '2026-03-08T00:00:00Z',
      node: {
        ...makeProposal().node,
        id: 'comp-reviewed-widget',
        name: 'Reviewed Widget',
      },
    };

    mockListProposedNodes
      .mockResolvedValueOnce({
        proposals: [makeProposal()],
        total: 1,
      })
      .mockResolvedValueOnce({
        proposals: [makeProposal(), reviewedProposal],
        total: 2,
      });

    render(
      <MemoryRouter initialEntries={['/discovery']}>
        <Routes>
          <Route path="/discovery" element={<DiscoveryPage />} />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(mockListProposedNodes).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByRole('button', { name: /^all$/i }));

    await waitFor(() => {
      expect(mockListProposedNodes).toHaveBeenCalledTimes(2);
    });

    expect(screen.getByRole('heading', { name: /pending review/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /^reviewed$/i })).toBeInTheDocument();
    expect(screen.getByText('Reviewed Widget')).toBeInTheDocument();
  });

  it('shows the empty-state review copy when no proposals are present', async () => {
    mockListProposedNodes.mockResolvedValueOnce({
      proposals: [],
      total: 0,
    });

    render(
      <MemoryRouter initialEntries={['/discovery']}>
        <Routes>
          <Route path="/discovery" element={<DiscoveryPage />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByText(/no pending proposals\./i)).toBeInTheDocument();
  });

  it('shows scan-in-progress feedback while a discovery scan is running', async () => {
    const user = userEvent.setup();
    let resolveScan: ((value: { results: never[]; total_proposed: number; total_edge_proposed: number }) => void) | null = null;
    mockRunDiscoveryScan.mockImplementationOnce(() => (
      new Promise((resolve) => {
        resolveScan = resolve;
      })
    ));

    render(
      <MemoryRouter initialEntries={['/discovery']}>
        <Routes>
          <Route path="/discovery" element={<DiscoveryPage />} />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(mockListProposedNodes).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByRole('button', { name: /run discovery scan/i }));

    expect(screen.getByRole('button', { name: /scanning/i })).toBeDisabled();

    await act(async () => {
      resolveScan?.({ results: [], total_proposed: 0, total_edge_proposed: 0 });
    });
  });
});
