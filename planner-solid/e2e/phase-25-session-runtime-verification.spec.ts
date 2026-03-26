// @ts-nocheck
import { expect, test } from "@playwright/test";

function bankedThread(categoryId: string, title: string, question: string) {
  return {
    category_id: categoryId,
    title,
    summary: `${title} summary`,
    question_count: 1,
    prompt: {
      prompt_id: `prompt-${categoryId}`,
      title,
      kind: "question_batch",
      origin_category_id: categoryId,
      items: [
        {
          item_id: `${categoryId}-q1`,
          kind: "discovery",
          text: question,
          options: [],
          required: true,
        },
      ],
      allow_partial_submit: true,
    },
  };
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    window.__socketMessages = [];
    window.__socketInstances = [];
    window.__dispatchSocketMessage = (payload) => {
      for (const socket of window.__socketInstances) {
        socket.onmessage?.({ data: JSON.stringify(payload) });
      }
    };

    class MockWebSocket {
      static OPEN = 1;
      readyState = 1;
      onopen = null;
      onclose = null;
      onerror = null;
      onmessage = null;

      constructor(_url) {
        window.__socketInstances.push(this);
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

test("phase 25 mocked client proof reveals the workspace only after the first truthful prompt bank arrives", async ({ page }) => {
  await page.route("**/api/sessions/session-start-proof", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session: {
          id: "session-start-proof",
          title: "Startup proof",
          archived: false,
          created_at: "2026-03-26T00:00:00Z",
          last_activity_at: "2026-03-26T00:01:00Z",
          pipeline_running: false,
          intake_phase: "waiting",
          project_description: "Workout countdown timer",
          project_id: "project-1",
          project_slug: "workout-countdown",
          project_name: "Workout Countdown",
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

  await page.route("**/api/sessions/session-start-proof/prompt-bank", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session_id: "session-start-proof",
        active_thread_id: null,
        banked_threads: [],
        queued_threads: [],
        build_ready: false,
        build_readiness_message: null,
        initial_bank_complete: false,
        saved_drafts: {},
      }),
    });
  });

  await page.goto("/sessions/session-start-proof");

  await expect(page.getByRole("heading", { name: "Starting analysis" })).toBeVisible();
  await expect(page.getByText("Building the initial prompt bank")).toHaveCount(0);

  await expect.poll(async () => {
    const messages = await page.evaluate(() => window.__socketMessages);
    return messages;
  }).toEqual([
    {
      type: "start_socratic",
      description: "Workout countdown timer",
    },
  ]);

  await page.evaluate((payload) => {
    window.__dispatchSocketMessage({
      type: "prompt_bank",
      bank: payload,
    });
  }, {
    session_id: "session-start-proof",
    active_thread_id: "goal",
    banked_threads: [
      bankedThread("goal", "Goal", "What should this timer help someone accomplish first?"),
      bankedThread("success", "Success criteria", "How will you know the first release works?"),
    ],
    queued_threads: [
      {
        category_id: "scope",
        title: "Out of scope",
        summary: "Queue later",
        status: "pending",
        question_count: 1,
      },
    ],
    build_ready: false,
    build_readiness_message: null,
    initial_bank_complete: true,
    saved_drafts: {},
  });

  await expect(page.locator(".session-interview-thread-title")).toHaveText("Goal");
  await expect(page.locator(".session-thread-chip")).toContainText(["Goal", "Success criteria"]);
  await expect(page.getByText("Building the initial prompt bank")).toHaveCount(0);
});

test("phase 25 mocked client proof reloads an in-progress interview from the bank-first contract without restarting startup", async ({ page }) => {
  await page.route("**/api/sessions/session-reload-proof", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session: {
          id: "session-reload-proof",
          title: "Reload proof",
          archived: false,
          created_at: "2026-03-26T00:00:00Z",
          last_activity_at: "2026-03-26T00:10:00Z",
          pipeline_running: false,
          intake_phase: "interviewing",
          project_description: "Workout countdown timer",
          project_id: "project-1",
          project_slug: "workout-countdown",
          project_name: "Workout Countdown",
          current_step: "socratic.prompt.generated",
          error_message: null,
          can_resume_live: false,
          can_resume_checkpoint: true,
          can_restart_from_description: true,
          can_retry_pipeline: false,
          has_checkpoint: true,
          resume_status: "interview_checkpoint_resumable",
          workspace_status: {
            state: "awaiting_response",
            label: "Waiting for your response",
            detail: "The first prompt bank is ready for local switching and answers.",
            tone: "neutral",
          },
        },
      }),
    });
  });

  await page.route("**/api/sessions/session-reload-proof/prompt-bank", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session_id: "session-reload-proof",
        active_thread_id: "goal",
        banked_threads: [
          bankedThread("goal", "Goal", "What should this timer help someone accomplish first?"),
          bankedThread("success", "Success criteria", "How will you know the first release works?"),
        ],
        queued_threads: [],
        build_ready: false,
        build_readiness_message: null,
        initial_bank_complete: true,
        saved_drafts: {},
      }),
    });
  });

  await page.goto("/sessions/session-reload-proof");

  await expect(page.locator(".session-interview-thread-title")).toHaveText("Goal");
  await expect(page.getByText("Building the initial prompt bank")).toHaveCount(0);

  let sentMessages = await page.evaluate(() => window.__socketMessages);
  expect(sentMessages).toEqual([]);

  await page.reload();

  await expect(page.locator(".session-interview-thread-title")).toHaveText("Goal");
  await expect(page.locator(".session-thread-chip")).toContainText(["Goal", "Success criteria"]);
  await expect(page.getByText("Building the initial prompt bank")).toHaveCount(0);

  sentMessages = await page.evaluate(() => window.__socketMessages);
  expect(sentMessages).toEqual([]);
});
