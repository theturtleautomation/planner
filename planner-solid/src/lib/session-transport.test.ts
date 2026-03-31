import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { getMockPromptBank, getMockSession, resetMockStateForTesting } from "./mock/store";
import { setMockRuntimeLocationSearch } from "./mock/runtime";
import { createSessionTransport } from "./session-transport";

describe("session transport", () => {
  beforeEach(() => {
    vi.stubEnv("VITE_PLANNER_FRONTEND_MOCK", "1");
    setMockRuntimeLocationSearch("");
    resetMockStateForTesting();
  });

  afterEach(() => {
    resetMockStateForTesting();
    vi.unstubAllEnvs();
  });

  it("emits a prompt bank after the mock startup handshake", async () => {
    const transport = createSessionTransport("session-1");
    const payloads: Array<Record<string, unknown>> = [];

    await new Promise<void>(resolve => {
      transport.onopen = () => {
        transport.send(
          JSON.stringify({
            type: "start_socratic",
            description: "A local-first calendar app with task planning.",
          }),
        );
      };
      transport.onmessage = event => {
        payloads.push(JSON.parse(event.data) as Record<string, unknown>);
        resolve();
      };
    });

    expect(payloads[0]?.type).toBe("prompt_bank");
    expect(getMockPromptBank("session-1").initial_bank_complete).toBe(true);
    expect(getMockSession("session-1").session.intake_phase).toBe("interviewing");
  });
});
