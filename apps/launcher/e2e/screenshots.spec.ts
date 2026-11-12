/**
 * Captures docs/screenshots/*.png (dark, Ember) and docs/screenshots/navigation.webm.
 * Run: pnpm screenshots (project "screenshots").
 */
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { expect, seed, test } from './fixtures';
import type { Page } from '@playwright/test';

const OUT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../docs/screenshots');

async function settle(page: Page, ms = 900) {
  await page.waitForLoadState('networkidle');
  await expect(page.locator('[aria-busy="true"]')).toHaveCount(0, { timeout: 15_000 });
  await page.waitForTimeout(ms);
}

async function shot(page: Page, name: string, hash: string, ms?: number) {
  await page.goto(`/#${hash}`);
  await settle(page, ms);
  await page.screenshot({ path: path.join(OUT, `${name}.png`) });
}

test.describe.configure({ mode: 'serial' });

test('pages', async ({ page }) => {
  test.setTimeout(240_000);
  await seed(page, { latency: 30, speed: 4 });
  await page.goto('/');
  await expect(page.getByTestId('play-button')).toBeVisible({ timeout: 30_000 });

  await shot(page, '01-home', '/', 1500);
  await shot(page, '02-instances', '/instances');
  await shot(page, '03-instance-detail', '/instances/inst_survival');
  await shot(page, '04-instance-resource-packs', '/instances/inst_survival?tab=resourcepacks');
  await shot(page, '05-instance-settings', '/instances/inst_survival?tab=settings');
  await shot(page, '06-content-mods', '/content');
  await shot(page, '07-content-shaders', '/content?kind=shader');
  await shot(page, '08-content-plugins', '/content?kind=plugin&target=server:srv_copains');
  const firstProject = page.locator('a[href*="#/content/"]').first();
  await page.goto('/#/content');
  await settle(page);
  if (await firstProject.count()) {
    await firstProject.click();
    await settle(page);
    await page.screenshot({ path: path.join(OUT, '09-content-detail.png') });
  }
  await shot(page, '10-servers', '/servers');
  await shot(page, '11-servers-multiplayer', '/servers?tab=multiplayer');
  await shot(page, '12-server-wizard', '/servers/new');
  await shot(page, '13-server-overview', '/servers/srv_copains', 2500);
  await shot(page, '14-server-console', '/servers/srv_copains?tab=console', 2000);
  await shot(page, '15-server-players', '/servers/srv_copains?tab=players');
  await shot(page, '16-server-plugins', '/servers/srv_copains?tab=plugins');
  await shot(page, '17-server-backups', '/servers/srv_copains?tab=backups');
  await shot(page, '18-server-access', '/servers/srv_copains?tab=access');
  await shot(page, '19-server-remote-vps', '/servers/srv_vps');
  await shot(page, '20-locker', '/locker', 2000);
  await shot(page, '23-settings-accounts', '/settings?section=accounts');
  await shot(page, '24-settings-appearance', '/settings?section=appearance');
  await shot(page, '25-settings-privacy', '/settings?section=privacy');

  await page.goto('/#/');
  await settle(page, 1200);
  await page.getByTestId('open-social').click();
  await page.waitForTimeout(600);
  await page.screenshot({ path: path.join(OUT, '27-social-friends.png') });
  await page.getByTestId('social-tab-chats').click();
  await page.getByTestId('chat-lea_builds').click();
  await page.waitForTimeout(600);
  await page.screenshot({ path: path.join(OUT, '28-social-chat.png') });
  await page.keyboard.press('Escape');
  await page.goto('/#/servers?tab=mine');
  await settle(page);
  await page.screenshot({ path: path.join(OUT, '10b-servers-mine.png') });

  await page.keyboard.press('Control+k');
  await page.keyboard.type('lea');
  await page.waitForTimeout(500);
  await page.screenshot({ path: path.join(OUT, '26-command-palette.png') });
  await page.keyboard.press('Escape');
});

test('onboarding', async ({ page }) => {
  test.setTimeout(120_000);
  await seed(page, { onboarded: false, latency: 30 });
  await page.goto('/');
  await expect(page).toHaveURL(/#\/onboarding/);
  await settle(page);
  const next = page.getByTestId('onboarding-next');
  const steps = ['welcome', 'microsoft', 'rustic', 'privacy', 'game', 'ready'];
  for (const [i, step] of steps.entries()) {
    await expect(page.getByTestId(`onboarding-step-${step}`)).toBeVisible();
    if (step === 'game') {
      await expect(page.getByTestId('game-found')).toBeVisible();
      await page.getByTestId('prepare-start').click();
      await page.waitForTimeout(1200);
    }
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(OUT, `00-onboarding-${i + 1}-${step}.png`) });
    if (step !== 'ready') await next.click();
  }
});

test('flows: VPS install, domains', async ({ page, context }) => {
  test.setTimeout(240_000);
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await seed(page, { latency: 30, speed: 2 });
  await page.goto('/#/servers/new');
  await settle(page);
  await page.getByTestId('wizard-name').fill('Survie VPS');
  await page.getByTestId('wizard-eula').click();
  await page.getByTestId('wizard-next').click();
  await page.getByTestId('hosting-vps').click();
  await page.getByRole('radio', { name: 'Add a VPS' }).click();
  await page.getByTestId('vps-host').fill('203.0.113.10');
  await page.getByTestId('vps-generate-key').click();
  await page.screenshot({ path: path.join(OUT, '30-wizard-where-to-host-vps.png') });
  await page.getByTestId('vps-test').click();
  await expect(page.getByTestId('vps-fingerprint')).toContainText('SHA256:');
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(OUT, '31-vps-fingerprint.png') });
  await page.getByTestId('vps-trust').click();
  await expect(page.getByTestId('vps-preflight')).toContainText('Your VPS is ready');
  await page.screenshot({ path: path.join(OUT, '32-vps-preflight.png') });
  await page.getByTestId('wizard-next').click();
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(OUT, '33-wizard-access-summary.png') });
  await page.getByTestId('wizard-next').click();
  await expect(page.getByTestId('vps-progress')).toBeVisible();
  await page.waitForTimeout(2500);
  await page.screenshot({ path: path.join(OUT, '34-vps-install-progress.png') });
  await expect(page.getByText('Survie VPS is running on 203.0.113.10')).toBeVisible({
    timeout: 60_000,
  });
  await page
    .getByTestId('vps-progress')
    .getByRole('button', { name: /Show log/ })
    .click();
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(OUT, '35-vps-install-done.png') });

  await page.goto('/#/servers/srv_copains?tab=access');
  await settle(page);
  await page.getByTestId('path-buy').click();
  await page.getByTestId('porkbun-api-key').fill('pk1_screenshotkey01');
  await page.getByTestId('porkbun-secret').fill('sk1_screenshotsecret');
  await page.waitForTimeout(300);
  await page.screenshot({ path: path.join(OUT, '36-domain-porkbun-connect.png') });
  await page.getByTestId('porkbun-connect-submit').click();
  await page.getByTestId('domain-search').fill('loucraft');
  await page.getByTestId('domain-search-submit').click();
  await expect(page.getByTestId('domain-loucraft.net')).toBeVisible();
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(OUT, '37-domain-search.png') });
  await page.getByTestId('select-loucraft.net').click();
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(OUT, '38-domain-purchase-dialog.png') });
  await page.keyboard.press('Escape');

  await page.goto('/#/servers/srv_creative?tab=access');
  await settle(page);
  await page.getByTestId('path-manual').click();
  await page.getByTestId('manual-domain').fill('loucraft.net');
  await page.getByRole('combobox', { name: 'Registrar' }).click();
  await page.getByRole('option', { name: 'OVHcloud' }).click();
  await page.getByTestId('manual-show-records').click();
  await expect(page.getByTestId('records-table')).toBeVisible();
  await page.waitForTimeout(1500);
  await page.getByTestId('records-table').scrollIntoViewIfNeeded();
  await page.screenshot({ path: path.join(OUT, '39-domain-own-records.png') });
});

test('palettes and themes', async ({ page }) => {
  test.setTimeout(120_000);
  await seed(page, { latency: 30 });
  await page.goto('/');
  await expect(page.getByTestId('play-button')).toBeVisible({ timeout: 30_000 });
  const set = (palette: string, theme: string) =>
    page.evaluate(
      ([p, t]) => {
        const raw = JSON.parse(localStorage.getItem('rustic.prefs') ?? '{"state":{}}');
        raw.state = { ...raw.state, palette: p, theme: t };
        localStorage.setItem('rustic.prefs', JSON.stringify(raw));
      },
      [palette, theme],
    );
  for (const [palette, theme, hash, name] of [
    ['verdigris', 'dark', '/', '50-verdigris-home'],
    ['heather', 'dark', '/servers/srv_copains', '51-heather-server'],
    ['ember', 'light', '/', '52-light-home'],
    ['ember', 'light', '/content', '53-light-content'],
    ['verdigris', 'light', '/settings?section=appearance', '54-light-verdigris-settings'],
  ] as const) {
    await set(palette, theme);
    await page.goto(`/#${hash}`);
    await page.reload();
    await expect(page.getByTestId('rail-home')).toBeVisible({ timeout: 30_000 });
    await settle(page, 1200);
    await page.screenshot({ path: path.join(OUT, `${name}.png`) });
  }
  await set('ember', 'dark');
});
