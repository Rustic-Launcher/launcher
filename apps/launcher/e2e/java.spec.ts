import path from 'node:path';
import type { Page } from '@playwright/test';
import { expect, seed, test } from './fixtures';

/** `RUSTIC_SHOTS_DIR=<folder>` saves the journey's screenshots there (nothing is saved otherwise). */
async function shot(page: Page, name: string) {
  const dir = process.env.RUSTIC_SHOTS_DIR;
  if (dir) await page.screenshot({ path: path.join(dir, `${name}.png`) });
}

test.describe('launchpad game switcher (ADR 0172)', () => {
  test('switches to the Java Launchpad and remembers it', async ({ app }) => {
    await expect(app.getByTestId('edition-current')).toHaveText('Rustic');
    await expect(app.getByTestId('edition-disclaimer')).toHaveText(
      'Not affiliated with Mojang or Microsoft.',
    );
    // The Rustic Launchpad stays as it was.
    await expect(app.getByTestId('play-button')).toBeVisible();
    await shot(app, 'launchpad-rustic-switcher');
    await app.getByTestId('edition-switcher').click();
    await expect(app.getByTestId('edition-bedrock')).toBeVisible();
    await shot(app, 'launchpad-switcher-open');
    await app.getByTestId('edition-java').click();
    await expect(app.getByTestId('java-launchpad')).toBeVisible();
    await expect(app.getByTestId('java-profile-java-fabric')).toContainText('Fabric 1.21.1');
    await expect(app.getByTestId('java-play-version')).toContainText('Fabric 0.20.1 · 1.21.1');
    await shot(app, 'launchpad-java');
    await app.reload();
    await expect(app.getByTestId('java-launchpad')).toBeVisible({ timeout: 30_000 });
    await expect(app.getByTestId('edition-current')).toHaveText('Minecraft: Java Edition');
  });

  test('creates a Fabric profile, launches it and reads its log', async ({ app }) => {
    await app.getByTestId('edition-switcher').click();
    await app.getByTestId('edition-java').click();
    await app.getByTestId('java-new-profile').click();
    await app.getByTestId('java-profile-name').fill('Fabric 26.3');
    await app.getByRole('combobox', { name: 'Mod loader' }).click();
    await app.getByRole('option', { name: 'Fabric' }).click();
    await expect(app.getByRole('combobox', { name: 'Fabric version' })).toBeVisible();
    await shot(app, 'java-new-profile');
    await app.getByTestId('java-profile-save').click();
    await expect(app.getByTestId('java-play-version')).toContainText('Fabric · 26.3');
    await expect(app.getByTestId('java-play')).toHaveAttribute('data-action', 'install_launch');
    await app.getByTestId('java-play').click();
    await expect(app.getByTestId('java-play')).toHaveAttribute('data-action', 'stop', {
      timeout: 20_000,
    });
    await expect(app.getByTestId('java-log')).toContainText('Setting user: SamCrafte');
    await shot(app, 'java-running-log');
    await app.getByTestId('java-tab-mods').click();
    await expect(app.getByTestId('java-hit-sodium')).toBeVisible();
    await shot(app, 'java-mods');
    await app.getByTestId('java-play').click();
    await expect(app.getByTestId('java-play')).toHaveAttribute('data-action', 'launch');
  });

  test('refuses to launch without a game-owning account', async ({ page }) => {
    await seed(page);
    // The mock's scenario: the signed-in account doesn't own the game.
    await page.goto('/?java=not-owned');
    await page.getByTestId('edition-switcher').click();
    await page.getByTestId('edition-java').click();
    await page.getByTestId('java-play').click();
    await expect(
      page.getByText("This Microsoft account doesn't own Minecraft: Java Edition."),
    ).toBeVisible();
  });

  test('shows the Bedrock panel', async ({ app }) => {
    await app.getByTestId('edition-switcher').click();
    await app.getByTestId('edition-bedrock').click();
    await expect(app.getByTestId('bedrock-play')).toHaveText('Play Bedrock Edition');
    await shot(app, 'launchpad-bedrock');
  });
});
