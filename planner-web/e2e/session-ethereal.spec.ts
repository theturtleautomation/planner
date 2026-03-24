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

async function sectionOffsetWithinDesk(page: Page, categoryId: string): Promise<number | null> {
  return page.evaluate((targetCategoryId) => {
    const deskBody = document.querySelector('.socratic-desk__body');
    const section = document.querySelector(`[data-category-id="${targetCategoryId}"]`);
    if (!(deskBody instanceof HTMLElement) || !(section instanceof HTMLElement)) {
      return null;
    }

    const deskRect = deskBody.getBoundingClientRect();
    const sectionRect = section.getBoundingClientRect();
    return sectionRect.top - deskRect.top;
  }, categoryId);
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

test('keyboard preview drives deep document jumps in the consultant desk', async ({ page }) => {
  test.setTimeout(120000);
  await startConsultantDesk(page);
  const interactiveRows = page.locator('.socratic-map .socratic-map-row:not(:disabled)');
  await expect(interactiveRows).toHaveCount(await interactiveRows.count());
  const rowCount = await interactiveRows.count();
  expect(rowCount).toBeGreaterThan(6);

  const startRow = interactiveRows.first();
  const targetRow = interactiveRows.nth(5);
  const targetLabel = await targetRow.getAttribute('aria-label');
  await startRow.focus();

  const deskBody = page.locator('.socratic-desk__body');
  const initialScrollTop = await deskBody.evaluate((element) => element.scrollTop);

  for (let index = 0; index < 5; index += 1) {
    await page.keyboard.press('ArrowDown');
  }

  await expect(page.locator('.socratic-map .socratic-map-row[aria-current="true"]').first()).toHaveAttribute('aria-label', targetLabel ?? '');
  await expect.poll(async () => deskBody.evaluate((element) => element.scrollTop)).toBeGreaterThan(initialScrollTop);
});

test('live category insertion preserves the current section anchor', async ({ page }) => {
  test.setTimeout(120000);
  await startConsultantDesk(page);

  const interactiveRows = page.locator('.socratic-map .socratic-map-row:not(:disabled)');
  const rowCount = await interactiveRows.count();
  expect(rowCount).toBeGreaterThan(6);

  const startRow = interactiveRows.first();
  const targetRow = interactiveRows.nth(5);
  const targetCategoryId = await targetRow.getAttribute('data-category-id');
  expect(targetCategoryId).toBeTruthy();

  await startRow.focus();
  for (let index = 0; index < 5; index += 1) {
    await page.keyboard.press('ArrowDown');
  }

  await expect(page.locator('.socratic-map .socratic-map-row[aria-current="true"]').first()).toHaveAttribute(
    'data-category-id',
    targetCategoryId ?? '',
  );

  await expect
    .poll(async () => sectionOffsetWithinDesk(page, targetCategoryId ?? ''), { timeout: 10000 })
    .not.toBeNull();
  const beforeOffset = await sectionOffsetWithinDesk(page, targetCategoryId ?? '');
  expect(beforeOffset).not.toBeNull();

  const insertion = await page.evaluate(async ({ categoryId }) => {
    const hook = window.__plannerSocraticDocumentTest;
    if (!hook) {
      throw new Error('Missing Socratic document test hook');
    }

    const state = hook.getState();
    const target = state.categoriesById[categoryId];
    if (!target) {
      throw new Error(`Target category ${categoryId} not found in document graph`);
    }

    const insertedCategoryId = `playwright-insert-${Date.now()}`;
    hook.hydrate({
      workspace: {
        focused_category_id: state.focusedCategoryId,
        branch_notice: null,
        category_snapshot: {
          revision: `${state.revision ?? 'playwright'}-insert-${Date.now()}`,
          root_category_ids: [insertedCategoryId],
          nodes: [{
            category_id: insertedCategoryId,
            parent_category_id: target.parentCategoryId ?? null,
            title: 'Playwright inserted thread',
            summary: 'Inserted during browser verification.',
            status: 'ready',
            depth: target.depth,
            mapped_dimensions: ['Playwright'],
            has_children: false,
            has_prompt_ready: true,
            item_count_hint: 1,
          }],
          active_category_path: state.activeCategoryPath,
          newly_available_category_ids: [insertedCategoryId],
          build_ready: state.buildReady,
          build_readiness_message: state.buildReadinessMessage,
        },
        groups: [],
      },
      currentPrompt: null,
    });

    const nextState = hook.getState();
    return {
      insertedCategoryId,
      beforeCount: state.categoryOrder.length,
      afterCount: nextState.categoryOrder.length,
    };
  }, { categoryId: targetCategoryId ?? '' });

  expect(insertion.afterCount).toBe(insertion.beforeCount + 1);
  await expect(interactiveRows).toHaveCount(rowCount + 1);
  await expect(page.locator('.socratic-map .socratic-map-row[aria-current="true"]').first()).toHaveAttribute(
    'data-category-id',
    targetCategoryId ?? '',
  );
  await expect(page.locator(`.socratic-map .socratic-map-row[data-category-id="${insertion.insertedCategoryId}"]`)).toHaveCount(1);

  await expect.poll(async () => {
    const afterOffset = await sectionOffsetWithinDesk(page, targetCategoryId ?? '');
    return (
      typeof afterOffset === 'number'
      && typeof beforeOffset === 'number'
      && Math.abs(afterOffset - beforeOffset) < 80
    );
  }, { timeout: 10000 }).toBe(true);
});
