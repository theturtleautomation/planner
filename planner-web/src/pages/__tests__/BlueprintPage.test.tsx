import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import BlueprintPage from '../BlueprintPage.tsx';
import type { BlueprintResponse, NodeSummary } from '../../types/blueprint.ts';

const mockGetBlueprint = vi.fn();

vi.mock('../../api/client.ts', () => ({
  createApiClient: vi.fn(() => ({
    getBlueprint: mockGetBlueprint,
  })),
}));

vi.mock('../../auth/useAuthenticatedFetch.ts', () => ({
  useGetAccessToken: vi.fn(() => vi.fn().mockResolvedValue('mock-token')),
}));

vi.mock('../../components/Layout.tsx', () => ({
  default: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

vi.mock('../../components/BlueprintGraph.tsx', () => ({
  default: ({ onSelectNode }: { onSelectNode: (nodeId: string | null) => void }) => (
    <button type="button" onClick={() => onSelectNode('node-task')}>Select task node</button>
  ),
}));

vi.mock('../../components/TableView.tsx', () => ({
  default: () => null,
}));

vi.mock('../../components/RadarView.tsx', () => ({
  default: () => null,
}));

vi.mock('../../components/DetailDrawer.tsx', () => ({
  default: () => null,
}));

vi.mock('../../components/ImpactPreviewModal.tsx', () => ({
  default: () => null,
}));

vi.mock('../../components/CreateNodeModal.tsx', () => ({
  default: () => null,
}));

vi.mock('../../components/DeleteNodeDialog.tsx', () => ({
  default: () => null,
}));

vi.mock('../../components/AddEdgeModal.tsx', () => ({
  default: () => null,
}));

vi.mock('../../components/ReconvergencePanel.tsx', () => ({
  default: () => null,
}));

function makeNode(overrides: Partial<NodeSummary>): NodeSummary {
  return {
    id: 'node-task',
    name: 'Task Tracker Widget',
    node_type: 'component',
    status: 'shipped',
    scope_class: 'project_contextual',
    scope_visibility: 'project_local',
    is_shared: false,
    project_id: 'proj-task-tracker',
    project_name: 'Task Tracker',
    secondary_scope: {
      feature: 'task_management',
      widget: 'task-tracker',
      artifact: 'widgets/task-tracker.tsx',
      component: 'task-widget',
    },
    linked_project_ids: [],
    tags: ['widget'],
    has_documentation: true,
    updated_at: '2026-03-07T00:00:00Z',
    ...overrides,
  };
}

function makeBlueprint(): BlueprintResponse {
  return {
    nodes: [makeNode({})],
    edges: [],
    counts: {
      component: 1,
    },
    total_nodes: 1,
    total_edges: 0,
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

describe('BlueprintPage Phase 4 contextual links', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetBlueprint.mockResolvedValue(makeBlueprint());
  });

  it('opens project-scoped related knowledge from selected task-tracker widget context', async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={['/blueprint']}>
        <Routes>
          <Route path="/blueprint" element={<BlueprintPage />} />
          <Route path="/knowledge/projects/:projectId" element={<LocationSnapshot />} />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(mockGetBlueprint).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByRole('button', { name: /select task node/i }));
    await user.click(screen.getByRole('button', { name: /view related knowledge/i }));

    expect(await screen.findByTestId('location-path')).toHaveTextContent('/knowledge/projects/proj-task-tracker');
    const params = JSON.parse(screen.getByTestId('location-params').textContent ?? '{}') as Record<string, string>;
    expect(params.project).toBe('proj-task-tracker');
    expect(params.feature).toBe('task_management');
    expect(params.widget).toBe('task-tracker');
    expect(params.artifact).toBe('widgets/task-tracker.tsx');
    expect(params.component).toBe('task-widget');
    expect(params.from).toBe('/blueprint');
    expect(params.from_label).toBe('Blueprint');
  });
});
