import { Title } from "@solidjs/meta";
import { createResource, createMemo, createSignal, For, Match, Show, Switch } from "solid-js";

import { createBlueprintSnapshot, listBlueprintEvents, listBlueprintHistory } from "~/lib/api";
import type { BlueprintEventPayload } from "~/lib/types";

const EVENT_TYPES = [
  { key: "all", label: "All events" },
  { key: "node_created", label: "Created" },
  { key: "node_updated", label: "Updated" },
  { key: "node_deleted", label: "Deleted" },
  { key: "edge_created", label: "Edge created" },
  { key: "edges_deleted", label: "Edges deleted" },
  { key: "export_recorded", label: "Exports" },
] as const;

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

function relativeTime(value: string): string {
  const diff = Date.now() - new Date(value).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  const days = Math.floor(hrs / 24);
  return `${days}d ago`;
}

function eventDayLabel(value: string): string {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleDateString([], {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

export default function EventsPage() {
  const [activeSection, setActiveSection] = createSignal<"events" | "snapshots">("events");
  const [filterType, setFilterType] = createSignal<(typeof EVENT_TYPES)[number]["key"]>("all");
  const [limit, setLimit] = createSignal(100);
  const [snapshotPending, setSnapshotPending] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const [events, { refetch: refetchEvents }] = createResource(
    () => limit(),
    async nextLimit => listBlueprintEvents({ limit: nextLimit }),
  );
  const [snapshots, { refetch: refetchSnapshots }] = createResource(
    () => activeSection(),
    async section => (section === "snapshots" ? listBlueprintHistory() : null),
  );

  const filteredEvents = createMemo(() => {
    const entries = events()?.events ?? [];
    const currentFilter = filterType();
    return currentFilter === "all" ? entries : entries.filter(event => event.event_type === currentFilter);
  });

  const groupedEvents = createMemo(() => {
    const groups: Array<{ key: string; label: string; events: BlueprintEventPayload[] }> = [];
    for (const event of filteredEvents()) {
      const label = eventDayLabel(event.timestamp);
      const existing = groups.at(-1);
      if (existing && existing.label === label) {
        existing.events.push(event);
      } else {
        groups.push({
          key: `${label}-${groups.length}`,
          label,
          events: [event],
        });
      }
    }
    return groups;
  });

  const handleRefresh = async () => {
    setError(null);
    try {
      if (activeSection() === "events") {
        await refetchEvents();
      } else {
        await refetchSnapshots();
      }
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : "Unable to refresh events.");
    }
  };

  const handleCreateSnapshot = async () => {
    setSnapshotPending(true);
    setError(null);
    try {
      await createBlueprintSnapshot();
      await refetchSnapshots();
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : "Unable to create a snapshot.");
    } finally {
      setSnapshotPending(false);
    }
  };

  return (
    <section class="page page-scroll">
      <Title>Events</Title>
      <div class="stack page-frame">
        <section class="hero-panel workspace-hero">
          <div class="eyebrow">Operational timeline</div>
          <h1 class="hero-title">Events</h1>
          <p class="hero-copy">
            Chronological blueprint activity stays primary. Snapshots remain close as a quieter recovery and audit surface.
          </p>
          <div class="hero-focus project-focus">
            <div>
              <div class="hero-focus-label">
                {activeSection() === "events" ? "Timeline view" : "Snapshot history"}
              </div>
              <h2 class="hero-focus-title">
                {activeSection() === "events"
                  ? `${filteredEvents().length} visible event${filteredEvents().length === 1 ? "" : "s"}`
                  : `${snapshots()?.snapshots.length ?? 0} saved snapshot${
                      (snapshots()?.snapshots.length ?? 0) === 1 ? "" : "s"
                    }`}
              </h2>
              <p class="hero-focus-copy">
                {activeSection() === "events"
                  ? "Use compact filters to narrow the main stream without displacing chronology."
                  : "Create or inspect snapshots without turning the route into a second primary surface."}
              </p>
            </div>
            <div class="hero-actions">
              <button class="btn btn-subtle" type="button" onClick={() => void handleRefresh()}>
                Refresh
              </button>
              <Show when={activeSection() === "snapshots"}>
                <button class="btn btn-primary" type="button" disabled={snapshotPending()} onClick={() => void handleCreateSnapshot()}>
                  {snapshotPending() ? "Creating…" : "Create snapshot"}
                </button>
              </Show>
            </div>
          </div>
          {error() ? <div class="error-copy">{error()}</div> : null}
        </section>

        <section class="section-panel">
          <div class="section-head">
            <div>
              <div class="eyebrow">Route mode</div>
              <h2 class="section-title">Timeline workspace</h2>
            </div>
            <div class="advanced-tab-row" role="tablist" aria-label="Events route sections">
              <button
                class={`advanced-tab${activeSection() === "events" ? " is-active" : ""}`}
                type="button"
                role="tab"
                aria-selected={activeSection() === "events"}
                onClick={() => setActiveSection("events")}
              >
                Events
              </button>
              <button
                class={`advanced-tab${activeSection() === "snapshots" ? " is-active" : ""}`}
                type="button"
                role="tab"
                aria-selected={activeSection() === "snapshots"}
                onClick={() => setActiveSection("snapshots")}
              >
                Snapshots
              </button>
            </div>
          </div>

          <Switch>
            <Match when={activeSection() === "events"}>
              <div class="timeline-toolbar">
                <div class="timeline-filter-row">
                  <For each={EVENT_TYPES}>
                    {eventType => (
                      <button
                        class={`timeline-filter-chip${filterType() === eventType.key ? " is-active" : ""}`}
                        type="button"
                        onClick={() => setFilterType(eventType.key)}
                      >
                        {eventType.label}
                      </button>
                    )}
                  </For>
                </div>
                <label class="timeline-limit-field">
                  <span>Visible range</span>
                  <select value={String(limit())} onInput={event => setLimit(Number(event.currentTarget.value))}>
                    <option value="50">Last 50</option>
                    <option value="100">Last 100</option>
                    <option value="250">Last 250</option>
                    <option value="500">Last 500</option>
                  </select>
                </label>
              </div>

              <Show when={!events.loading} fallback={<div class="advanced-loading">Loading event timeline…</div>}>
                <Show
                  when={groupedEvents().length > 0}
                  fallback={<div class="empty-state">No events match the current filter.</div>}
                >
                  <div class="timeline-stack">
                    <For each={groupedEvents()}>
                      {group => (
                        <section class="timeline-group">
                          <div class="timeline-group-head">
                            <h3 class="section-title timeline-group-title">{group.label}</h3>
                            <span class="pill">
                              {group.events.length} event{group.events.length === 1 ? "" : "s"}
                            </span>
                          </div>
                          <div class="advanced-list">
                            <For each={group.events}>
                              {event => (
                                <div class="advanced-list-row timeline-row">
                                  <div>
                                    <div class="advanced-item-title timeline-event-head">
                                      <span class={`timeline-event-badge is-${event.event_type}`}>{event.event_type.replace(/_/g, " ")}</span>
                                      <span>{event.summary}</span>
                                    </div>
                                    <Show when={Object.keys(event.data ?? {}).length > 0}>
                                      <pre class="timeline-event-data">{JSON.stringify(event.data, null, 2)}</pre>
                                    </Show>
                                  </div>
                                  <div class="advanced-item-meta">
                                    <div>{relativeTime(event.timestamp)}</div>
                                    <div>{formatTimestamp(event.timestamp)}</div>
                                  </div>
                                </div>
                              )}
                            </For>
                          </div>
                        </section>
                      )}
                    </For>
                  </div>
                </Show>
              </Show>
            </Match>

            <Match when={activeSection() === "snapshots"}>
              <Show when={!snapshots.loading} fallback={<div class="advanced-loading">Loading snapshots…</div>}>
                <Show
                  when={(snapshots()?.snapshots.length ?? 0) > 0}
                  fallback={
                    <div class="empty-state">
                      No snapshots saved yet. Create one when you want a stable blueprint checkpoint.
                    </div>
                  }
                >
                  <div class="advanced-list">
                    <For each={snapshots()?.snapshots ?? []}>
                      {snapshot => (
                        <div class="advanced-list-row">
                          <div>
                            <div class="advanced-item-title">{snapshot.filename}</div>
                            <div class="advanced-item-copy">Blueprint checkpoint</div>
                          </div>
                          <div class="advanced-item-meta">
                            <div>{relativeTime(snapshot.timestamp)}</div>
                            <div>{formatTimestamp(snapshot.timestamp)}</div>
                          </div>
                        </div>
                      )}
                    </For>
                  </div>
                </Show>
              </Show>
            </Match>
          </Switch>
        </section>
      </div>
    </section>
  );
}
