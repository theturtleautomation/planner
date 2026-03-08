import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import KnowledgeLibraryPage from '../KnowledgeLibraryPage.tsx';
import type { BlueprintEventPayload, BlueprintNode, BlueprintResponse, NodeSummary } from '../../types/blueprint.ts';

const mockGetBlueprint = vi.fn();
const mockGetBlueprintNode = vi.fn();
const mockListBlueprintEvents = vi.fn();
const mockRecordBlueprintExport = vi.fn();
const mockUpdateBlueprintNode = vi.fn();
const mockCreateBlueprintNode = vi.fn();
const mockGetAccessToken = vi.fn().mockResolvedValue('mock-token');

vi.mock('../../api/client.ts', () => ({
  createApiClient: vi.fn(() => ({
    getBlueprint: mockGetBlueprint,
    getBlueprintNode: mockGetBlueprintNode,
    listBlueprintEvents: mockListBlueprintEvents,
    recordBlueprintExport: mockRecordBlueprintExport,
    updateBlueprintNode: mockUpdateBlueprintNode,
    createBlueprintNode: mockCreateBlueprintNode,
    deleteBlueprintNode: vi.fn(),
  })),
}));

vi.mock('../../auth/useAuthenticatedFetch.ts', () => ({
  useGetAccessToken: vi.fn(() => mockGetAccessToken),
}));

vi.mock('../../components/Layout.tsx', () => ({
  default: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

vi.mock('../../components/NodeListPanel.tsx', () => ({
  default: ({
    nodes,
    onSelectNode,
    onToggleSelectNode,
    selectedNodeIds = [],
  }: {
    nodes: NodeSummary[];
    onSelectNode: (nodeId: string) => void;
    onToggleSelectNode?: (nodeId: string, selected: boolean) => void;
    selectedNodeIds?: string[];
  }) => (
    <div data-testid="node-list-panel">
      {nodes.map(node => (
        <button
          key={node.id}
          type="button"
          onClick={() => {
            onSelectNode(node.id);
            onToggleSelectNode?.(node.id, !selectedNodeIds.includes(node.id));
          }}
        >
          Select {node.name}
        </button>
      ))}
    </div>
  ),
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
    lifecycle: 'active',
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
        tags: ['platform', 'api', 'owner:Alice', 'team:Platform'],
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

function makeArchivedBlueprint(): BlueprintResponse {
  const blueprint = makeBlueprint();
  return {
    ...blueprint,
    nodes: blueprint.nodes.map(node => (
      node.id === 'alpha-local'
        ? { ...node, lifecycle: 'archived' as const }
        : node
    )),
  };
}

function makeBlueprintWithUnscopedRecord(): BlueprintResponse {
  const blueprint = makeBlueprint();
  return {
    ...blueprint,
    nodes: [
      ...blueprint.nodes,
      makeNode({
        id: 'legacy-unscoped',
        name: 'Legacy Pattern',
        node_type: 'pattern',
        scope_class: 'unscoped',
        scope_visibility: 'unscoped',
        lifecycle: 'active',
        project_id: undefined,
        project_name: undefined,
        secondary_scope: {},
        linked_project_ids: [],
        tags: ['legacy'],
        updated_at: '2026-03-03T12:00:00Z',
      }),
    ],
    counts: {
      ...blueprint.counts,
      pattern: (blueprint.counts.pattern ?? 0) + 1,
    },
    total_nodes: blueprint.total_nodes + 1,
  };
}

function makeBlueprintNode(overrides: Partial<BlueprintNode> = {}): BlueprintNode {
  return {
    node_type: 'decision',
    id: 'alpha-local',
    title: 'Alpha Decision',
    status: 'accepted',
    context: 'Decision context',
    options: [],
    consequences: [],
    assumptions: [],
    documentation: 'https://docs.example.test/alpha',
    tags: ['platform', 'api', 'owner:Alice', 'team:Platform'],
    scope: {
      scope_class: 'project',
      project: {
        project_id: 'proj-alpha',
        project_name: 'Alpha Project',
      },
      secondary: {},
      is_shared: false,
      lifecycle: 'active',
    },
    created_at: '2026-03-01T12:00:00Z',
    updated_at: '2026-03-06T12:00:00Z',
    ...overrides,
  };
}

function makeProjectEvents(includeExport = false): BlueprintEventPayload[] {
  const events: BlueprintEventPayload[] = [
    {
      event_type: 'node_updated',
      summary: "Updated decision 'alpha-local'",
      timestamp: '2026-03-07T10:00:00Z',
      data: {
        node_id: 'alpha-local',
        before: makeBlueprintNode({ tags: ['platform', 'api', 'owner:Alice', 'team:Platform'] }),
        after: makeBlueprintNode({ tags: ['platform', 'api', 'owner:Alice', 'team:Platform', 'archived'] }),
      },
    },
  ];

  if (includeExport) {
    events.unshift({
      event_type: 'export_recorded',
      summary: 'Recorded scoped view export',
      timestamp: '2026-03-07T11:00:00Z',
      data: {
        kind: 'scoped_view',
        export_id: 'exp-knowledge',
        project_id: 'proj-alpha',
        project_name: 'Alpha Project',
        node_count: 3,
        edge_count: 1,
      },
    });
  }

  return events;
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
    mockGetAccessToken.mockResolvedValue('mock-token');
    window.localStorage.clear();
    mockGetBlueprint.mockResolvedValue(makeBlueprint());
    mockGetBlueprintNode.mockResolvedValue(makeBlueprintNode());
    mockUpdateBlueprintNode.mockResolvedValue(makeBlueprintNode());
    mockCreateBlueprintNode.mockResolvedValue({ id: 'created-node', message: 'ok' });
    mockListBlueprintEvents.mockResolvedValue({ events: makeProjectEvents(), total: 1 });
    mockRecordBlueprintExport.mockResolvedValue({
      export_id: 'exp-test',
      recorded_at: '2026-03-07T12:00:00Z',
    });
  });

  it('shows project cards first on /knowledge with explicit All Knowledge entry point', async () => {
    renderPage('/knowledge');

    await waitFor(() => {
      expect(mockGetBlueprint).toHaveBeenCalled();
    });

    expect(screen.getByText('Alpha Project')).toBeInTheDocument();
    expect(screen.getByText('Beta Suite')).toBeInTheDocument();
    expect(screen.getByText('All Knowledge')).toBeInTheDocument();
    expect(screen.getByText('Owner: Alice')).toBeInTheDocument();
    expect(screen.getByText('Team: Platform')).toBeInTheDocument();
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
    expect(screen.getByRole('button', { name: /clear all/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /more filters/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /reset to project scope/i })).toBeInTheDocument();
    const primaryFilterLabels = screen.getAllByRole('combobox').map(input => input.getAttribute('aria-label'));
    expect(primaryFilterLabels).toEqual([
      'Type',
      'Feature Area',
      'Surface',
      'Artifact',
      'Related Component',
    ]);
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

    expect(screen.getByText(/project event history/i)).toBeInTheDocument();
    expect(screen.getByText(/review queue/i)).toBeInTheDocument();
    expect(screen.getByText(/recent node changes/i)).toBeInTheDocument();
  });

  it('shows durable export history entries sourced from project events', async () => {
    const user = userEvent.setup();
    mockListBlueprintEvents.mockResolvedValueOnce({ events: makeProjectEvents(true), total: 2 });

    renderPage('/knowledge/projects/proj-alpha');

    await waitFor(() => {
      expect(screen.getByText(/project: alpha project/i)).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /activity/i }));

    expect(screen.getByText(/durable export history/i)).toBeInTheDocument();
    expect(screen.getByText(/exported scoped view \(3 records\)/i)).toBeInTheDocument();
  });

  it('archives selected knowledge via the lifecycle field', async () => {
    const user = userEvent.setup();
    renderPage('/knowledge/projects/proj-alpha');

    await waitFor(() => {
      expect(screen.getByText(/project: alpha project/i)).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /select alpha decision/i }));
    await user.click(screen.getByRole('button', { name: /archive selected knowledge/i }));

    await waitFor(() => {
      expect(mockUpdateBlueprintNode).toHaveBeenCalledWith(
        'alpha-local',
        expect.objectContaining({
          scope: expect.objectContaining({ lifecycle: 'archived' }),
        }),
      );
    });
  });

  it('restores archived knowledge via the lifecycle field', async () => {
    const user = userEvent.setup();
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    mockGetBlueprint.mockResolvedValue(makeArchivedBlueprint());

    renderPage('/knowledge/projects/proj-alpha');

    await waitFor(() => {
      expect(screen.getByText(/project: alpha project/i)).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /more filters/i }));
    await user.selectOptions(screen.getByRole('combobox', { name: /lifecycle/i }), 'archived');
    await user.click(screen.getByRole('button', { name: /select alpha decision/i }));
    await user.click(screen.getByRole('button', { name: /restore archived knowledge/i }));

    await waitFor(() => {
      expect(mockUpdateBlueprintNode).toHaveBeenCalledWith(
        'alpha-local',
        expect.objectContaining({
          scope: expect.objectContaining({ lifecycle: 'active' }),
        }),
      );
    });

    confirmSpy.mockRestore();
  });

  it('exports a single selected record from project scope', async () => {
    const user = userEvent.setup();
    const createObjectUrl = vi.spyOn(window.URL, 'createObjectURL').mockReturnValue('blob:knowledge-record');
    const revokeObjectUrl = vi.spyOn(window.URL, 'revokeObjectURL').mockImplementation(() => {});

    renderPage('/knowledge/projects/proj-alpha');

    await waitFor(() => {
      expect(screen.getByText(/project: alpha project/i)).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /select alpha decision/i }));
    await user.click(screen.getByRole('button', { name: /export selected record/i }));

    await waitFor(() => {
      expect(mockGetBlueprintNode).toHaveBeenCalledWith('alpha-local');
    });
    expect(createObjectUrl).toHaveBeenCalled();
    expect(revokeObjectUrl).toHaveBeenCalledWith('blob:knowledge-record');
  });

  it('supports contextual deep links with project + secondary scope filters', async () => {
    renderPage('/knowledge?project=proj-alpha&feature=tasking&widget=tracker&artifact=task-service&component=task-widget');

    await waitFor(() => {
      expect(screen.getByText(/project: alpha project/i)).toBeInTheDocument();
    });

    expect(screen.getByText(/feature area: tasking/i)).toBeInTheDocument();
    expect(screen.getByText(/surface: tracker/i)).toBeInTheDocument();
    expect(screen.getByText(/artifact: task-service/i)).toBeInTheDocument();
    expect(screen.getByText(/related component: task-widget/i)).toBeInTheDocument();
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

  it('persists scoped filters inside the same project context', async () => {
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

    await user.click(screen.getByRole('button', { name: /more filters/i }));
    await user.selectOptions(screen.getByRole('combobox', { name: /docs/i }), 'with_docs');
    expect(screen.getByRole('combobox', { name: /docs/i })).toHaveValue('with_docs');
    unmount();

    render(
      <MemoryRouter initialEntries={['/knowledge/projects/proj-alpha']}>
        <Routes>
          <Route path="/knowledge/projects/:projectId" element={<KnowledgeLibraryPage />} />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText(/project: alpha project/i)).toBeInTheDocument();
    });
    await user.click(screen.getByRole('button', { name: /more filters/i }));
    expect(screen.getByRole('combobox', { name: /docs/i })).toHaveValue('with_docs');
  });

  it('resolves unscoped records from the quality review workflow', async () => {
    const user = userEvent.setup();
    mockGetBlueprint.mockResolvedValue(makeBlueprintWithUnscopedRecord());

    renderPage('/knowledge/projects/proj-alpha');

    await waitFor(() => {
      expect(screen.getByText(/project: alpha project/i)).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /^quality$/i }));
    await user.click(screen.getByRole('button', { name: /assign to project/i }));

    await waitFor(() => {
      expect(mockUpdateBlueprintNode).toHaveBeenCalledWith(
        'legacy-unscoped',
        expect.objectContaining({
          scope: expect.objectContaining({
            scope_class: 'project',
            project: expect.objectContaining({
              project_id: 'proj-alpha',
            }),
          }),
        }),
      );
    });
  });
});
