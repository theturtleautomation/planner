import { Title } from "@solidjs/meta";
import { A, useNavigate, useSearchParams } from "@solidjs/router";
import { For, Match, Show, Switch, createEffect, createMemo, createResource, createSignal } from "solid-js";
import { isServer } from "solid-js/web";

import { ProjectCreateForm } from "~/components/projects/ProjectCreateForm";
import { ConfirmDialog } from "~/components/ui/ConfirmDialog";
import { StatusBadge } from "~/components/ui/StatusBadge";
import { createProject, deleteProject, listProjects, listSessions } from "~/lib/api";
import { isFrontendMockEnabled, withFrontendMockSearch } from "~/lib/mock/runtime";
import { createMockProject, getMockProject } from "~/lib/mock/store";
import { buildProjectWorkSummaries } from "~/lib/projects";
import type { Project } from "~/lib/types";

const PENDING_MOCK_PROJECT_STORAGE_KEY = "planner.frontend-mock.pending-project";

function loadWorkEntry() {
  return Promise.all([listProjects(), listSessions()]).then(([projects, sessions]) => ({
    projects: projects.projects,
    sessions: sessions.sessions,
  }));
}

function firstParam(value: string | string[] | undefined): string {
  return Array.isArray(value) ? value[0] ?? "" : value ?? "";
}

function homeFallbackSlug(input: string): string {
  return (
    input
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 48) || "project"
  );
}

export default function HomePage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const [data, { refetch }] = createResource(loadWorkEntry);
  const [projectPendingDelete, setProjectPendingDelete] = createSignal<Project | null>(null);
  const [deletePendingSlug, setDeletePendingSlug] = createSignal<string | null>(null);
  const [deleteError, setDeleteError] = createSignal<string | null>(null);
  const [autoCreateError, setAutoCreateError] = createSignal<string | null>(null);
  const [autoCreating, setAutoCreating] = createSignal(false);
  const [autoCreateStarted, setAutoCreateStarted] = createSignal(false);
  const initialName = createMemo(() => firstParam(searchParams.name).trim());
  const initialDescription = createMemo(() => firstParam(searchParams.description).trim());
  const summaries = createMemo(() => {
    const current = data();
    return current ? buildProjectWorkSummaries(current.projects, current.sessions) : [];
  });

  if (isServer && isFrontendMockEnabled() && initialName()) {
    const slug = homeFallbackSlug(initialName());
    const response = (() => {
      try {
        return getMockProject(slug);
      } catch {
        return createMockProject({
          name: initialName(),
          description: initialDescription() || null,
          slug,
        });
      }
    })();
    const target = withFrontendMockSearch(`/projects/${response.project.slug}`);

    return (
      <section class="page page-scroll">
        <Title>Creating Project</Title>
        <div class="stack page-frame">
          <section class="section-panel form-panel page-intro-panel">
            <div class="empty-state">Creating project…</div>
            <script>
              {`window.sessionStorage.setItem(${JSON.stringify(PENDING_MOCK_PROJECT_STORAGE_KEY)}, ${JSON.stringify(
                JSON.stringify({
                  name: response.project.name,
                  description: response.project.description,
                  slug: response.project.slug,
                }),
              )}); window.location.replace(${JSON.stringify(target)});`}
            </script>
          </section>
        </div>
      </section>
    );
  }

  createEffect(() => {
    if (!initialName() || autoCreateStarted()) return;
    if (isFrontendMockEnabled()) return;

    setAutoCreateStarted(true);
    setAutoCreating(true);
    setAutoCreateError(null);

    void createProject({
      name: initialName(),
      description: initialDescription() || null,
    })
      .then(response => {
        navigate(withFrontendMockSearch(`/projects/${response.project.slug}`), {
          replace: true,
        });
      })
      .catch(error => {
        setAutoCreateError(error instanceof Error ? error.message : "Unable to create project.");
        setAutoCreating(false);
      });
  });

  const openDeleteDialog = (project: Project) => {
    setDeleteError(null);
    setProjectPendingDelete(project);
  };

  const handleDeleteDialogOpenChange = (open: boolean) => {
    if (open || !!deletePendingSlug()) return;
    setProjectPendingDelete(null);
  };

  const handleDeleteProject = async () => {
    const project = projectPendingDelete();
    if (!project) return;

    setDeleteError(null);
    setDeletePendingSlug(project.slug);
    try {
      await deleteProject(project.slug);
      await refetch();
      setProjectPendingDelete(null);
    } catch (error) {
      setDeleteError(error instanceof Error ? error.message : "Failed to delete project.");
    } finally {
      setDeletePendingSlug(null);
    }
  };

  return (
    <section class="page page-scroll">
      <Title>Planner Home</Title>
      <div class="stack page-frame">
        <section class="section-panel home-entry-panel">
          <div class="home-entry-composer-shell">
            <div class="home-entry-composer-copy">
              <div class="eyebrow">Home</div>
              <p class="home-entry-copy">
                Start a new project immediately, then reopen existing work from the same shared
                directory below.
              </p>
            </div>
            <Switch>
              <Match when={autoCreating()}>
                <div class="empty-state">Creating project…</div>
              </Match>
              <Match when={autoCreateError()}>
                {message => (
                  <div class="stack">
                    <div class="error-copy">{message()}</div>
                    <ProjectCreateForm
                      class="inline-form home-entry-form"
                      initialDescription={firstParam(searchParams.description)}
                      initialName={firstParam(searchParams.name)}
                      titlePlaceholder="Project title"
                      descriptionPlaceholder="What are you shaping, testing, or trying to make real?"
                      secondaryAction={
                        <A class="btn btn-subtle" href={withFrontendMockSearch("/sessions/new")}>
                          Direct session
                        </A>
                      }
                    />
                  </div>
                )}
              </Match>
              <Match when={true}>
                <ProjectCreateForm
                  class="inline-form home-entry-form"
                  titlePlaceholder="Project title"
                  descriptionPlaceholder="What are you shaping, testing, or trying to make real?"
                  secondaryAction={
                    <A class="btn btn-subtle" href={withFrontendMockSearch("/sessions/new")}>
                      Direct session
                    </A>
                  }
                />
              </Match>
            </Switch>
          </div>
        </section>

        <section class="section-panel page-intro-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Projects</div>
              <h2 class="group-title">Current work</h2>
            </div>
          </div>
          <Show when={deleteError()}>
            {message => <div class="error-copy">{message()}</div>}
          </Show>

          <Show when={data()} fallback={<div class="empty-state">Loading projects…</div>}>
            <Show
              when={summaries().length > 0}
              fallback={
                <div class="empty-state">
                  No projects yet. Create one above and start a Socratic analysis.
                </div>
              }
            >
              <div class="project-list">
                <For each={summaries()}>
                  {summary => (
                    <div class="project-row">
                      <A
                        aria-label={summary.project.name}
                        class="project-row-link"
                        href={withFrontendMockSearch(`/projects/${summary.project.slug}`)}
                      >
                        <div class="project-row-main">
                          <div class="project-row-title">{summary.project.name}</div>
                          <div class="project-row-copy">
                            {summary.primarySession?.project_description?.trim() ||
                              summary.project.description?.trim() ||
                              "Ready to start a new Socratic analysis."}
                          </div>
                        </div>
                        <div class="project-row-facts">
                          <StatusBadge tone={summary.status}>{summary.statusLabel}</StatusBadge>
                          <span>{summary.sessionCount} sessions</span>
                          <span>{summary.nextActionLabel}</span>
                        </div>
                      </A>
                      <div class="project-row-actions">
                        <button
                          aria-label={`Delete ${summary.project.name}`}
                          class="btn btn-subtle btn-danger"
                          disabled={deletePendingSlug() === summary.project.slug}
                          type="button"
                          onClick={() => openDeleteDialog(summary.project)}
                        >
                          {deletePendingSlug() === summary.project.slug ? "Deleting…" : "Delete"}
                        </button>
                      </div>
                    </div>
                  )}
                </For>
              </div>
            </Show>
          </Show>
        </section>
      </div>
      <ConfirmDialog
        confirmLabel="Delete project"
        description={
          projectPendingDelete()
            ? `Delete "${projectPendingDelete()!.name}" permanently? This will stop any active sessions, remove this project's sessions and owned knowledge, and preserve shared knowledge by unlinking it from this project. This action cannot be undone.`
            : ""
        }
        error={deleteError()}
        open={!!projectPendingDelete()}
        pending={deletePendingSlug() === projectPendingDelete()?.slug}
        title="Delete project permanently"
        onConfirm={handleDeleteProject}
        onOpenChange={handleDeleteDialogOpenChange}
      />
    </section>
  );
}
