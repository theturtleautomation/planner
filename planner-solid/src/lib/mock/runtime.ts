export const FRONTEND_MOCK_ENV = "VITE_PLANNER_FRONTEND_MOCK";

export const MOCK_SCENARIO_KEYS = [
  "default",
  "empty",
  "session-workspace",
  "session-startup",
  "session-complete",
  "session-attention",
  "project-active",
  "project-ready",
  "project-empty",
  "import-review",
  "import-applied",
  "import-empty",
  "multi-project-graph",
  "ops-quiet",
  "ops-history",
  "ops-attention",
] as const;

export type MockScenarioKey = (typeof MOCK_SCENARIO_KEYS)[number];

let locationSearch = "";

function isScenarioKey(value: string): value is MockScenarioKey {
  return (MOCK_SCENARIO_KEYS as readonly string[]).includes(value);
}

function currentSearch(): string {
  if (typeof window !== "undefined") {
    return window.location.search;
  }
  return locationSearch;
}

export function isFrontendMockEnabled(): boolean {
  return import.meta.env[FRONTEND_MOCK_ENV] === "1";
}

export function setMockRuntimeLocationSearch(search: string): void {
  locationSearch = search;
}

export function getFrontendMockScenarioKey(): MockScenarioKey {
  if (!isFrontendMockEnabled()) {
    return "default";
  }

  const params = new URLSearchParams(currentSearch());
  const requested = params.get("mockScenario")?.trim();
  if (requested && isScenarioKey(requested)) {
    return requested;
  }
  return "default";
}

export function getFrontendMockBadgeCopy(scenarioKey: MockScenarioKey): string {
  return `Frontend mock · ${scenarioKey}`;
}

export function withFrontendMockSearch(path: string): string {
  if (!isFrontendMockEnabled()) {
    return path;
  }

  const url = new URL(path, "http://planner.local");
  url.searchParams.set("mockScenario", getFrontendMockScenarioKey());
  return `${url.pathname}${url.search}${url.hash}`;
}
