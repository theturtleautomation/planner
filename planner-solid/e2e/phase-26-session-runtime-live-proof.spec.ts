// @ts-nocheck
import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    window.__socketMessages = [];
    const NativeWebSocket = window.WebSocket;

    window.WebSocket = class TrackingWebSocket extends NativeWebSocket {
      send(payload) {
        try {
          window.__socketMessages.push(JSON.parse(String(payload)));
        } catch {
          // Ignore non-JSON payloads from browser instrumentation.
        }
        return super.send(payload);
      }
    };
  });
});

async function createWaitingSessionWithSavedBrief(request, description: string) {
  const nonce = Date.now();
  const projectResponse = await request.post("/api/projects", {
    data: {
      name: `Phase 26 Live Proof ${nonce}`,
    },
  });
  expect(projectResponse.ok()).toBeTruthy();
  const projectPayload = await projectResponse.json();
  const projectRef = projectPayload.project.slug ?? projectPayload.project.id;

  const sessionResponse = await request.post(`/api/projects/${projectRef}/sessions`, {
    data: {
      description,
    },
  });
  expect(sessionResponse.ok()).toBeTruthy();
  const sessionPayload = await sessionResponse.json();
  return sessionPayload.session.id as string;
}

async function startLiveSession(page, request, description: string) {
  const sessionId = await createWaitingSessionWithSavedBrief(request, description);
  await page.goto(`/sessions/${sessionId}`);

  const question = page.locator(".session-interview-question");
  await expect(question).toBeVisible({ timeout: 20_000 });
  await expect.poll(async () => page.locator(".session-thread-chip").count()).toBeGreaterThanOrEqual(2);
  await expect.poll(async () =>
    page.evaluate(() =>
      window.__socketMessages.filter((message) => message?.type === "start_socratic").length,
    ),
  ).toBe(1);

  return {
    sessionId,
    question,
    threadTitle: page.locator(".session-interview-thread-title"),
  };
}

test("phase 26 live reload replays the revealed bank without restarting startup", async ({ page, request }) => {
  const { question, threadTitle } = await startLiveSession(
    page,
    request,
    "Build a CLI workout timer that guides one person through interval training.",
  );

  const initialQuestion = (await question.textContent())?.trim();
  const initialTitle = (await threadTitle.textContent())?.trim();
  expect(initialQuestion).toBeTruthy();
  expect(initialTitle).toBeTruthy();

  await page.reload();

  await expect(threadTitle).toHaveText(initialTitle ?? "", { timeout: 20_000 });
  await expect(question).toHaveText(initialQuestion ?? "", { timeout: 20_000 });
  await expect(page.getByRole("heading", { name: "Starting analysis" })).toHaveCount(0);

  const sentMessages = await page.evaluate(() =>
    window.__socketMessages.filter((message) => message?.type === "start_socratic"),
  );
  expect(sentMessages).toEqual([]);
});

test("phase 26 live post-answer progression keeps the workspace visible while the bank refreshes", async ({ page, request }) => {
  const { question, threadTitle } = await startLiveSession(
    page,
    request,
    "Build a CLI workout timer that guides one person through interval training.",
  );

  const initialQuestion = (await question.textContent())?.trim() ?? "";
  await page.locator("textarea").fill("It must guide one workout from warmup through cooldown.");
  await page.getByRole("button", { name: "Commit and next" }).click();

  await expect.poll(async () =>
    page.evaluate(() =>
      window.__socketMessages.some((message) => message?.type === "prompt_response"),
    ),
  ).toBe(true);

  await expect.poll(async () => (await question.textContent())?.trim(), {
    timeout: 20_000,
  }).not.toBe(initialQuestion);

  await expect(page.getByRole("heading", { name: "Starting analysis" })).toHaveCount(0);
  await expect(threadTitle).toBeVisible();
  await expect.poll(async () => page.locator(".session-thread-chip").count()).toBeGreaterThanOrEqual(1);
});
