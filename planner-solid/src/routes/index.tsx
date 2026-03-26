import { Title } from "@solidjs/meta";
import { A } from "@solidjs/router";
import { For, Show, createMemo, createResource } from "solid-js";

import { listProjects, listSessions } from "~/lib/api";
import { buildProjectWorkSummaries, selectGuidedEntryProject } from "~/lib/projects";

function loadWorkEntry() {
  return Promise.all([listProjects(), listSessions()]).then(([projects, sessions]) => ({
    projects: projects.projects,
    sessions: sessions.sessions,
  }));
}

export default function HomePage() {
  const [data] = createResource(loadWorkEntry);
  const summaries = createMemo(() => {
    const current = data();
    return current ? buildProjectWorkSummaries(current.projects, current.sessions) : [];
  });
  const featured = createMemo(() => selectGuidedEntryProject(summaries()));

  return (
    <section class="page page-scroll">
      <Title>Planner Work Entry</Title>
      <div class="stack page-frame">
        <section class="section-panel home-entry-panel">
          <div class="home-entry-head">
            <div>
              <div class="eyebrow">Work entry</div>
              <h1 class="home-entry-title">Project work</h1>
              <p class="home-entry-copy">
                Projects stay primary. Reopen the next project, or use a direct session when you need a focused one-off analysis.
              </p>
            </div>
            <A class="btn btn-subtle" href="/projects">
              All projects
            </A>
          </div>
          <Show
            when={featured()}
            fallback={
              <div class="home-entry-actions">
                <A class="btn btn-primary" href="/projects/new">
                  Start the first project
                </A>
                <A class="btn btn-subtle" href="/sessions/new">
                  Direct session
                </A>
              </div>
            }
          >
            {summary => (
              <div class="home-entry-spotlight">
                <div class="home-entry-spotlight-meta">
                  <span class={`state-badge is-${summary().status}`}>{summary().statusLabel}</span>
                  <span>{summary().sessionCount} sessions</span>
                </div>
                <h2 class="home-entry-spotlight-title">{summary().project.name}</h2>
                <p class="home-entry-spotlight-copy">
                  {summary().primarySession?.project_description?.trim() ||
                    summary().project.description?.trim() ||
                    "Continue shaping the current idea without hunting through route clutter."}
                </p>
                <div class="home-entry-actions">
                  <A class="btn btn-primary" href={`/projects/${summary().project.slug}`}>
                    {summary().nextActionLabel}
                  </A>
                  <A class="btn btn-subtle" href="/projects/new">
                    New project
                  </A>
                  <A class="btn btn-subtle" href="/sessions/new">
                    Direct session
                  </A>
                </div>
              </div>
            )}
          </Show>
        </section>

        <section class="section-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Recent projects</div>
              <h2 class="section-title">Project-first work directory</h2>
            </div>
            <A class="btn btn-subtle" href="/projects">
              All projects
            </A>
          </div>

          <Show
            when={data()}
            fallback={<div class="empty-state">Loading recent project work…</div>}
          >
            <Show
              when={summaries().length > 0}
              fallback={<div class="empty-state">No projects yet. Create one and start a Socratic analysis.</div>}
            >
              <div class="project-list compact">
                <For each={summaries().slice(0, 4)}>
                  {summary => (
                    <A class="project-row" href={`/projects/${summary.project.slug}`}>
                      <div class="project-row-main">
                        <div class="project-row-title">{summary.project.name}</div>
                        <div class="project-row-copy">
                          {summary.primarySession?.project_description?.trim() ||
                            summary.project.description?.trim() ||
                            "Ready for a new analysis path."}
                        </div>
                      </div>
                      <div class="project-row-facts">
                        <span class={`state-badge is-${summary.status}`}>{summary.statusLabel}</span>
                        <span>{summary.sessionCount} sessions</span>
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
