import { expect, test } from './fixtures';

test.describe('server management', () => {
  test('console command, kick a player, back up, open via relay', async ({ app, context }) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    await app.goto('/#/servers/srv_copains');
    await expect(app.getByRole('heading', { level: 1, name: 'Les Copains SMP' })).toBeVisible();
    await expect(app.getByTestId('server-state')).toHaveText(/Running/);

    // Overview shows live metrics.
    await expect(app.getByTestId('metric-tps')).toBeVisible();

    // Console: send a command, see the answer.
    await app.getByTestId('server-tab-console').click();
    const input = app.getByTestId('console-input');
    await input.fill('lis');
    await expect(app.getByRole('option', { name: 'list' })).toBeVisible();
    await input.press('Tab');
    await expect(input).toHaveValue('list ');
    await input.press('Enter');
    await expect(app.getByTestId('console-log')).toContainText('players online');

    // Players: kick someone online.
    await app.getByTestId('server-tab-players').click();
    const kick = app.locator('[data-testid^="kick-"]').first();
    await expect(kick).toBeVisible();
    const name = ((await kick.getAttribute('data-testid')) ?? '').replace('kick-', '');
    await kick.click();
    await expect(app.getByText(`${name} was kicked`, { exact: true })).toBeVisible();
    await expect(app.getByTestId(`kick-${name}`)).toHaveCount(0);

    // Backups: back up now with a note.
    await app.getByTestId('server-tab-backups').click();
    await app.getByRole('textbox', { name: 'Note' }).fill('e2e snapshot');
    await app.getByTestId('backup-now').click();
    await expect(app.getByTestId('backups-list')).toContainText('e2e snapshot');
  });

  test('turn on the temporary relay address and copy it', async ({ app, context }) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    await app.goto('/#/servers/srv_creative?tab=access');
    await expect(app.getByTestId('relay-card')).toBeVisible();
    await expect(app.getByTestId('relay-hostname')).toHaveCount(0);
    await app.getByTestId('relay-label').fill('creatif-test');
    await app.getByTestId('relay-toggle').click();
    const host = app.getByTestId('relay-hostname');
    await expect(host).toHaveText('creatif-test.relay.example.invalid');
    await app
      .getByTestId('relay-card')
      .getByRole('button', { name: 'Copy the temporary address' })
      .click();
    await expect
      .poll(() => app.evaluate(() => navigator.clipboard.readText()))
      .toBe('creatif-test.relay.example.invalid');
    // The relay address becomes the server's shared address.
    await expect(app.getByTestId('server-address')).toHaveText(
      'creatif-test.relay.example.invalid',
    );
  });

  test('plugins tab explains Rustic Server on a vanilla server', async ({ app }) => {
    await app.goto('/#/servers/srv_creative?tab=plugins');
    await expect(app.getByText('Plugins need Rustic Server')).toBeVisible();
  });
});
