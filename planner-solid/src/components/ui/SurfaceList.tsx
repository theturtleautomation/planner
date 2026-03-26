import { For, Show, type JSX } from "solid-js";

import styles from "./SurfaceList.module.css";

export interface SurfaceListItem {
  title: JSX.Element | string;
  copy?: JSX.Element | string;
  meta?: JSX.Element | string;
  action?: JSX.Element;
}

interface SurfaceListProps {
  items: SurfaceListItem[];
}

export function SurfaceList(props: SurfaceListProps) {
  return (
    <div class={styles.list}>
      <For each={props.items}>
        {item => (
          <div class={styles.row} classList={{ [styles.actionRow]: !!item.action }}>
            <div class={styles.content}>
              <div class={styles.title}>{item.title}</div>
              <Show when={item.copy}>
                {copy => <div class={styles.copy}>{copy()}</div>}
              </Show>
            </div>
            <Show when={item.action} fallback={<Show when={item.meta}>{meta => <div class={styles.meta}>{meta()}</div>}</Show>}>
              {action => <div class={styles.action}>{action()}</div>}
            </Show>
          </div>
        )}
      </For>
    </div>
  );
}
