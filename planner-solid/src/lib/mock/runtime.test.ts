import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  getFrontendMockBadgeCopy,
  getFrontendMockScenarioKey,
  isFrontendMockEnabled,
  setMockRuntimeLocationSearch,
  withFrontendMockSearch,
} from "./runtime";

describe("frontend mock runtime helpers", () => {
  beforeEach(() => {
    setMockRuntimeLocationSearch("");
  });

  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("enables frontend mock mode from the Vite env gate", () => {
    vi.stubEnv("VITE_PLANNER_FRONTEND_MOCK", "1");
    expect(isFrontendMockEnabled()).toBe(true);
  });

  it("defaults to the default scenario when no query override exists", () => {
    vi.stubEnv("VITE_PLANNER_FRONTEND_MOCK", "1");
    expect(getFrontendMockScenarioKey()).toBe("default");
  });

  it("reads the scenario override from the location search when mock mode is enabled", () => {
    vi.stubEnv("VITE_PLANNER_FRONTEND_MOCK", "1");
    window.history.replaceState({}, "", "/?mockScenario=empty");
    expect(getFrontendMockScenarioKey()).toBe("empty");
  });

  it("accepts the richer operational history scenario override", () => {
    vi.stubEnv("VITE_PLANNER_FRONTEND_MOCK", "1");
    window.history.replaceState({}, "", "/?mockScenario=ops-history");
    expect(getFrontendMockScenarioKey()).toBe("ops-history");
  });

  it("builds the shell badge copy from the active scenario", () => {
    expect(getFrontendMockBadgeCopy("default")).toBe("Frontend mock · default");
  });

  it("preserves the active mock scenario in app navigation paths", () => {
    vi.stubEnv("VITE_PLANNER_FRONTEND_MOCK", "1");
    window.history.replaceState({}, "", "/projects?mockScenario=empty");

    expect(withFrontendMockSearch("/sessions/new")).toBe("/sessions/new?mockScenario=empty");
  });
});
