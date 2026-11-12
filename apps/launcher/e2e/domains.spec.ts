import type { Page } from '@playwright/test';
import { expect, test } from './fixtures';

/** Clicks "Check now" until the domain is connected (the mock progresses per check). */
async function checkUntilConnected(page: Page) {
  const connected = page.getByTestId('domain-connected');
  for (let i = 0; i < 10; i++) {
    if (await connected.isVisible()) return;
    const btn = page.getByTestId('check-now');
    if (await btn.isEnabled().catch(() => false)) await btn.click();
    await page.waitForTimeout(700);
  }
  await expect(connected).toBeVisible();
}

test.describe('domains', () => {
  test('buy a domain at Porkbun and configure it automatically', async ({ app }) => {
    await app.goto('/#/servers/srv_copains?tab=access');
    await app.getByTestId('path-buy').click();

    // Connect the user's own Porkbun account first.
    await app.getByTestId('porkbun-api-key').fill('pk1_e2etestkey0001');
    await app.getByTestId('porkbun-secret').fill('sk1_e2etestsecret01');
    await app.getByTestId('porkbun-connect-submit').click();

    await app.getByTestId('domain-search').fill('loucraft');
    await app.getByTestId('domain-search-submit').click();
    const row = app.getByTestId('domain-loucraft.net');
    await expect(row).toContainText('$11.08');
    await expect(app.getByTestId('domain-loucraft.com')).toContainText('Taken');
    await app.getByTestId('select-loucraft.net').click();

    // Explicit purchase confirmation: Porkbun is the seller, two unticked boxes.
    const dialog = app.getByRole('dialog', { name: 'Confirm purchase at Porkbun' });
    await expect(dialog).toContainText('Porkbun LLC, not Rustic');
    await expect(dialog.getByTestId('buy-price')).toContainText('$11.08');
    const confirm = dialog.getByTestId('buy-confirm');
    await expect(confirm).toBeDisabled();
    await dialog.getByTestId('agree-terms').click();
    await expect(confirm).toBeDisabled();
    await dialog.getByTestId('agree-order').click();
    await expect(confirm).toBeEnabled();
    await confirm.click();

    await expect(app.getByText('loucraft.net is yours', { exact: true })).toBeVisible();
    await expect(app.getByTestId('verification-panel')).toBeVisible();
    await checkUntilConnected(app);
    await expect(app.getByTestId('domain-connected')).toContainText('play.loucraft.net');
    await expect(app.getByTestId('server-address')).toHaveText('play.loucraft.net');
  });

  test('connect Porkbun and configure an owned domain', async ({ app }) => {
    await app.goto('/#/servers/srv_copains?tab=access');
    await app.getByTestId('path-porkbun').click();
    await app.getByTestId('porkbun-api-key').fill('pk1_e2etestkey0002');
    await app.getByTestId('porkbun-secret').fill('sk1_e2etestsecret02');
    await app.getByTestId('porkbun-connect-submit').click();
    await app.getByRole('combobox', { name: 'Domain' }).click();
    await app.getByRole('option', { name: 'lescopains.gg' }).click();
    await expect(app.getByTestId('records-table')).toContainText('_rustic-challenge.play');
    await app.getByTestId('porkbun-apply').click();
    await checkUntilConnected(app);
    await expect(app.getByTestId('domain-connected')).toContainText('play.lescopains.gg');
  });

  test('use my own domain from any registrar with live verification', async ({ app, context }) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    await app.goto('/#/servers/srv_copains?tab=access');
    await app.getByTestId('path-manual').click();
    await app.getByTestId('manual-domain').fill('loucraft.net');
    await app.getByRole('combobox', { name: 'Registrar' }).click();
    await app.getByRole('option', { name: 'OVHcloud' }).click();
    await app.getByTestId('manual-show-records').click();

    await expect(app.getByTestId('registrar-guide')).toContainText('DNS zone');
    const table = app.getByTestId('records-table');
    await expect(table.getByTestId('record-CNAME')).toContainText('play');
    await expect(table.getByTestId('record-TXT')).toContainText('rustic-verify=');
    await expect(table.getByTestId('record-SRV')).toContainText('optional');

    // Copy the CNAME target.
    await table.getByRole('button', { name: 'Copy the CNAME value' }).click();
    await expect
      .poll(() => app.evaluate(() => navigator.clipboard.readText()))
      .toMatch(/\.relay\.example\.invalid\.$/);

    // First check: CNAME still has the old value, TXT missing.
    await expect(table.getByTestId('record-CNAME')).toContainText('Wrong value');
    await expect(table.getByTestId('record-CNAME')).toContainText('Found:');
    await expect(table.getByTestId('record-TXT')).toContainText('Missing');
    await checkUntilConnected(app);
    await expect(app.getByTestId('domain-connected')).toContainText('Domain connected');
    await expect(table.getByTestId('record-SRV')).toContainText('Not needed');
  });
});
