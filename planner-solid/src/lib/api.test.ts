import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  createSession,
  deleteProject,
  listProjects,
  listSessions,
  resetApiCacheForTesting,
} from "./api";
import { resetMockStateForTesting } from "./mock/store";
import { setMockRuntimeLocationSearch } from "./mock/runtime";

describe("planner API client", () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    fetchMock.mockReset();
    resetApiCacheForTesting();
    resetMockStateForTesting();
    setMockRuntimeLocationSearch("");
    vi.unstubAllGlobals();
    vi.unstubAllEnvs();
  });

  it("surfaces a clear error when an API request returns HTML", async () => {
    fetchMock.mockResolvedValue(
      new Response("<!DOCTYPE html><html><body>Planner</body></html>", {
        status: 200,
        headers: {
          "Content-Type": "text/html",
        },
      }),
    );

    await expect(listSessions()).rejects.toThrow(
      "Expected JSON from /sessions, but received non-JSON content: <!DOCTYPE html><html><body>Planner</body></html>",
    );
  });

  it("serves frontend mock data without calling fetch when mock mode is enabled", async () => {
    vi.stubEnv("VITE_PLANNER_FRONTEND_MOCK", "1");
    const response = await listSessions();

    expect(fetchMock).not.toHaveBeenCalled();
    expect(response.sessions.length).toBeGreaterThan(0);
  });

  it("invalidates cached sessions after creating a direct session in mock mode", async () => {
    vi.stubEnv("VITE_PLANNER_FRONTEND_MOCK", "1");

    const before = await listSessions();
    await createSession({ description: "Map a lightweight itinerary flow." });
    const after = await listSessions();

    expect(after.sessions).toHaveLength(before.sessions.length + 1);
  });

  it("serves project deletion through the mock provider", async () => {
    vi.stubEnv("VITE_PLANNER_FRONTEND_MOCK", "1");

    const before = await listProjects();
    const deleted = await deleteProject(before.projects[0]!.slug);
    const after = await listProjects();

    expect(deleted.deleted_project_record).toBe(true);
    expect(after.projects).toHaveLength(before.projects.length - 1);
  });
});
