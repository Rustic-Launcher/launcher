import type { Page } from '@playwright/test';
import { expect, seed, test } from './fixtures';

/** Starts the app with the update service in a mock scenario (src/ipc/mock/handlers/updates.ts). */
async function boot(page: Page, scenario: string) {
  await seed(page);
  await page.addInitScript((s) => {
    window.localStorage.setItem('rustic.mock.update', JSON.stringify(s));
  }, scenario);
  await page.goto('/');
  await expect(page.getByTestId('rail-home')).toBeVisible({ timeout: 30_000 });
}

test.describe('automatic updates (ADR 0162)', () => {
  test('up to date: no pill, the card shows versions and checks on demand', async ({ app }) => {
    await expect(app.getByTestId('update-pill')).toBeHidden();
    await app.goto('/#/settings?section=launcher');
    const card = app.getByTestId('updates-card');
    await expect(card.getByTestId('launcher-version')).toHaveText('0.2.0');
    await expect(card.getByText('Stable')).toBeVisible();
    await expect(card.getByTestId('edition-version')).toHaveText(
      'rustic-edition-2026-11-05-a84f637',
    );
    await expect(card.getByText(/^Last checked/)).toBeVisible();
    await card.getByRole('button', { name: 'Check for updates' }).click();
    await expect(app.getByText('You are up to date').first()).toBeVisible();
  });

  test('automatic updates off: "Update and restart" installs and restarts', async ({ page }) => {
    await boot(page, 'available');
    const pill = page.getByTestId('update-pill');
    await expect(pill).toHaveText('Update and restart');
    await page.goto('/#/settings?section=launcher');
    const card = page.getByTestId('updates-card');
    await expect(card.getByText('Version 0.2.1 available')).toBeVisible();
    await expect(
      card.getByText('Version rustic-edition-2026-11-12-4c1e2a9 available'),
    ).toBeVisible();
    await card.getByTestId('update-apply').click();
    await expect(page.getByText('Downloading Rustic Launcher 0.2.1…').first()).toBeVisible();
    await expect(
      page.getByText('Rustic Launcher restarts to finish the update.').first(),
    ).toBeVisible();
    await expect(card.getByTestId('launcher-version')).toHaveText('0.2.1', { timeout: 15_000 });
    await expect(pill).toBeHidden();
  });

  test('a running game holds the update back', async ({ page }) => {
    await boot(page, 'deferred');
    await expect(page.getByTestId('update-pill')).toHaveText('Update after your game');
    await page.goto('/#/settings?section=about');
    await expect(page.getByText('Waiting for the game to close')).toBeVisible();
  });

  test('a download in progress shows in the title bar', async ({ page }) => {
    await boot(page, 'downloading');
    const pill = page.getByTestId('update-pill');
    await expect(pill).toContainText('Updating 62%');
    await expect(pill.getByRole('progressbar')).toBeVisible();
  });

  test('after the restart: "updated" and the interrupted launch resumes', async ({ page }) => {
    await boot(page, 'updated');
    await expect(page.getByText('Rustic Launcher updated to 0.2.0').first()).toBeVisible();
    await expect(page.getByText('Update done: launching your game').first()).toBeVisible();
    await expect(page.getByTestId('running-pill')).toContainText('1 instance running', {
      timeout: 20_000,
    });
  });

  test('a failed check never shows in the title bar, Settings tells', async ({ page }) => {
    await boot(page, 'error');
    await expect(page.getByTestId('update-pill')).toBeHidden();
    await page.goto('/#/settings?section=launcher');
    await expect(
      page.getByText('The last attempt failed. It will be retried automatically.'),
    ).toBeVisible();
  });
});
