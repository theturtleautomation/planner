// @ts-nocheck
import { expect, test } from "@playwright/test";

const eventPayload = {
  total: 3,
  events: [
    {
      event_type: "export_recorded",
      summary: "Recorded project export for Personal Calendar",
      timestamp: "2026-03-24T06:10:00Z",
      data: {
        project_id: "project-1",
        project_name: "Personal Calendar",
        node_count: 6,
      },
    },
    {
      event_type: "node_updated",
      summary: "Updated Task Service component",
      timestamp: "2026-03-24T06:05:00Z",
      data: {
        node_id: "component-1",
        field: "status",
        before: "planned",
        after: "active",
      },
    },
    {
      event_type: "node_created",
      summary: "Created Reminder Engine component",
      timestamp: "2026-03-23T20:00:00Z",
      data: {
        node_id: "component-2",
      },
    },
  ],
};

const historyPayload = {
  snapshots: [
    {
      timestamp: "2026-03-24T06:00:00Z",
      filename: "2026-03-24-personal-calendar.msgpack",
    },
  ],
};

test.beforeEach(async ({ page }) => {
  await page.route("**/api/blueprint/events?**", async route => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(eventPayload),
    });
  });

  await page.route("**/api/blueprint/history", async route => {
    if (route.request().method() === "POST") {
      await route.fulfill({
        contentType: "application/json",
        body: JSON.stringify({
          timestamp: "2026-03-24T06:12:00Z",
          filename: "2026-03-24-personal-calendar-2.msgpack",
        }),
      });
      return;
    }

    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify(historyPayload),
    });
  });
});

test("phase 08 keeps the event stream primary while snapshots stay secondary", async ({ page }) => {
  await page.goto("/events");

  await expect(page.getByRole("heading", { name: "Events", exact: true })).toBeVisible();
  await expect(page.getByText("Recorded project export for Personal Calendar")).toBeVisible();
  await expect(page.getByRole("button", { name: "Exports" })).toBeVisible();

  await page.getByRole("button", { name: "Exports" }).click();
  await expect(page.getByText("Recorded project export for Personal Calendar")).toBeVisible();
  await expect(page.getByText("Updated Task Service component")).not.toBeVisible();

  await page.getByRole("tab", { name: "Snapshots" }).click();
  await expect(page.getByText("2026-03-24-personal-calendar.msgpack")).toBeVisible();
  await page.getByRole("button", { name: "Create snapshot" }).click();
  await expect(page.getByText("2026-03-24-personal-calendar.msgpack")).toBeVisible();
});
