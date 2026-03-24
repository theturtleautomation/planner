import { Title } from "@solidjs/meta";
import { A } from "@solidjs/router";
import { For, Show, createMemo, createResource } from "solid-js";

import { listProjects, listSessions } from "~/lib/api";
import { buildProjectWorkSummaries } from "~/lib/projects";

function loadProjectsDirectory() {
  return Promise.all([listProjects(), listSessions()]).then(([projects, sessions]) => ({
    projects: projects.projects,
    sessions: sessions.sessions,
  }));
}

export default function ProjectsPage() {
  const [data] = createResource(loadProjectsDirectory);
  const summaries = createMemo(() => {
    const current = data();
    return current ? buildProjectWorkSummaries(current.projects, current.sessions) : [];
  });

  return (
    <section class="page page-scroll">
      <Title>Projects</Title>
      <div class="stack page-frame">
        <section class="section-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Projects</div>
              <h1 class="section-title">Active work directory</h1>
              <p class="section-copy">
                Open the project container, not a maze of route families. Active analysis rises
                first. Quiet projects stay compact.
              </p>
            </div>
            <A class="btn btn-primary" href="/projects/new">
              New project
            </A>
          </div>

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
                    <A class="project-row" href={`/projects/${summary.project.slug}`}>
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
