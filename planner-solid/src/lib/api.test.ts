import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { listSessions } from "./api";

describe("planner API client", () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    fetchMock.mockReset();
    vi.unstubAllGlobals();
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
});
