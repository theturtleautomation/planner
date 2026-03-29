import { Title } from "@solidjs/meta";
import { A } from "@solidjs/router";
import { For, Show, createResource } from "solid-js";

import { listSessions } from "~/lib/api";
import { getSessionSummaryStatus, getSessionSummarySurfaceTone } from "~/lib/session-status";
import { presentSessionTitle } from "~/lib/workspace";

function formatTimestamp(value: string): string {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export default function SessionsPage() {
  const [data] = createResource(listSessions);

  return (
    <section class="page page-scroll">
      <Title>Sessions</Title>
      <div class="stack page-frame">
        <section class="section-panel page-intro-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Sessions</div>
              <h1 class="page-title">Current work queue</h1>
              <p class="page-copy">
                Reopen work already in motion here. Projects remain the primary container for
                ongoing work, and direct sessions stay available only as a focused detour.
              </p>
            </div>
            <div class="page-actions">
              <A class="btn btn-primary" href="/projects/new">
                New project
              </A>
              <A class="btn btn-subtle" href="/sessions/new">
                Direct session
              </A>
            </div>
          </div>
        </section>

        <section class="section-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Queue</div>
              <h2 class="section-title">All sessions</h2>
              <p class="section-copy">
                Open active work directly. New long-lived work should start as a project.
              </p>
            </div>
          </div>

          <Show when={data.latest} fallback={<div class="stack"><div class="empty-state">Loading sessions…</div></div>}>
            {(response) => (
              <Show
                when={response().sessions.length > 0}
                fallback={<div class="stack"><div class="empty-state">No sessions exist yet.</div></div>}
              >
                <div class="queue-list">
                  <For each={response().sessions}>
                    {(session) => {
                      const summaryStatus = getSessionSummaryStatus(session);
                      const surfaceTone = getSessionSummarySurfaceTone(session);

                      return (
                        <A class="queue-row" href={`/sessions/${session.id}`}>
                          <div>
                            <h3 class="queue-title">{presentSessionTitle(session)}</h3>
                            <div class="queue-meta">
                              <span class={`state-badge is-${surfaceTone}`}>{summaryStatus.label}</span>
                              <Show when={session.project_name}>
                                <span>{session.project_name}</span>
                              </Show>
                            </div>
                            <Show when={session.project_description}>
                              <p class="panel-copy">{session.project_description}</p>
                            </Show>
                          </div>
                          <div class="queue-facts">
                            <span>Updated {formatTimestamp(session.last_activity_at)}</span>
                            <span>{summaryStatus.nextActionLabel}</span>
                          </div>
                        </A>
                      );
                    }}
                  </For>
                </div>
              </Show>
            )}
          </Show>
        </section>
      </div>
    </section>
  );
}
