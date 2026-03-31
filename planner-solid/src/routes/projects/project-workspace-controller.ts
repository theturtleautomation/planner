import { useNavigate, useParams, useSearchParams } from "@solidjs/router";
import {
  createMemo,
  createResource,
  createSignal,
  type Accessor,
} from "solid-js";

import {
  applyProjectImportReview,
  createProjectSession,
  getProject,
  getProjectBlueprint,
  getProjectImportReview,
  getProjectImportState,
  getPromptBank,
  getSessionEvents,
  getSessionRuns,
  listBlueprintExportHistory,
  listSessions,
  reimportProject,
  updateProjectImportReviewSelection,
} from "~/lib/api";
import type {
  BlueprintSummary,
  BuildExecutionSummary,
  BuildPathSummary,
  BuildReadinessSummary,
  KnowledgeSummary,
  OutputArtifactSummary,
  ProjectActivitySummary,
  ReviewSummary,
} from "~/lib/advanced";
import {
  summarizeBlueprint,
  summarizeBuildExecution,
  summarizeBuildPath,
  summarizeBuildReadiness,
  summarizeKnowledge,
  summarizeOutputArtifacts,
  summarizeProjectActivity,
  summarizeReview,
} from "~/lib/advanced";
import type { ProjectWorkSummary } from "~/lib/projects";
import { summarizeProjectWork } from "~/lib/projects";
import {
  readinessToneForState,
  type ProjectSurfaceTab,
} from "~/lib/project-surface";
import { withFrontendMockSearch } from "~/lib/mock/runtime";
import type {
  ProjectImportResponse,
  ProjectResponse,
  PromptBankResponse,
  SessionEventsResponse,
  SessionSummary,
} from "~/lib/types";
import {
  filterProjectSessionsBySlug,
  resolveProjectWorkspaceSurfaceState,
} from "./project-workspace-route-state";

export interface ProjectWorkspaceController {
  project: Accessor<ProjectResponse | undefined>;
  projectSessions: Accessor<SessionSummary[]>;
  summary: Accessor<ProjectWorkSummary | null>;
  activeSession: Accessor<SessionSummary | null>;
  promptBank: Accessor<PromptBankResponse | null | undefined>;
  importReview: Accessor<ProjectImportResponse | null | undefined>;
  importState: Accessor<ProjectImportResponse | null | undefined>;
  reviewSummary: Accessor<ReviewSummary>;
  buildReadiness: Accessor<BuildReadinessSummary>;
  buildPath: Accessor<BuildPathSummary>;
  activitySummary: Accessor<ProjectActivitySummary>;
  buildExecution: Accessor<BuildExecutionSummary>;
  outputArtifacts: Accessor<OutputArtifactSummary>;
  knowledgeSummary: Accessor<KnowledgeSummary | null>;
  blueprintSummary: Accessor<BlueprintSummary | null>;
  readinessTone: Accessor<ReturnType<typeof readinessToneForState>>;
  advancedOpen: Accessor<boolean>;
  advancedTab: Accessor<ProjectSurfaceTab>;
  reviewLoading: Accessor<boolean>;
  readinessLoading: Accessor<boolean>;
  buildLoading: Accessor<boolean>;
  activityLoading: Accessor<boolean>;
  executionLoading: Accessor<boolean>;
  outputsLoading: Accessor<boolean>;
  starting: Accessor<boolean>;
  error: Accessor<string | null>;
  reviewError: Accessor<string | null>;
  applyPending: Accessor<boolean>;
  reimportPending: Accessor<boolean>;
  selectionPendingNodeId: Accessor<string | null>;
  openProjectSurfaces: (tab?: ProjectSurfaceTab) => void;
  closeProjectSurfaces: () => void;
  handleProjectSurfaceTabChange: (tab: ProjectSurfaceTab) => void;
  handleStartAnalysis: () => Promise<void>;
  handleSetImportNodeIncluded: (nodeId: string, included: boolean) => Promise<void>;
  handleApplyImportReview: () => Promise<void>;
  handleReimport: () => Promise<void>;
}

export function useProjectWorkspaceController(): ProjectWorkspaceController {
  const params = useParams();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams<{ tab?: string }>();

  const [project] = createResource(() => params.projectSlug, getProject);
  const [sessions] = createResource(listSessions);
  const [starting, setStarting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [reviewError, setReviewError] = createSignal<string | null>(null);
  const [applyPending, setApplyPending] = createSignal(false);
  const [reimportPending, setReimportPending] = createSignal(false);
  const [selectionPendingNodeId, setSelectionPendingNodeId] = createSignal<string | null>(null);

  const projectSessions = createMemo(() =>
    filterProjectSessionsBySlug(sessions(), params.projectSlug),
  );
  const summary = createMemo(() => {
    const currentProject = project()?.project;
    if (!currentProject) return null;
    return summarizeProjectWork(currentProject, projectSessions());
  });
  const activeSession = createMemo(() => summary()?.primarySession ?? null);
  const currentProjectId = createMemo(() => project()?.project.id);
  const surfaceState = createMemo(() =>
    resolveProjectWorkspaceSurfaceState(searchParams.tab),
  );
  const advancedOpen = createMemo(() => surfaceState().advancedOpen);
  const advancedTab = createMemo(() => surfaceState().advancedTab);

  const [projectBlueprint] = createResource(
    () => (advancedOpen() ? params.projectSlug : undefined),
    async projectSlug => {
      if (!projectSlug) return null;
      return getProjectBlueprint(projectSlug, {
        includeShared: true,
        includeGlobal: false,
      });
    },
  );
  const [projectImportState, { refetch: refetchImportState }] = createResource(
    () => (advancedOpen() ? params.projectSlug : undefined),
    async projectSlug => (projectSlug ? getProjectImportState(projectSlug) : null),
  );
  const [projectImportReview, { refetch: refetchImportReview }] = createResource(
    () => (advancedOpen() ? params.projectSlug : undefined),
    async projectSlug => (projectSlug ? getProjectImportReview(projectSlug) : null),
  );
  const [promptBank, { refetch: refetchPromptBank }] = createResource(
    () => (advancedOpen() && activeSession()?.id ? activeSession()!.id : undefined),
    async sessionId => (sessionId ? getPromptBank(sessionId) : null),
  );
  const [sessionRuns] = createResource(
    () => (advancedOpen() && activeSession()?.id ? activeSession()!.id : undefined),
    async sessionId => (sessionId ? getSessionRuns(sessionId) : null),
  );
  const [sessionEvents] = createResource(
    () => (advancedOpen() && activeSession()?.id ? activeSession()!.id : undefined),
    async sessionId =>
      sessionId
        ? getSessionEvents(sessionId, {
            source: "pipeline",
            limit: 5,
          })
        : null,
  );
  const [exportHistory] = createResource(
    () => (advancedOpen() ? currentProjectId() : undefined),
    async projectId => (projectId ? listBlueprintExportHistory({ projectId, limit: 6 }) : null),
  );

  const knowledgeSummary = createMemo(() => {
    const blueprint = projectBlueprint();
    return blueprint ? summarizeKnowledge(blueprint) : null;
  });
  const blueprintSummary = createMemo(() => {
    const blueprint = projectBlueprint();
    return blueprint ? summarizeBlueprint(blueprint) : null;
  });
  const reviewSummary = createMemo(() =>
    summarizeReview({
      importReview: projectImportReview(),
      importState: projectImportState(),
      promptBank: promptBank(),
    }),
  );
  const buildReadiness = createMemo(() =>
    summarizeBuildReadiness({
      primarySession: activeSession(),
      promptBank: promptBank(),
      importState: projectImportState(),
      importReview: projectImportReview(),
      blueprintSummary: blueprintSummary(),
    }),
  );
  const buildPath = createMemo(() =>
    summarizeBuildPath({
      projectName: project()?.project.name ?? "Project",
      primarySession: activeSession(),
      readiness: buildReadiness(),
      promptBank: promptBank(),
      blueprintSummary: blueprintSummary(),
    }),
  );
  const activitySummary = createMemo(() =>
    summarizeProjectActivity({
      sessions: projectSessions(),
      importState: projectImportState(),
      promptBank: promptBank(),
      buildPath: buildPath(),
    }),
  );
  const buildExecution = createMemo(() =>
    summarizeBuildExecution({
      primarySession: activeSession(),
      runs: sessionRuns(),
      events: (sessionEvents() as SessionEventsResponse | null)?.events ?? [],
    }),
  );
  const outputArtifacts = createMemo(() =>
    summarizeOutputArtifacts({
      projectName: project()?.project.name ?? "Project",
      history: exportHistory(),
    }),
  );
  const reviewLoading = createMemo(
    () => projectImportReview.loading || projectImportState.loading || promptBank.loading,
  );
  const readinessLoading = createMemo(
    () => projectImportReview.loading || projectImportState.loading || promptBank.loading,
  );
  const buildLoading = createMemo(
    () => projectImportReview.loading || projectImportState.loading || promptBank.loading,
  );
  const activityLoading = createMemo(
    () => projectImportReview.loading || projectImportState.loading || promptBank.loading,
  );
  const executionLoading = createMemo(() => sessionRuns.loading || sessionEvents.loading);
  const outputsLoading = createMemo(() => exportHistory.loading);
  const readinessTone = createMemo(() => readinessToneForState(buildReadiness().state));

  const openProjectSurfaces = (tab: ProjectSurfaceTab = "review") => {
    setSearchParams({ tab });
  };

  const closeProjectSurfaces = () => {
    setSearchParams({ tab: undefined });
  };

  const handleProjectSurfaceTabChange = (tab: ProjectSurfaceTab) => {
    setSearchParams({ tab });
  };

  const handleStartAnalysis = async () => {
    const currentProject = project()?.project;
    if (!currentProject) return;
    setStarting(true);
    setError(null);

    try {
      const response = await createProjectSession(currentProject.slug, {
        title: `${currentProject.name} analysis`,
        description: currentProject.description ?? null,
      });
      navigate(withFrontendMockSearch(`/sessions/${response.session.id}`));
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unable to start a new project analysis.");
      setStarting(false);
    }
  };

  const handleSetImportNodeIncluded = async (nodeId: string, included: boolean) => {
    if (!params.projectSlug) return;
    setSelectionPendingNodeId(nodeId);
    setReviewError(null);
    try {
      await updateProjectImportReviewSelection(params.projectSlug, { nodeId, included });
      await Promise.all([refetchImportReview(), refetchImportState()]);
    } catch (err) {
      setReviewError(err instanceof Error ? err.message : "Unable to update review selection.");
    } finally {
      setSelectionPendingNodeId(null);
    }
  };

  const handleApplyImportReview = async () => {
    if (!params.projectSlug) return;
    setApplyPending(true);
    setReviewError(null);
    try {
      await applyProjectImportReview(params.projectSlug);
      await Promise.all([refetchImportReview(), refetchImportState(), refetchPromptBank()]);
    } catch (err) {
      setReviewError(err instanceof Error ? err.message : "Unable to apply the import review.");
    } finally {
      setApplyPending(false);
    }
  };

  const handleReimport = async () => {
    if (!params.projectSlug) return;
    setReimportPending(true);
    setReviewError(null);
    try {
      await reimportProject(params.projectSlug);
      await Promise.all([refetchImportReview(), refetchImportState(), refetchPromptBank()]);
      openProjectSurfaces("review");
    } catch (err) {
      setReviewError(err instanceof Error ? err.message : "Unable to start a re-import.");
    } finally {
      setReimportPending(false);
    }
  };

  return {
    project,
    projectSessions,
    summary,
    activeSession,
    promptBank,
    importReview: projectImportReview,
    importState: projectImportState,
    reviewSummary,
    buildReadiness,
    buildPath,
    activitySummary,
    buildExecution,
    outputArtifacts,
    knowledgeSummary,
    blueprintSummary,
    readinessTone,
    advancedOpen,
    advancedTab,
    reviewLoading,
    readinessLoading,
    buildLoading,
    activityLoading,
    executionLoading,
    outputsLoading,
    starting,
    error,
    reviewError,
    applyPending,
    reimportPending,
    selectionPendingNodeId,
    openProjectSurfaces,
    closeProjectSurfaces,
    handleProjectSurfaceTabChange,
    handleStartAnalysis,
    handleSetImportNodeIncluded,
    handleApplyImportReview,
    handleReimport,
  };
}
