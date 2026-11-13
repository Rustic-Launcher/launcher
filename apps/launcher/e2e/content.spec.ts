import { expect, test } from './fixtures';

test.describe('Content', () => {
  test('install a mod into the active instance', async ({ app }) => {
    await app.getByTestId('rail-content').click();
    await expect(app.getByTestId('breadcrumb')).toContainText('Explore/Mods');
    // Explore opens on Discover; typing a search moves to Browse.
    await expect(app.getByTestId('explore-tab-discover')).toHaveAttribute('aria-selected', 'true');
    await app.getByTestId('content-search').fill('shulker');
    await expect(app.getByTestId('explore-tab-browse')).toHaveAttribute('aria-selected', 'true');
    // The compatibility filter is on by default.
    await app.getByTestId('content-filters').click();
    await expect(app.getByTestId('compatible-only')).toHaveAttribute('aria-checked', 'true');
    await app.keyboard.press('Escape');

    const row = app.getByTestId('content-row-p_shulker');
    await expect(row).toBeVisible();
    await expect(row.getByText('Builds on your Horizon')).toBeVisible();

    await row.getByRole('button', { name: /Install Shulker Peek into Survie entre amis/ }).click();
    await expect(row.getByRole('button', { name: 'Installed' })).toBeVisible({ timeout: 15_000 });

    // It shows up in the instance's content list.
    await app.goto('/#/instances/inst_survival?tab=content');
    await expect(app.getByText('Shulker Peek').first()).toBeVisible({ timeout: 15_000 });
  });

  test('incompatible and unfair content is hidden by default and explained when shown', async ({
    app,
  }) => {
    await app.goto('/#/content?kind=mod');
    await app.getByTestId('content-search').fill('cogwork');
    await expect(app.getByText('Nothing matches')).toBeVisible();
    await app.getByRole('button', { name: 'Show incompatible results' }).click();
    const row = app.getByTestId('content-row-p_create');
    await expect(row.getByText('Incompatible: Java Edition only')).toBeVisible();
    await expect(row.getByRole('button', { name: 'Install', exact: true })).toHaveAttribute(
      'aria-disabled',
      'true',
    );
  });

  test('plugins can only target Rustic Server servers', async ({ app }) => {
    await app.goto('/#/content?kind=plugin&tab=browse');
    const picker = app.getByRole('combobox', { name: 'Installing into' });
    await expect(picker).toContainText('Les Copains SMP');
    await picker.click();
    const options = app.getByRole('option');
    await expect(options.filter({ hasText: 'Les Copains SMP' })).toBeVisible();
    await expect(options.filter({ hasText: 'Survie publique' })).toBeVisible();
    // A vanilla Horizon server and game instances are never offered for plugins.
    await expect(options.filter({ hasText: 'Creative test' })).toHaveCount(0);
    await expect(options.filter({ hasText: 'Survie entre amis' })).toHaveCount(0);
    await app.keyboard.press('Escape');

    const row = app.getByTestId('content-row-pl_claims');
    await row.getByRole('button', { name: /Install Land Claims/ }).click();
    await expect(row.getByRole('button', { name: 'Installed' })).toBeVisible({ timeout: 15_000 });
  });

  test('datapacks install into a chosen world', async ({ app }) => {
    await app.goto('/#/content?kind=datapack&target=instance:inst_survival&tab=browse');
    const row = app.getByTestId('content-row-p_dp_graves');
    await row.getByRole('button', { name: 'Install in a world' }).click();
    await app.getByRole('button', { name: /Royaume du Nord/ }).click();
    await expect(row.getByRole('button', { name: 'Installed' })).toBeVisible({ timeout: 15_000 });
  });

  test('project detail shows versions, dependencies and installs', async ({ app }) => {
    await app.goto('/#/content/p_storage?target=instance:inst_pvp');
    await expect(app.getByRole('heading', { name: 'Crate & Barrel Storage' })).toBeVisible();
    await app.getByRole('tab', { name: /Dependencies/ }).click();
    await expect(app.getByText('Installed automatically')).toBeVisible();
    await app.getByRole('tab', { name: /Versions/ }).click();
    await expect(app.getByRole('table')).toBeVisible();
    await expect(app.getByRole('table').getByText('Release').first()).toBeVisible();
    await app
      .getByRole('button', { name: /Install Crate & Barrel Storage into PvP Practice/ })
      .first()
      .click();
    await expect(app.getByRole('button', { name: 'Installed' }).first()).toBeVisible({
      timeout: 15_000,
    });
  });
});
