import { Title } from "@solidjs/meta";
import { A } from "@solidjs/router";
import { For, Show, createMemo, createResource, createSignal } from "solid-js";

import { deleteProject, listProjects, listSessions } from "~/lib/api";
import { buildProjectWorkSummaries } from "~/lib/projects";
import type { Project } from "~/lib/types";

function loadProjectsDirectory() {
  return Promise.all([listProjects(), listSessions()]).then(([projects, sessions]) => ({
    projects: projects.projects,
    sessions: sessions.sessions,
  }));
}

export default function ProjectsPage() {
  const [data, { refetch }] = createResource(loadProjectsDirectory);
  const [deletingProjectId, setDeletingProjectId] = createSignal<string | null>(null);
  const [deleteError, setDeleteError] = createSignal<string | null>(null);
  const summaries = createMemo(() => {
    const current = data();
    return current ? buildProjectWorkSummaries(current.projects, current.sessions) : [];
  });

  const handleDeleteProject = async (project: Project) => {
    const confirmed = window.confirm(
      `Delete "${project.name}" permanently?\n\nThis will stop any active sessions, remove this project's sessions and owned knowledge, and preserve shared knowledge by unlinking it from this project. This action cannot be undone.`,
    );
    if (!confirmed) return;

    setDeletingProjectId(project.id);
    setDeleteError(null);
    try {
      await deleteProject(project.slug);
      await refetch();
    } catch (error) {
      setDeleteError(error instanceof Error ? error.message : "Failed to delete project.");
    } finally {
      setDeletingProjectId(null);
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
                          <span class={`state-badge is-${summary.status}`}>{summary.statusLabel}</span>
                          <span>{summary.sessionCount} sessions</span>
                          <span>{summary.nextActionLabel}</span>
                        </div>
                      </A>
                      <div class="project-row-actions">
                        <button
                          aria-label={`Delete ${summary.project.name}`}
                          class="btn btn-subtle btn-danger"
                          disabled={deletingProjectId() === summary.project.id}
                          type="button"
                          onClick={() => void handleDeleteProject(summary.project)}
                        >
                          {deletingProjectId() === summary.project.id ? "Deleting…" : "Delete"}
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
    </section>
  );
}
