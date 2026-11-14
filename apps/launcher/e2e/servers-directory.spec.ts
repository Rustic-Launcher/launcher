import type { Page } from '@playwright/test';
import { expect, seed, test } from './fixtures';

/** Where the review screenshots go (`E2E_SHOTS`), when set. */
const SHOTS = process.env.E2E_SHOTS;
async function shot(page: Page, name: string) {
  if (!SHOTS) return;
  await page.waitForTimeout(600);
  await page.screenshot({ path: `${SHOTS}/${name}.png` });
}

test.describe('Servers: Mine, Discover, Browse and a server page (ADR 0158)', () => {
  test('Discover shows staff picks in the carousel and themed rows; See all opens Browse', async ({
    app,
  }) => {
    await app.getByTestId('rail-servers').click();
    // The player has servers: My servers opens first (Discover when the list is empty).
    await expect(app.getByTestId('servers-tab-mine')).toHaveAttribute('aria-selected', 'true');
    await app.getByTestId('servers-tab-discover').click();
    await expect(app.getByTestId('servers-tab-discover')).toHaveAttribute('aria-selected', 'true');
    const hero = app.getByTestId('servers-hero');
    await expect(hero).toBeVisible();
    // Staff picks lead the carousel.
    await expect(hero.getByRole('heading', { name: 'Arena Nord' })).toBeVisible();
    await expect(hero.getByRole('button', { name: 'Play on Arena Nord' })).toBeVisible();
    await expect(app.getByTestId('servers-row-trending')).toBeVisible();
    await expect(app.getByTestId('servers-row-players')).toBeVisible();
    await expect(app.getByTestId('servers-row-new')).toBeVisible();
    await expect(app.getByTestId('servers-row-recent')).toContainText('Les Copains SMP');
    await expect(
      app.getByTestId('servers-row-tag-skyblock').getByTestId('listing-card-skyreach'),
    ).toBeVisible();
    await shot(app, 'launcher-servers-discover');

    const pvp = app.getByTestId('servers-row-tag-pvp');
    await pvp.scrollIntoViewIfNeeded();
    await pvp.getByTestId('servers-row-tag-pvp-see-all').click();
    await expect(app.getByTestId('servers-tab-browse')).toHaveAttribute('aria-selected', 'true');
    await expect(app.getByTestId('servers-active-filters')).toContainText('PvP');
    await expect(app.getByTestId('listing-card-arena-nord')).toBeVisible();
    await expect(app.getByTestId('listing-card-skyreach')).toHaveCount(0);
  });

  test('Browse filters, sorts, searches and switches to the dense list', async ({ app }) => {
    await app.goto('/#/servers?tab=browse');
    await expect(app.getByTestId('servers-browse-grid')).toBeVisible();
    await expect(app.getByText('12 results')).toBeVisible();
    await app.getByTestId('servers-filters').click();
    await app.getByTestId('servers-lang-fr').click();
    await app.getByTestId('servers-engine-horizon').click();
    await app.keyboard.press('Escape');
    await expect(app.getByTestId('servers-active-filters')).toContainText('French');
    await expect(app.getByTestId('listing-card-terres-sauvages')).toBeVisible();
    await expect(app.getByTestId('listing-card-arena-nord')).toHaveCount(0);
    await app.getByRole('button', { name: 'Remove the filter French' }).click();
    await app.getByRole('button', { name: 'Remove the filter Vanilla Horizon' }).click();
    await expect(app.getByText('12 results')).toBeVisible();

    // The dense list, sorted by players.
    await app.getByRole('radio', { name: 'List' }).click();
    const list = app.getByTestId('servers-browse-list');
    await expect(list).toBeVisible();
    await app.getByRole('combobox', { name: 'Sort' }).click();
    await app.getByRole('option', { name: 'Most players' }).click();
    await expect(list.locator('li').first()).toContainText('BlockParty');
    await expect(app).toHaveURL(/sort=players/);
    await shot(app, 'launcher-servers-browse');

    await app.getByTestId('servers-search').fill('skyblock');
    await expect(app.getByTestId('listing-row-skyreach')).toBeVisible();
    await expect(app.getByTestId('listing-row-arena-nord')).toHaveCount(0);
  });

  test('a server page: live status, add to the list, play, report', async ({ app }) => {
    await app.goto('/#/servers?tab=browse');
    await app
      .getByTestId('listing-card-mythos-rp')
      .getByRole('link', { name: 'Mythos RP' })
      .click();
    await expect(app).toHaveURL(/#\/servers\/listing\/mythos-rp/);
    const page = app.getByTestId('listing-page');
    await expect(page.getByTestId('listing-name')).toHaveText('Mythos RP');
    await expect(page.getByTestId('listing-address')).toHaveText('mythos-rp.example.org');
    await expect(page.getByTestId('listing-status')).toContainText('ms');
    await expect(page.getByTestId('listing-description')).toContainText('Fair play');
    await shot(app, 'launcher-servers-detail');

    await page.getByTestId('listing-save').click();
    await expect(page.getByTestId('listing-save')).toHaveText('In your list');
    await page.getByTestId('listing-report').click();
    const dialog = app.getByRole('dialog');
    await expect(dialog).toContainText('Report Mythos RP');
    await dialog.getByTestId('listing-report-send').click();
    await expect(
      app.getByText('Thank you, your report went to the moderation team.', { exact: true }),
    ).toBeVisible();

    await page.getByTestId('listing-play').click();
    await expect(app.getByTestId('running-pill')).not.toContainText('0 instances', {
      timeout: 15_000,
    });
    // It is in the multiplayer list now (Mine).
    await app.goto('/#/servers?tab=mine');
    await expect(app.getByTestId('mp-others')).toContainText('Mythos RP');
  });

  test('Play from a card adds the server to the list and joins', async ({ app }) => {
    await app.goto('/#/servers?tab=browse&q=parkour');
    await app.getByTestId('listing-play-parkour-paradise').click();
    await expect(app.getByTestId('running-pill')).not.toContainText('0 instances', {
      timeout: 15_000,
    });
    await app.goto('/#/servers?tab=mine');
    await expect(app.getByTestId('mp-others')).toContainText('Parkour Paradise');
  });

  test('Mine shows the multiplayer list and the hosted servers as cards', async ({ app }) => {
    await app.goto('/#/servers?tab=mine');
    await expect(app.getByTestId('servers-tab-mine')).toHaveAttribute('aria-selected', 'true');
    await expect(app.getByTestId('mp-favorites').getByTestId('mp-mp_copains')).toBeVisible();
    await expect(app.getByTestId('mp-mp_arena')).toContainText('Ranked duels');
    await expect(app.getByTestId('server-card-srv_copains')).toBeVisible();
    await shot(app, 'launcher-servers-mine');
  });

  test('rustic://server links open the page; bad ones are ignored', async ({ app }) => {
    expect(await app.evaluate(() => window.rusticMockDeepLink?.('rustic://server/../admin'))).toBe(
      false,
    );
    expect(
      await app.evaluate(() => window.rusticMockDeepLink?.('rustic://server/skyreach?join=1')),
    ).toBe(false);
    await expect(app).not.toHaveURL(/listing/);
    expect(await app.evaluate(() => window.rusticMockDeepLink?.('rustic://server/skyreach'))).toBe(
      true,
    );
    await expect(app.getByTestId('listing-name')).toHaveText('Skyreach');
    // Opening never joins: nothing runs.
    await expect(app.getByTestId('running-pill')).toContainText('0 instances');
    // An unknown server says it is not available.
    expect(
      await app.evaluate(() => window.rusticMockDeepLink?.('rustic://server/no-such-server')),
    ).toBe(true);
    await expect(app.getByTestId('listing-missing')).toContainText('This server is not available');
  });

  test('a server link that started the launcher opens its page (French)', async ({ page }) => {
    await seed(page, { language: 'fr' });
    await page.addInitScript(() => {
      if (window.sessionStorage.getItem('rx-sl')) return;
      window.sessionStorage.setItem('rx-sl', '1');
      window.localStorage.setItem('rustic.mock.deeplink', 'rustic://server/arena-nord');
    });
    await page.goto('/');
    await expect(page.getByTestId('listing-name')).toHaveText('Arena Nord', { timeout: 30_000 });
    await expect(page.getByTestId('listing-play')).toHaveText('Jouer');
    await expect(page.getByTestId('listing-description')).toContainText('Weekly tournaments');
  });
});
