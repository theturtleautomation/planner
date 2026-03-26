import type { JSX } from "solid-js";

import styles from "./MetricCard.module.css";

interface MetricCardProps {
  label: string;
  value: JSX.Element | string | number;
  text?: boolean;
}

export function MetricCard(props: MetricCardProps) {
  return (
    <div class={styles.card}>
      <div class={styles.label}>{props.label}</div>
      <div class={styles.value} classList={{ [styles.text]: props.text }}>
        {props.value}
      </div>
    </div>
  );
}
