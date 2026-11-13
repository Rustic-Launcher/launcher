import { expect, seed, test } from './fixtures';
import type { Page } from '@playwright/test';

/** The Profiles page (ADR 0159): cards, search, filters, layouts, quick actions, states. */

const cards = (app: Page) => app.locator('[data-testid^="instance-card-"]');

test.describe('profiles page', () => {
  test('shows every profile as a card with its build, state and native mods', async ({ app }) => {
    await app.getByTestId('rail-instances').click();
    await expect(cards(app)).toHaveCount(4);
    await expect(app.getByTestId('profiles-summary')).toContainText('4 profiles');
    // The active profile (pinned) is marked; builds are stamped on the covers.
    await expect(app.getByTestId('badge-active-inst_survival')).toHaveText('Active');
    await expect(app.getByTestId('badge-update-inst_survival')).toContainText('Update 0.2.15');
    await expect(app.getByTestId('version-inst_pvp')).toContainText('Horizon 0.2.15');
    await expect(app.getByTestId('version-inst_pvp')).toContainText('Rustic edition');
    await expect(app.getByTestId('badge-native-inst_essentials')).toHaveText(
      'Native: not built yet',
    );
    await expect(app.getByTestId('badge-native-inst_pvp')).toHaveCount(0);
    // "Jump back in" with the last server to rejoin, and the create tile closing the grid.
    const recent = app.getByTestId('profiles-recent');
    await expect(recent.getByTestId('instance-recent-inst_survival')).toContainText(
      'Last on Les Copains SMP',
    );
    await expect(recent.getByTestId('recent-rejoin-inst_pvp')).toBeVisible();
    await expect(app.getByTestId('create-profile-tile')).toBeVisible();
    // The card's title opens the profile.
    await app.getByRole('link', { name: 'Creative Builds' }).last().click();
    await expect(app.getByRole('heading', { name: 'Creative Builds', level: 1 })).toBeVisible();
    await expect(app.getByTestId('instance-header')).toBeVisible();
  });

  test('search, source, sort and filters live in the URL', async ({ app }) => {
    await app.goto('/#/instances');
    const search = app.getByTestId('profiles-search');
    await search.fill('pvp');
    await expect(cards(app)).toHaveCount(1);
    await expect(app.getByTestId('instance-card-inst_pvp')).toBeVisible();
    await expect(app.getByTestId('profiles-recent')).toHaveCount(0);
    await expect(app).toHaveURL(/q=pvp/);
    await search.press('Escape');
    await expect(cards(app)).toHaveCount(4);

    await app.getByTestId('profiles-source').click();
    await app.getByTestId('profiles-source-modpack').click();
    await expect(cards(app)).toHaveCount(1);
    await expect(app.getByTestId('instance-card-inst_essentials')).toBeVisible();
    await app.getByTestId('profiles-source').click();
    await app.getByTestId('profiles-source-all').click();
    await expect(cards(app)).toHaveCount(4);

    await app.getByTestId('profiles-filters').click();
    await app.getByTestId('profiles-native-only').click();
    await expect(cards(app)).toHaveCount(1);
    await expect(app.getByTestId('instance-card-inst_essentials')).toBeVisible();
    await app.getByTestId('profiles-native-only').click();
    await app.getByTestId('profiles-version-v0.2.13').click();
    await app.keyboard.press('Escape');
    await expect(cards(app)).toHaveCount(1);
    await expect(app.getByTestId('instance-card-inst_creative')).toBeVisible();
    await expect(app.getByTestId('profiles-filters')).toHaveText('Filters · 1');
    // The filter survives a reload (URL), then its chip removes it.
    await app.reload();
    await expect(cards(app)).toHaveCount(1);
    await app.getByRole('button', { name: 'Remove the filter: Horizon build 0.2.13' }).click();
    await expect(cards(app)).toHaveCount(4);

    // Sort by name: the pinned profile first, then alphabetical.
    await app.getByRole('combobox', { name: 'Sort by' }).click();
    await app.getByRole('option', { name: 'Name' }).click();
    await expect(app).toHaveURL(/sort=name/);
    await expect(cards(app).nth(1)).toHaveAttribute('data-testid', 'instance-card-inst_creative');

    await search.fill('nothing like this');
    await expect(app.getByTestId('profiles-no-match')).toBeVisible();
    await app
      .getByTestId('profiles-no-match')
      .getByRole('button', { name: 'Reset filters' })
      .click();
    await expect(cards(app)).toHaveCount(4);
  });

  test('the list view is remembered', async ({ app }) => {
    await app.goto('/#/instances');
    await app.getByRole('radio', { name: 'List' }).click();
    await expect(app.getByTestId('profiles-list')).toBeVisible();
    const row = app.getByTestId('instance-row-inst_essentials');
    await expect(row).toContainText('Horizon 0.2.15');
    await expect(row).toContainText('32 mods · 3 packs · 2 shaders');
    await expect(row.getByTestId('badge-native-inst_essentials')).toBeVisible();
    await app.reload();
    await expect(app.getByTestId('profiles-list')).toBeVisible();
    await app.getByRole('radio', { name: 'Grid' }).click();
    await expect(app.getByTestId('profiles-grid')).toBeVisible();
  });

  test('quick actions: duplicate, use on the Launchpad, edit, delete', async ({ app }) => {
    await app.goto('/#/instances');
    const pvp = app.getByTestId('instance-card-inst_pvp');
    await pvp.hover();
    await app.getByTestId('instance-menu-inst_pvp').click();
    await app.getByRole('menuitem', { name: 'Duplicate' }).click();
    const copy = cards(app).filter({ hasText: 'PvP Practice (2)' });
    await expect(copy).toHaveCount(1);

    await pvp.hover();
    await app.getByTestId('instance-menu-inst_pvp').click();
    await app.getByRole('menuitem', { name: 'Use on the Launchpad' }).click();
    await expect(app.getByTestId('badge-active-inst_pvp')).toBeVisible();
    await expect(app.getByTestId('badge-active-inst_survival')).toHaveCount(0);

    await pvp.hover();
    await app.getByTestId('instance-menu-inst_pvp').click();
    await app.getByRole('menuitem', { name: 'Edit settings' }).click();
    await expect(app).toHaveURL(/instances\/inst_pvp\?tab=settings/);
    await expect(app.getByTestId('tab-settings')).toHaveAttribute('aria-selected', 'true');

    await app.goBack();
    await copy.hover();
    await copy.locator('[data-testid^="instance-menu-"]').click();
    await app.getByRole('menuitem', { name: 'Delete' }).click();
    await app.getByTestId('confirm-delete-instance').click();
    await expect(copy).toHaveCount(0);
    await expect(cards(app)).toHaveCount(4);
  });

  test('Play and the menu show on keyboard focus', async ({ app }) => {
    await app.goto('/#/instances');
    const play = app.getByTestId('play-inst_creative');
    await expect(play).toHaveCSS('opacity', '0');
    await app.getByRole('link', { name: 'Creative Builds' }).last().focus();
    await app.keyboard.press('Shift+Tab'); // the "..." menu
    await expect(app.getByTestId('instance-menu-inst_creative')).toBeFocused();
    await app.keyboard.press('Shift+Tab');
    await expect(play).toBeFocused();
    await expect(play).toHaveCSS('opacity', '1');
    await expect(play).toHaveAccessibleName('Update — Creative Builds');
  });

  test('Create and Import open the sheet on their tab', async ({ app }) => {
    await app.goto('/#/instances');
    await app.getByTestId('import-profile').click();
    const dialog = app.getByRole('dialog', { name: 'New instance' });
    await expect(dialog.getByTestId('new-tab-import')).toHaveAttribute('aria-selected', 'true');
    await expect(dialog.getByTestId('import-mrpack')).toBeVisible();
    await app.keyboard.press('Escape');
    await expect(dialog).toBeHidden();
    await app
      .getByTestId('create-profile-tile')
      .getByRole('button', { name: 'Create profile' })
      .click();
    await expect(dialog.getByTestId('new-tab-create')).toHaveAttribute('aria-selected', 'true');
    await expect(dialog.getByTestId('new-instance-name')).toBeVisible();
  });
});

test('profiles: empty state offers Create and Import', async ({ page }) => {
  await seed(page);
  await page.addInitScript(() => window.localStorage.setItem('rustic.mock.no-instances', 'true'));
  await page.goto('/#/instances');
  const empty = page.getByTestId('profiles-empty');
  await expect(empty).toContainText('No profile yet', { timeout: 30_000 });
  await expect(page.getByTestId('profiles-recent')).toHaveCount(0);
  await empty.getByRole('button', { name: 'Import' }).click();
  await expect(page.getByRole('dialog', { name: 'New instance' })).toBeVisible();
});

test('profiles: in French', async ({ page }) => {
  await seed(page, { language: 'fr' });
  await page.goto('/#/instances');
  await expect(page.getByRole('heading', { level: 1 })).toContainText('Vos profils', {
    timeout: 30_000,
  });
  await expect(page.getByTestId('new-instance')).toContainText('Créer un profil');
  await expect(page.getByTestId('profiles-recent')).toContainText('Reprendre');
  await expect(page.getByTestId('badge-active-inst_survival')).toHaveText('Actif');
  await expect(page.getByTestId('badge-native-inst_essentials')).toHaveText(
    'Natif : pas encore compilé',
  );
});
