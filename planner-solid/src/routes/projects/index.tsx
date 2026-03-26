import { Title } from "@solidjs/meta";
import { A, action, useAction } from "@solidjs/router";
import { For, Show, createMemo, createResource, createSignal } from "solid-js";

import { ConfirmDialog } from "~/components/ui/ConfirmDialog";
import { StatusBadge } from "~/components/ui/StatusBadge";
import { deleteProject, listProjects, listSessions } from "~/lib/api";
import { buildProjectWorkSummaries } from "~/lib/projects";
import type { Project } from "~/lib/types";

const deleteProjectAction = action(async (projectSlug: string) => deleteProject(projectSlug), {
  name: "project-delete",
});

function loadProjectsDirectory() {
  return Promise.all([listProjects(), listSessions()]).then(([projects, sessions]) => ({
    projects: projects.projects,
    sessions: sessions.sessions,
  }));
}

export default function ProjectsPage() {
  const [data, { refetch }] = createResource(loadProjectsDirectory);
  const [projectPendingDelete, setProjectPendingDelete] = createSignal<Project | null>(null);
  const [deletePendingSlug, setDeletePendingSlug] = createSignal<string | null>(null);
  const [deleteError, setDeleteError] = createSignal<string | null>(null);
  const runDeleteProject = useAction(deleteProjectAction);
  const summaries = createMemo(() => {
    const current = data();
    return current ? buildProjectWorkSummaries(current.projects, current.sessions) : [];
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
      await runDeleteProject(project.slug);
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
      <Title>Projects</Title>
      <div class="stack page-frame">
        <section class="section-panel page-intro-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Projects</div>
              <h1 class="page-title">Active work directory</h1>
              <p class="page-copy">
                Open the project container, not a maze of route families. Active analysis rises
                first. Quiet projects stay compact.
              </p>
            </div>
            <div class="page-actions">
              <A class="btn btn-primary" href="/projects/new">
                New project
              </A>
            </div>
          </div>
          <Show when={deleteError()}>
            {message => <div class="error-copy">{message()}</div>}
          </Show>

          <Show
            when={data()}
            fallback={<div class="empty-state">Loading projects…</div>}
          >
            <Show
              when={summaries().length > 0}
              fallback={<div class="empty-state">No projects exist yet.</div>}
            >
              <div class="project-list">
                <For each={summaries()}>
                  {summary => (
                    <div class="project-row">
                      <A
                        aria-label={summary.project.name}
                        class="project-row-link"
                        href={`/projects/${summary.project.slug}`}
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
                          {deletePendingSlug() === summary.project.slug
                            ? "Deleting…"
                            : "Delete"}
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
