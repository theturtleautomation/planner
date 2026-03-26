import { Tabs } from "@kobalte/core";
import { For, type JSX } from "solid-js";

import styles from "./AttachedTabs.module.css";

export interface AttachedTabItem<T extends string> {
  value: T;
  label: string;
  content: JSX.Element;
}

interface AttachedTabsProps<T extends string> {
  label: string;
  value: T;
  items: AttachedTabItem<T>[];
  onChange: (value: T) => void;
}

export function AttachedTabs<T extends string>(props: AttachedTabsProps<T>) {
  return (
    <Tabs.Root
      class={styles.root}
      value={props.value}
      onChange={value => props.onChange(value as T)}
    >
      <Tabs.List class={styles.list} aria-label={props.label}>
        <For each={props.items}>
          {item => (
            <Tabs.Trigger class={styles.trigger} value={item.value}>
              {item.label}
            </Tabs.Trigger>
          )}
        </For>
      </Tabs.List>
      <For each={props.items}>
        {item => (
          <Tabs.Content class={styles.content} value={item.value}>
            {item.content}
          </Tabs.Content>
        )}
      </For>
    </Tabs.Root>
  );
}
