import { test as base, expect, type Page } from '@playwright/test';

export interface MockOptions {
  /** Skip the first-launch flow (default true). */
  onboarded?: boolean;
  /** Mock latency in ms (default 40). */
  latency?: number;
  /** Speed multiplier for simulated downloads/installs (default 6). */
  speed?: number;
  /** A game locale (`de_de`), or a short language of an older setting (`fr`, migrated). */
  language?: string;
  /** A new machine: no account, game files not prepared (default false). */
  fresh?: boolean;
}

/** Seeds localStorage before the app boots so the mock starts in a known state. */
export async function seed(page: Page, o: MockOptions = {}) {
  await page.addInitScript(
    ({ onboarded, latency, speed, language, fresh }) => {
      if (window.sessionStorage.getItem('rx-seeded')) return;
      window.sessionStorage.setItem('rx-seeded', '1');
      window.localStorage.clear();
      window.localStorage.setItem('rustic.mock.onboarded', JSON.stringify(onboarded));
      window.localStorage.setItem('rustic.mock.latency', String(latency));
      window.localStorage.setItem('rustic.mock.speed', String(speed));
      window.localStorage.setItem('rustic.mock.fresh', JSON.stringify(fresh));
      window.localStorage.setItem(
        'rustic.prefs',
        JSON.stringify({
          state: { language, theme: 'dark', palette: 'ember', friendsPanelOpen: true },
          version: 1,
        }),
      );
    },
    {
      onboarded: o.onboarded ?? true,
      latency: o.latency ?? 40,
      speed: o.speed ?? 6,
      language: o.language ?? 'en',
      fresh: o.fresh ?? false,
    },
  );
}

export const test = base.extend<{ app: Page }>({
  app: async ({ page }, use) => {
    await seed(page);
    await page.goto('/');
    await expect(page.getByTestId('rail-home')).toBeVisible({ timeout: 30_000 });
    await use(page);
  },
});

export { expect };

/** A game is running: the title bar's running pill counts it. */
export async function expectRunning(page: Page, count = 1) {
  await expect(page.getByTestId('running-pill')).toContainText(
    `${count} instance${count === 1 ? '' : 's'} running`,
    { timeout: 20_000 },
  );
}
