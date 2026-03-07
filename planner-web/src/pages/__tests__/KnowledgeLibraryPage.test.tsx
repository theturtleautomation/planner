import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import KnowledgeLibraryPage from '../KnowledgeLibraryPage.tsx';
import type { BlueprintResponse, NodeSummary } from '../../types/blueprint.ts';

const mockGetBlueprint = vi.fn();

vi.mock('../../api/client.ts', () => ({
  createApiClient: vi.fn(() => ({
    getBlueprint: mockGetBlueprint,
    deleteBlueprintNode: vi.fn(),
  })),
}));

vi.mock('../../auth/useAuthenticatedFetch.ts', () => ({
  useGetAccessToken: vi.fn(() => vi.fn().mockResolvedValue('mock-token')),
}));

vi.mock('../../components/Layout.tsx', () => ({
  default: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

vi.mock('../../components/NodeListPanel.tsx', () => ({
  default: () => <div data-testid="node-list-panel">Node List Panel</div>,
}));

vi.mock('../../components/DetailDrawer.tsx', () => ({
  default: () => null,
}));

vi.mock('../../components/DeleteNodeDialog.tsx', () => ({
  default: () => null,
}));

function makeNode(overrides: Partial<NodeSummary>): NodeSummary {
  return {
    id: 'node-1',
    name: 'Sample Node',
    node_type: 'decision',
    status: 'accepted',
    scope_class: 'project',
    scope_visibility: 'project_local',
    is_shared: false,
    project_id: 'proj-alpha',
    project_name: 'Alpha Project',
    secondary_scope: {},
    linked_project_ids: [],
    tags: [],
    has_documentation: false,
    updated_at: '2026-03-06T12:00:00Z',
    ...overrides,
  };
}

function makeBlueprint(): BlueprintResponse {
  return {
    nodes: [
      makeNode({
        id: 'alpha-local',
        name: 'Alpha Decision',
        project_id: 'proj-alpha',
        project_name: 'Alpha Project',
        node_type: 'decision',
        tags: ['platform', 'api'],
        has_documentation: true,
        updated_at: '2026-03-06T12:00:00Z',
      }),
      makeNode({
        id: 'alpha-shared',
        name: 'Shared Pattern',
        node_type: 'pattern',
        scope_class: 'global',
        scope_visibility: 'shared',
        is_shared: true,
        project_id: undefined,
        project_name: undefined,
        linked_project_ids: ['proj-alpha', 'proj-beta'],
        tags: ['patterns'],
        updated_at: '2026-02-20T12:00:00Z',
      }),
      makeNode({
        id: 'beta-local',
        name: 'Beta Component',
        node_type: 'component',
        project_id: 'proj-beta',
        project_name: 'Beta Suite',
        tags: ['frontend', 'growth'],
        updated_at: '2026-03-05T12:00:00Z',
      }),
      makeNode({
        id: 'beta-tech',
        name: 'Beta Runtime',
        node_type: 'technology',
        project_id: 'proj-beta',
        project_name: 'Beta Suite',
        tags: ['runtime'],
        updated_at: '2026-03-04T12:00:00Z',
      }),
    ],
    edges: [
      { source: 'alpha-local', target: 'alpha-shared', edge_type: 'depends_on' },
    ],
    counts: {
      decision: 1,
      component: 1,
      technology: 1,
      pattern: 1,
    },
    total_nodes: 4,
    total_edges: 1,
  };
}

function renderPage(route: string) {
  render(
    <MemoryRouter initialEntries={[route]}>
      <Routes>
        <Route path="/knowledge" element={<KnowledgeLibraryPage />} />
        <Route path="/knowledge/all" element={<KnowledgeLibraryPage />} />
        <Route path="/knowledge/projects/:projectId" element={<KnowledgeLibraryPage />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe('KnowledgeLibraryPage phase 2 routing', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
    mockGetBlueprint.mockResolvedValue(makeBlueprint());
  });

  it('shows project cards first on /knowledge with explicit All Knowledge entry point', async () => {
    renderPage('/knowledge');

    await waitFor(() => {
      expect(mockGetBlueprint).toHaveBeenCalled();
    });

    expect(screen.getByText('Alpha Project')).toBeInTheDocument();
    expect(screen.getByText('Beta Suite')).toBeInTheDocument();
    expect(screen.getByText('All Knowledge')).toBeInTheDocument();
    expect(screen.queryByTestId('node-list-panel')).not.toBeInTheDocument();
  });

  it('supports favorites filtering on the project chooser', async () => {
    const user = userEvent.setup();
    renderPage('/knowledge');

    await waitFor(() => {
      expect(screen.getByText('Alpha Project')).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /add alpha project to favorites/i }));
    await user.click(screen.getByRole('checkbox', { name: /favorites only/i }));

    expect(screen.getByText('Alpha Project')).toBeInTheDocument();
    expect(screen.queryByText('Beta Suite')).not.toBeInTheDocument();
  });

  it('filters projects by search text', async () => {
    const user = userEvent.setup();
    renderPage('/knowledge');

    await waitFor(() => {
      expect(screen.getByText('Alpha Project')).toBeInTheDocument();
    });

    await user.type(screen.getByPlaceholderText(/search projects by name or tag/i), 'beta suite');
    expect(screen.queryByText('Alpha Project')).not.toBeInTheDocument();
    expect(screen.getByText('Beta Suite')).toBeInTheDocument();
  });

  it('sorts projects by selected sort option', async () => {
    const user = userEvent.setup();
    renderPage('/knowledge');

    await waitFor(() => {
      expect(screen.getByText('Alpha Project')).toBeInTheDocument();
    });

    await user.selectOptions(screen.getByRole('combobox', { name: /sort projects/i }), 'knowledge_desc');

    const headings = screen.getAllByRole('heading', { level: 2 }).map(el => el.textContent);
    expect(headings).toEqual(['All Knowledge', 'Beta Suite', 'Alpha Project']);
  });

  it('persists favorites across remounts', async () => {
    const user = userEvent.setup();
    const { unmount } = render(
      <MemoryRouter initialEntries={['/knowledge']}>
        <Routes>
          <Route path="/knowledge" element={<KnowledgeLibraryPage />} />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText('Alpha Project')).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /add alpha project to favorites/i }));
    unmount();

    render(
      <MemoryRouter initialEntries={['/knowledge']}>
        <Routes>
          <Route path="/knowledge" element={<KnowledgeLibraryPage />} />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /remove alpha project from favorites/i })).toBeInTheDocument();
    });
  });

  it('keeps /knowledge/all as the global table view', async () => {
    renderPage('/knowledge/all');

    await waitFor(() => {
      expect(mockGetBlueprint).toHaveBeenCalled();
    });

    expect(screen.getByTestId('node-list-panel')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /back to project chooser/i })).toBeInTheDocument();
    expect(screen.queryByPlaceholderText(/search projects by name or tag/i)).not.toBeInTheDocument();
  });

  it('shows persistent project scope controls on project routes', async () => {
    renderPage('/knowledge/projects/proj-alpha');

    await waitFor(() => {
      expect(mockGetBlueprint).toHaveBeenCalled();
    });

    expect(screen.getByText(/project: alpha project/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /clear filters/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /reset to project scope/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /archive selected knowledge/i })).toBeDisabled();
    expect(screen.getByRole('button', { name: /^overview$/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^inventory$/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^architecture$/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^quality$/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^activity$/i })).toBeInTheDocument();
    expect(screen.getByTestId('node-list-panel')).toBeInTheDocument();
  });

  it('shows project activity panel when the activity section is selected', async () => {
    const user = userEvent.setup();
    renderPage('/knowledge/projects/proj-alpha');

    await waitFor(() => {
      expect(screen.getByText(/project: alpha project/i)).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /activity/i }));

    expect(screen.getByText(/action history/i)).toBeInTheDocument();
    expect(screen.getByText(/review queue/i)).toBeInTheDocument();
    expect(screen.getByText(/recent node changes/i)).toBeInTheDocument();
  });

  it('supports contextual deep links with project + secondary scope filters', async () => {
    renderPage('/knowledge?project=proj-alpha&feature=tasking&widget=tracker&artifact=task-service&component=task-widget');

    await waitFor(() => {
      expect(screen.getByText(/project: alpha project/i)).toBeInTheDocument();
    });

    expect(screen.getByText(/feature: tasking/i)).toBeInTheDocument();
    expect(screen.getByText(/widget: tracker/i)).toBeInTheDocument();
    expect(screen.getByText(/artifact: task-service/i)).toBeInTheDocument();
    expect(screen.getByText(/component: task-widget/i)).toBeInTheDocument();
    expect(screen.getByTestId('node-list-panel')).toBeInTheDocument();
  });

  it('shows back-navigation to the originating surface when provided', async () => {
    renderPage('/knowledge/projects/proj-alpha?from=%2Fblueprint&from_label=Blueprint');

    await waitFor(() => {
      expect(screen.getByText(/project: alpha project/i)).toBeInTheDocument();
    });

    const link = screen.getByRole('link', { name: /back to blueprint/i });
    expect(link).toHaveAttribute('href', '/blueprint');
  });

  it('persists scoped chips inside the same project context', async () => {
    const user = userEvent.setup();
    const { unmount } = render(
      <MemoryRouter initialEntries={['/knowledge/projects/proj-alpha']}>
        <Routes>
          <Route path="/knowledge/projects/:projectId" element={<KnowledgeLibraryPage />} />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText(/project: alpha project/i)).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /with docs/i }));
    expect(screen.getByRole('button', { name: /with docs/i })).toHaveAttribute('aria-pressed', 'true');
    unmount();

    render(
      <MemoryRouter initialEntries={['/knowledge/projects/proj-alpha']}>
        <Routes>
          <Route path="/knowledge/projects/:projectId" element={<KnowledgeLibraryPage />} />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /with docs/i })).toHaveAttribute('aria-pressed', 'true');
    });
  });
});
