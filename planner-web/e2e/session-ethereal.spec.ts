import { expect, test, type Page } from '@playwright/test';

test.describe.configure({ mode: 'serial' });

async function startConsultantDesk(page: Page) {
  await page.goto('/');

  if (await page.getByRole('button', { name: 'Enter Planner' }).isVisible()) {
    await page.getByRole('button', { name: 'Enter Planner' }).click();
  }

  await expect(
    page.getByRole('heading', { name: /Home Hub/i }).or(page.getByRole('button', { name: 'New Project' })),
  ).toBeVisible();

  const projectName = `Consultant Desk ${Date.now()} ${Math.random().toString(36).slice(2, 8)}`;
  await page.getByRole('button', { name: 'New Project' }).click();
  await page.getByPlaceholder('Project name').fill(projectName);
  await page.getByRole('button', { name: 'Create' }).click();

  await expect(page.getByRole('heading', { name: projectName })).toBeVisible();

  await page.getByRole('button', { name: 'New Project Session' }).click();
  const planningBrief = page.getByRole('textbox', { name: 'Planning brief' });
  await expect(planningBrief).toBeVisible();
  await planningBrief.fill('task tracking web app with calendar and reminders');
  await expect(planningBrief).toHaveValue('task tracking web app with calendar and reminders');
  const startSessionButton = page.getByRole('button', { name: 'Start Session' });
  await expect(startSessionButton).toBeEnabled();
  await startSessionButton.click();

  await expect(page.locator('.socratic-consultant-desk')).toBeVisible({ timeout: 90000 });
  await expect(page.locator('.socratic-map')).toBeVisible();
  await expect(page.locator('.socratic-desk')).toBeVisible();
}

test('consultant desk owns scroll while the document root stays locked', async ({ page }) => {
  test.setTimeout(120000);
  await startConsultantDesk(page);

  const layout = await page.evaluate(() => {
    const root = document.getElementById('root');
    const deskBody = document.querySelector('.socratic-desk__body');
    if (!(root instanceof HTMLElement) || !(deskBody instanceof HTMLElement)) {
      return null;
    }

    const filler = document.createElement('div');
    filler.setAttribute('data-scroll-probe', 'true');
    filler.style.height = '2400px';
    deskBody.appendChild(filler);

    document.documentElement.scrollTop = 180;
    document.body.scrollTop = 180;
    root.scrollTop = 180;
    deskBody.scrollTop = 220;

    const result = {
      htmlOverflowY: getComputedStyle(document.documentElement).overflowY,
      bodyOverflowY: getComputedStyle(document.body).overflowY,
      rootOverflowY: getComputedStyle(root).overflowY,
      deskOverflowY: getComputedStyle(deskBody).overflowY,
      documentScrollTop: document.documentElement.scrollTop,
      bodyScrollTop: document.body.scrollTop,
      rootScrollTop: root.scrollTop,
      deskScrollTop: deskBody.scrollTop,
      deskClientHeight: deskBody.clientHeight,
      deskScrollHeight: deskBody.scrollHeight,
    };

    filler.remove();
    return result;
  });

  expect(layout).not.toBeNull();
  expect(layout?.htmlOverflowY).toBe('hidden');
  expect(layout?.bodyOverflowY).toBe('hidden');
  expect(layout?.rootOverflowY).toBe('hidden');
  expect(layout?.deskOverflowY).toBe('auto');
  expect(layout?.documentScrollTop).toBe(0);
  expect(layout?.bodyScrollTop).toBe(0);
  expect(layout?.rootScrollTop).toBe(0);
  expect(layout?.deskScrollHeight).toBeGreaterThan(layout?.deskClientHeight ?? 0);
  expect(layout?.deskScrollTop).toBeGreaterThan(0);
});

test('keyboard row traversal updates the active workspace locally', async ({ page }) => {
  test.setTimeout(120000);
  await startConsultantDesk(page);
  const interactiveRows = page.locator('.socratic-map .socratic-map-row:not(:disabled)');
  await expect(interactiveRows).toHaveCount(await interactiveRows.count());
  const rowCount = await interactiveRows.count();
  expect(rowCount).toBeGreaterThan(2);

  const startRow = interactiveRows.first();
  const targetRow = interactiveRows.nth(1);
  const targetCategoryId = await targetRow.getAttribute('data-category-id');
  const targetLabel = ((await targetRow.locator('.socratic-map-row__label').textContent()) ?? '').trim();
  await startRow.focus();
  await page.keyboard.press('ArrowDown');

  await expect(page.locator('.socratic-map .socratic-map-row[aria-current="true"]').first()).toHaveAttribute(
    'data-category-id',
    targetCategoryId ?? '',
  );
  await expect(page.locator('.socratic-desk__title')).toHaveText(targetLabel);
  await expect(page.locator('.socratic-desk__body [data-category-id]')).toHaveCount(1);
});

test('switching threads resets the right-desk scroll position intentionally', async ({ page }) => {
  test.setTimeout(120000);
  await startConsultantDesk(page);

  const interactiveRows = page.locator('.socratic-map .socratic-map-row:not(:disabled)');
  const rowCount = await interactiveRows.count();
  expect(rowCount).toBeGreaterThan(1);

  const deskBody = page.locator('.socratic-desk__body');
  await deskBody.evaluate((element) => {
    const filler = document.createElement('div');
    filler.setAttribute('data-scroll-reset-probe', 'true');
    filler.style.height = '2400px';
    element.appendChild(filler);
    element.scrollTop = 420;
  });

  await expect.poll(async () => deskBody.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
  await interactiveRows.nth(1).click();
  await expect.poll(async () => deskBody.evaluate((element) => element.scrollTop)).toBe(0);
});
