// @ts-nocheck
import { expect, test } from "@playwright/test";

const sessionPayload = {
  session: {
    id: "session-runtime",
    title: "Runtime contract reset",
    archived: false,
    created_at: "2026-03-26T00:00:00Z",
    last_activity_at: "2026-03-26T00:10:00Z",
    pipeline_running: false,
    intake_phase: "interviewing",
    project_description: "Runtime contract reset proof",
    project_id: "project-1",
    project_slug: "runtime-contract-reset",
    project_name: "Runtime Contract Reset",
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
};

function initialPromptBank() {
  return {
    session_id: "session-runtime",
    active_thread_id: "success_criteria",
    banked_threads: [
      {
        category_id: "success_criteria",
        title: "Success criteria",
        summary: "Lock down the first outcome that matters.",
        question_count: 1,
        prompt: {
          prompt_id: "prompt-success",
          title: "Success criteria",
          kind: "question_batch",
          origin_category_id: "success_criteria",
          items: [
            {
              item_id: "success-q1",
              kind: "discovery",
              text: "What outcome matters most in the first release?",
              options: [],
              required: true,
            },
          ],
          allow_partial_submit: true,
        },
      },
    ],
    queued_threads: [
      {
        category_id: "integrations",
        title: "Integrations",
        summary: "Follow once the main release outcome is clear.",
        status: "pending",
        question_count: 2,
      },
    ],
    build_ready: false,
    build_readiness_message: null,
    initial_bank_complete: true,
    saved_drafts: {},
  };
}

function expandedPromptBank() {
  return {
    session_id: "session-runtime",
    active_thread_id: "integrations",
    banked_threads: [
      {
        category_id: "success_criteria",
        title: "Success criteria",
        summary: "Lock down the first outcome that matters.",
        question_count: 1,
        prompt: {
          prompt_id: "prompt-success",
          title: "Success criteria",
          kind: "question_batch",
          origin_category_id: "success_criteria",
          items: [
            {
              item_id: "success-q1",
              kind: "discovery",
              text: "What outcome matters most in the first release?",
              options: [],
              required: true,
            },
          ],
          allow_partial_submit: true,
        },
      },
      {
        category_id: "integrations",
        title: "Integrations",
        summary: "The answer exposed a concrete integration thread.",
        question_count: 1,
        prompt: {
          prompt_id: "prompt-integrations",
          title: "Integrations",
          kind: "question_batch",
          origin_category_id: "integrations",
          items: [
            {
              item_id: "integrations-q1",
              kind: "discovery",
              text: "Which calendar should the first release sync with?",
              options: [
                { option_id: "google", label: "Google Calendar", semantic_value: "Google Calendar" },
              ],
              required: true,
            },
          ],
          allow_partial_submit: true,
        },
      },
    ],
    queued_threads: [
      {
        category_id: "reporting",
        title: "Reporting",
        summary: "Now queued behind the integration answer.",
        status: "pending",
        question_count: 1,
      },
    ],
    build_ready: false,
    build_readiness_message: null,
    initial_bank_complete: true,
    saved_drafts: {},
  };
}

async function mockSessionWorkspace(page, draftSaves) {
  await page.route("**/api/sessions/session-runtime", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(sessionPayload),
    });
  });

  await page.route("**/api/sessions/session-runtime/prompt-bank", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(initialPromptBank()),
    });
  });

  await page.route("**/api/sessions/session-runtime/prompt-drafts", async route => {
    const payload = route.request().postDataJSON();
    draftSaves.push(payload);

    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session_id: "session-runtime",
        prompt_id: payload.prompt_id ?? payload.promptId,
        saved_count: payload.answers.length,
        cleared_count: 0,
        saved_at: "2026-03-26T00:06:00Z",
      }),
    });
  });
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

test("phase 24 renders dynamic prompt-bank expansion after answering a first-reveal thread", async ({ page }) => {
  const draftSaves = [];
  await mockSessionWorkspace(page, draftSaves);

  await page.goto("/sessions/session-runtime");

  await expect(page.locator(".session-interview-thread-title")).toHaveText("Success criteria");
  await page.locator("textarea").fill("A Google Calendar sync must work end to end.");
  await page.getByRole("button", { name: "Commit and next" }).click();

  await expect.poll(async () => {
    const messages = await page.evaluate(() => window.__socketMessages);
    return messages.some(
      (message) => message.type === "prompt_response" && message.prompt_id === "prompt-success",
    );
  }).toBe(true);

  await page.evaluate((payload) => {
    window.__dispatchSocketMessage({
      type: "prompt_bank",
      bank: payload,
    });
  }, expandedPromptBank());

  await expect(page.locator(".session-thread-chip")).toContainText(["Success criteria", "Integrations"]);
  await expect(page.locator(".session-interview-thread-title")).toHaveText("Integrations");
  await expect(page.locator(".session-interview-question")).toHaveText(
    "Which calendar should the first release sync with?",
  );
  await expect(page.locator(".session-up-next-panel")).toContainText("Reporting");

  expect(draftSaves.at(-1)).toMatchObject({
    prompt_id: "prompt-success",
    answers: [
      {
        item_id: "success-q1",
        custom_text: "A Google Calendar sync must work end to end.",
      },
    ],
  });
});
