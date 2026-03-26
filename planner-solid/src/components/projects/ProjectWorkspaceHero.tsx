import { A } from "@solidjs/router";
import { Show } from "solid-js";

import { StatusBadge } from "~/components/ui/StatusBadge";
import type { ProjectSurfaceTone } from "~/lib/project-surface";

import styles from "./ProjectWorkspaceHero.module.css";

interface ProjectWorkspaceHeroProps {
  projectName: string;
  projectDescription?: string | null;
  statusLabel: string;
  focusTitle: string;
  focusCopy: string;
  readinessTone: ProjectSurfaceTone;
  readinessLabel: string;
  activeSessionId?: string | null;
  starting: boolean;
  error?: string | null;
  onStartAnalysis: () => void;
}

export function ProjectWorkspaceHero(props: ProjectWorkspaceHeroProps) {
  return (
    <section class={styles.root}>
      <div class="eyebrow">Project workspace</div>
      <h1 class="hero-title">{props.projectName}</h1>
      <p class="hero-copy">
        {props.projectDescription?.trim() ||
          "Use this project as the stable container for deep Socratic analysis and the next build-shaping moves."}
      </p>
      <div class={styles.focus}>
        <div class={styles.focusBody}>
          <div class={styles.focusLabel}>{props.statusLabel}</div>
          <h2 class={styles.focusTitle}>{props.focusTitle}</h2>
          <p class={styles.focusCopy}>{props.focusCopy}</p>
        </div>
        <div class={styles.actions}>
          <StatusBadge tone={props.readinessTone}>{props.readinessLabel}</StatusBadge>
          <Show
            when={props.activeSessionId}
            fallback={
              <button
                class="btn btn-primary"
                disabled={props.starting}
                type="button"
                onClick={props.onStartAnalysis}
              >
                {props.starting ? "Starting…" : "Start analysis"}
              </button>
            }
          >
            {sessionId => (
              <>
                <A class="btn btn-primary" href={`/sessions/${sessionId()}`}>
                  Continue analysis
                </A>
                <button
                  class="btn btn-subtle"
                  disabled={props.starting}
                  type="button"
                  onClick={props.onStartAnalysis}
                >
                  {props.starting ? "Starting…" : "New analysis"}
                </button>
              </>
            )}
          </Show>
        </div>
      </div>
      {props.error ? <div class="error-copy">{props.error}</div> : null}
    </section>
  );
}
