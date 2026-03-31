import { A } from "@solidjs/router";
import { For, Show } from "solid-js";

import { withFrontendMockSearch } from "~/lib/mock/runtime";
import { formatProjectSurfaceTimestamp } from "~/lib/project-surface";
import { getSessionSummaryStatus, getSessionSummarySurfaceTone } from "~/lib/session-status";
import type { SessionSummary } from "~/lib/types";
import { presentSessionTitle } from "~/lib/workspace";

import styles from "./ProjectSessionList.module.css";

interface ProjectSessionListProps {
  sessions: SessionSummary[];
}

export function ProjectSessionList(props: ProjectSessionListProps) {
  return (
    <section class={styles.root}>
      <div class={styles.head}>
        <div>
          <h2 class="section-title">Analysis sessions</h2>
        </div>
        <A class="btn btn-subtle" href={withFrontendMockSearch("/sessions")}>
          All sessions
        </A>
      </div>

      <Show
        when={props.sessions.length > 0}
        fallback={<div class="empty-state">No sessions yet. Start the first analysis from this workspace.</div>}
      >
        <div class={styles.list}>
          <For each={props.sessions.slice(0, 6)}>
            {session => {
              const summaryStatus = getSessionSummaryStatus(session);
              const surfaceTone = getSessionSummarySurfaceTone(session);

              return (
                <A class={styles.row} href={withFrontendMockSearch(`/sessions/${session.id}`)}>
                  <div class={styles.rowMain}>
                    <div class={styles.rowTitle}>{presentSessionTitle(session)}</div>
                    <div class={styles.rowCopy}>
                      {session.project_description?.trim() || "Project analysis session"}
                    </div>
                  </div>
                  <div class={styles.rowFacts}>
                    <span class={`state-badge is-${surfaceTone}`}>{summaryStatus.label}</span>
                    <span>Updated {formatProjectSurfaceTimestamp(session.last_activity_at)}</span>
                  </div>
                </A>
              );
            }}
          </For>
        </div>
      </Show>
    </section>
  );
}
