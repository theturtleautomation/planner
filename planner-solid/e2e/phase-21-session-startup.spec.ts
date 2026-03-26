// @ts-nocheck
import { expect, test } from "@playwright/test";

function promptBank(sessionId: string) {
  return {
    session_id: sessionId,
    active_thread_id: null,
    banked_threads: [],
    queued_threads: [],
    build_ready: false,
    build_readiness_message: null,
    initial_bank_complete: false,
  };
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    window.__socketMessages = [];

    class MockWebSocket {
      static OPEN = 1;
      readyState = 1;
      onopen = null;
      onclose = null;
      onerror = null;
      onmessage = null;

      constructor(_url) {
        setTimeout(() => this.onopen?.(), 0);
      }

      send(payload) {
        window.__socketMessages.push(JSON.parse(payload));
      }

      close() {
        this.onclose?.();
      }
    }

    window.WebSocket = MockWebSocket;
  });
});

test("phase 21 auto-starts a saved-brief waiting session and keeps the status truthful", async ({ page }) => {
  await page.route("**/api/sessions/session-start", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session: {
          id: "session-start",
          title: "Fresh analysis",
          archived: false,
          created_at: "2026-03-25T00:00:00Z",
          last_activity_at: "2026-03-25T00:05:00Z",
          pipeline_running: false,
          intake_phase: "waiting",
          project_description: "Fresh automation concept",
          project_id: "project-1",
          project_slug: "fresh-automation",
          project_name: "Fresh Automation",
          current_step: null,
          error_message: null,
          can_resume_live: false,
          can_resume_checkpoint: false,
          can_restart_from_description: false,
          can_retry_pipeline: false,
          has_checkpoint: false,
          resume_status: "ready_to_start",
          workspace_status: {
            state: "ready_to_start",
            label: "Ready to start analysis",
            detail: "Waiting for the session workspace to begin from the saved brief.",
            tone: "neutral",
          },
        },
      }),
    });
  });

  await page.route("**/api/sessions/session-start/prompt-bank", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(promptBank("session-start")),
    });
  });

  await page.goto("/sessions/session-start");

  await expect(page.getByRole("heading", { name: "Starting analysis" })).toBeVisible();
  await expect(page.getByText("Building the initial prompt bank")).toHaveCount(0);
  await expect(
    page.getByRole("paragraph").filter({
      hasText: "Waiting for the session workspace to begin from the saved brief.",
    }),
  ).toBeVisible();
  await expect(page.getByText(/^waiting$/i)).toHaveCount(0);

  const sentMessages = await page.evaluate(() => window.__socketMessages);
  expect(sentMessages).toEqual([
    {
      type: "start_socratic",
      description: "Fresh automation concept",
    },
  ]);
});

test("phase 21 shows an explicit idle start state when no saved brief exists", async ({ page }) => {
  await page.route("**/api/sessions/session-idle", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session: {
          id: "session-idle",
          title: "Untitled session",
          archived: false,
          created_at: "2026-03-25T00:00:00Z",
          last_activity_at: "2026-03-25T00:05:00Z",
          pipeline_running: false,
          intake_phase: "waiting",
          project_description: null,
          project_id: null,
          project_slug: null,
          project_name: null,
          current_step: null,
          error_message: null,
          can_resume_live: false,
          can_resume_checkpoint: false,
          can_restart_from_description: false,
          can_retry_pipeline: false,
          has_checkpoint: false,
          resume_status: "ready_to_start",
          workspace_status: {
            state: "ready_to_start",
            label: "Ready to start analysis",
            detail: "This session does not have a saved brief yet.",
            tone: "neutral",
          },
        },
      }),
    });
  });

  await page.route("**/api/sessions/session-idle/prompt-bank", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(promptBank("session-idle")),
    });
  });

  await page.goto("/sessions/session-idle");

  await expect(page.getByRole("heading", { name: "Ready to start analysis" })).toBeVisible();
  await expect(page.getByRole("paragraph").filter({ hasText: "This session does not have a saved brief yet." })).toBeVisible();
  await expect(page.getByRole("link", { name: "Start a new session" })).toBeVisible();

  const sentMessages = await page.evaluate(() => window.__socketMessages);
  expect(sentMessages).toEqual([]);
});
