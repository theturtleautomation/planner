import { Title } from "@solidjs/meta";
import { createMemo, createResource, createSignal, For, Show } from "solid-js";

import { getAdminEvents, getAdminStatus } from "~/lib/api";

function formatUptime(secs: number): string {
  if (secs <= 0) return "0m";
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const parts: string[] = [];
  if (d > 0) parts.push(`${d}d`);
  if (h > 0) parts.push(`${h}h`);
  if (m > 0 || parts.length === 0) parts.push(`${m}m`);
  return parts.join(" ");
}

function formatTimestamp(value: string): string {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

function levelBadgeClass(level: string): string {
  if (level === "error") return "state-badge is-attention";
  if (level === "warn") return "state-badge is-recent";
  return "state-badge is-active";
}

export default function AdminPage() {
  const [level, setLevel] = createSignal<"all" | "error" | "warn" | "info">("all");
  const [status] = createResource(getAdminStatus);
  const [events, { refetch }] = createResource(
    () => level(),
    async nextLevel =>
      getAdminEvents({
        limit: 200,
        level: nextLevel === "all" ? undefined : nextLevel,
      }),
  );

  const posture = createMemo(() => {
    const current = status();
    if (!current) {
      return {
        label: "Loading",
        headline: "Checking runtime posture",
        copy: "Reading runtime status and provider availability.",
      };
    }

    const unavailable = current.providers.filter(provider => !provider.available);
    if (current.status !== "ok" || unavailable.length > 0) {
      return {
        label: "Needs attention",
        headline: "Runtime posture is degraded or missing providers",
        copy:
          unavailable.length > 0
            ? `${unavailable.length} provider${unavailable.length === 1 ? "" : "s"} unavailable.`
            : "Runtime health is reporting a non-ok status.",
      };
    }

    return {
      label: "Healthy",
      headline: "Runtime posture is healthy",
      copy: "Providers, sessions, and recent event flow are currently stable.",
    };
  });

  return (
    <section class="page page-scroll">
      <Title>Admin</Title>
      <div class="stack page-frame">
        <section class="hero-panel workspace-hero">
          <div class="eyebrow">Operations</div>
          <h1 class="hero-title">Admin</h1>
          <p class="hero-copy">
            One dominant health desk first, then recent operational events below it. This route stays dense without turning into dashboard theater.
          </p>
          <div class="hero-focus project-focus">
            <div>
              <div class="hero-focus-label">{posture().label}</div>
              <h2 class="hero-focus-title">{posture().headline}</h2>
              <p class="hero-focus-copy">{posture().copy}</p>
            </div>
            <div class="hero-actions">
              <button class="btn btn-subtle" type="button" onClick={() => void refetch()}>
                Refresh
              </button>
            </div>
          </div>
        </section>

        <section class="section-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Health desk</div>
              <h2 class="section-title">Runtime posture</h2>
            </div>
          </div>

          <Show when={status()} fallback={<div class="advanced-loading">Loading runtime status…</div>}>
            {current => (
              <>
                <div class="advanced-summary-grid">
                  <div class="advanced-summary-card">
                    <div class="advanced-label">Status</div>
                    <div class="advanced-metric advanced-metric-text">{current().status}</div>
                  </div>
                  <div class="advanced-summary-card">
                    <div class="advanced-label">Version</div>
                    <div class="advanced-metric advanced-metric-text">{current().version}</div>
                  </div>
                  <div class="advanced-summary-card">
                    <div class="advanced-label">Uptime</div>
                    <div class="advanced-metric advanced-metric-text">{formatUptime(current().uptime_secs)}</div>
                  </div>
                  <div class="advanced-summary-card">
                    <div class="advanced-label">Active sessions</div>
                    <div class="advanced-metric">{current().sessions.active}</div>
                  </div>
                </div>
                <div class="advanced-list">
                  <For each={current().providers}>
                    {provider => (
                      <div class="advanced-list-row">
                        <div>
                          <div class="advanced-item-title">{provider.name}</div>
                          <div class="advanced-item-copy">{provider.binary}</div>
                        </div>
                        <span class={provider.available ? "state-badge is-active" : "state-badge is-attention"}>
                          {provider.available ? "Available" : "Unavailable"}
                        </span>
                      </div>
                    )}
                  </For>
                </div>
              </>
            )}
          </Show>
        </section>

        <section class="section-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Event stream</div>
              <h2 class="section-title">Recent operator-visible events</h2>
            </div>
            <div class="timeline-filter-row">
              <button class={`timeline-filter-chip${level() === "all" ? " is-active" : ""}`} type="button" onClick={() => setLevel("all")}>
                All
              </button>
              <button class={`timeline-filter-chip${level() === "error" ? " is-active" : ""}`} type="button" onClick={() => setLevel("error")}>
                Errors
              </button>
              <button class={`timeline-filter-chip${level() === "warn" ? " is-active" : ""}`} type="button" onClick={() => setLevel("warn")}>
                Warnings
              </button>
              <button class={`timeline-filter-chip${level() === "info" ? " is-active" : ""}`} type="button" onClick={() => setLevel("info")}>
                Info
              </button>
            </div>
          </div>

          <Show when={!events.loading} fallback={<div class="advanced-loading">Loading admin events…</div>}>
            <Show
              when={(events()?.events.length ?? 0) > 0}
              fallback={<div class="empty-state">No admin events match the current filter.</div>}
            >
              <div class="advanced-list">
                <For each={events()?.events ?? []}>
                  {event => (
                    <div class="advanced-list-row timeline-row">
                      <div>
                        <div class="advanced-item-title timeline-event-head">
                          <span class={levelBadgeClass(event.level)}>{event.level}</span>
                          <span>{event.message}</span>
                        </div>
                        <div class="advanced-item-copy">
                          {event.source}
                          {event.step ? ` · ${event.step}` : ""}
                          {event.project_name ? ` · ${event.project_name}` : ""}
                        </div>
                      </div>
                      <div class="advanced-item-meta">
                        <div>{formatTimestamp(event.timestamp)}</div>
                        <div>{event.session_id ? event.session_id.slice(0, 8) : "system"}</div>
                      </div>
                    </div>
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
