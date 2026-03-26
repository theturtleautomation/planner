import type { JSX } from "solid-js";

import type { ProjectSurfaceTone } from "~/lib/project-surface";

import styles from "./StatusBadge.module.css";

interface StatusBadgeProps {
  tone: ProjectSurfaceTone;
  children: JSX.Element;
  class?: string;
}

export function StatusBadge(props: StatusBadgeProps) {
  return (
    <span
      class={`${styles.root}${props.class ? ` ${props.class}` : ""}`}
      classList={{
        [styles.active]: props.tone === "active",
        [styles.attention]: props.tone === "attention",
        [styles.recent]: props.tone === "recent",
        [styles.quiet]: props.tone === "quiet",
      }}
    >
      {props.children}
    </span>
  );
}
