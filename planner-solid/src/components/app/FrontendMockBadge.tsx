import { Show } from "solid-js";

interface FrontendMockBadgeProps {
  active: boolean;
  label: string;
}

export function FrontendMockBadge(props: FrontendMockBadgeProps) {
  return (
    <Show when={props.active}>
      <span class="app-mock-badge" data-testid="frontend-mock-badge">
        {props.label}
      </span>
    </Show>
  );
}
