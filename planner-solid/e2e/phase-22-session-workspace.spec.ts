// @ts-nocheck
import { expect, test } from "@playwright/test";

const sessionPayload = {
  session: {
    id: "session-1",
    title: "Calendar analysis",
    archived: false,
    created_at: "2026-03-25T00:00:00Z",
    last_activity_at: "2026-03-25T00:10:00Z",
    pipeline_running: false,
    intake_phase: "interviewing",
    project_description: "Personal calendar analysis",
    project_id: "project-1",
    project_slug: "personal-calendar",
    project_name: "Personal Calendar",
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

const promptBankPayload = {
  session_id: "session-1",
  active_thread_id: "success_criteria",
  banked_threads: [
    {
      category_id: "success_criteria",
      title: "Success criteria",
      summary: "Define what makes the first release a success.",
      question_count: 2,
      prompt: {
        prompt_id: "prompt-success",
        title: "Success criteria",
        kind: "question_batch",
        origin_category_id: "success_criteria",
        items: [
          {
            item_id: "success-q1",
            kind: "discovery",
            text: "How will you judge the first release as successful?",
            options: [
              { option_id: "main-flow", label: "Main flow works", semantic_value: "Main flow works" },
              { option_id: "time-saved", label: "Time saved", semantic_value: "Time saved" },
            ],
            required: true,
          },
          {
            item_id: "success-q2",
            kind: "discovery",
            text: "What failure would make this release a miss?",
            options: [],
            required: false,
          },
        ],
        allow_partial_submit: true,
      },
    },
    {
      category_id: "core_workflows",
      title: "Core workflows",
      summary: "Clarify the minimum must-work flows for the first release.",
      question_count: 1,
      prompt: {
        prompt_id: "prompt-workflows",
        title: "Core workflows",
        kind: "question_batch",
        origin_category_id: "core_workflows",
        items: [
          {
            item_id: "workflow-q1",
            kind: "discovery",
            text: "Which actions must work end to end on day one?",
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
      summary: "Wait for decisions on core workflows first.",
      status: "pending",
      question_count: 2,
    },
  ],
  build_ready: false,
  build_readiness_message: null,
  initial_bank_complete: true,
  saved_drafts: {
    "success-q1": {
      prompt_id: "prompt-success",
      item_id: "success-q1",
      selected_option_id: "main-flow",
      custom_text: "Main flow means scheduling and task completion feel reliable.",
      skipped: false,
      updated_at: "2026-03-25T00:05:00Z",
    },
  },
};

async function mockSessionWorkspace(page, draftSaves) {
  await page.route("**/api/sessions/session-1", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(sessionPayload),
    });
  });

  await page.route("**/api/sessions/session-1/prompt-bank", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(promptBankPayload),
    });
  });

  await page.route("**/api/sessions/session-1/prompt-drafts", async route => {
    const payload = route.request().postDataJSON();
    draftSaves.push(payload);
    const savedCount = payload.answers.filter(
      answer => answer.selected_option_id || answer.custom_text,
    ).length;

    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        session_id: "session-1",
        prompt_id: payload.promptId,
        saved_count: savedCount,
        cleared_count: payload.answers.length - savedCount,
        saved_at: "2026-03-25T00:06:00Z",
      }),
    });
  });
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

test("phase 22 renders the active session workspace with autosave and thread switching", async ({ page }) => {
  const draftSaves = [];
  await mockSessionWorkspace(page, draftSaves);

  await page.goto("/sessions/session-1");

  await expect(page.locator(".session-thread-switcher")).toBeVisible();
  await expect(page.locator(".session-thread-switcher")).toContainText("Success criteria");
  await expect(page.locator(".session-thread-switcher")).toContainText("Core workflows");
  await expect(page.locator(".session-interview-thread-title")).toHaveText("Success criteria");
  await expect(page.locator("textarea").first()).toHaveValue(
    "Main flow means scheduling and task completion feel reliable.",
  );
  await expect(page.getByRole("button", { name: "Commit and next" })).toBeVisible();

  await page.locator("textarea").first().fill(
    "Main flow means scheduling, reminders, and task completion feel reliable.",
  );

  await page.getByRole("button", { name: /Core workflows/ }).click();

  await expect(page.locator(".session-interview-thread-title")).toHaveText("Core workflows");

  await page.locator("textarea").first().fill(
    "Creating, rescheduling, and completing tasks must all work in one pass.",
  );

  await page.getByRole("button", { name: "Commit and next" }).click();

  await expect(page.locator(".session-artifact-section").first()).toContainText("Success criteria");
});

test("phase 22 keeps interview and artifact surfaces switchable on tablet widths", async ({ page }) => {
  const draftSaves = [];
  await mockSessionWorkspace(page, draftSaves);
  await page.setViewportSize({ width: 840, height: 900 });

  await page.goto("/sessions/session-1");

  await expect(page.getByRole("tab", { name: "Interview" })).toBeVisible();
  await expect(page.getByRole("tab", { name: "Artifact" })).toBeVisible();
  await expect(page.locator(".session-interview-pane")).toBeVisible();
  await expect(page.locator(".session-artifact-pane")).toHaveCount(0);

  await page.getByRole("tab", { name: "Artifact" }).click();
  await expect(page.locator(".session-artifact-pane")).toBeVisible();
  await expect(page.locator(".session-artifact-pane")).toContainText("Success criteria");
  await expect(page.locator(".session-artifact-pane")).toContainText("Integrations");

  await page.getByRole("tab", { name: "Interview" }).click();
  await expect(page.locator(".session-interview-pane")).toBeVisible();
});
