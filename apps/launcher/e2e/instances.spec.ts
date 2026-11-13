import { expect, expectRunning, test } from './fixtures';

test.describe('instances', () => {
  test('play in one click from Home (update included)', async ({ app }) => {
    // The active (pinned) instance needs an update: PLAY shows UPDATE, one click updates then launches.
    const play = app.getByTestId('play-button');
    await expect(play).toHaveAttribute('data-state', 'update');
    await play.click();
    await expectRunning(app);
    await expect(play).toHaveAttribute('data-state', 'running');
  });

  test('play another instance in two clicks (rail + card)', async ({ app }) => {
    await app.getByTestId('rail-instances').click(); // click 1
    await app.getByTestId('play-inst_pvp').click(); // click 2
    await expectRunning(app);
    await expect(app.getByTestId('play-inst_pvp')).toHaveAttribute('data-state', 'running');
    await app.getByTestId('rail-home').click();
    await expect(app.getByTestId('instance-selector')).toContainText('PvP Practice');
  });

  test('create an instance', async ({ app }) => {
    await app.getByTestId('rail-instances').click();
    await expect(app.getByTestId('breadcrumb')).toContainText('Profiles');
    await app.getByTestId('new-instance').click();
    const dialog = app.getByRole('dialog', { name: 'New instance' });
    await expect(dialog).toBeVisible();
    // Validation: empty name.
    await dialog.getByTestId('create-instance-submit').click();
    await expect(dialog.getByText('Give the instance a name.')).toBeVisible();
    await dialog.getByTestId('new-instance-name').fill('Weekend build');
    await dialog.getByRole('radio', { name: 'Castle' }).click();
    await dialog.getByTestId('create-instance-submit').click();
    await expect(app.getByRole('heading', { name: 'Weekend build', level: 1 })).toBeVisible();
    await expect(app.getByText('Horizon 0.2.15', { exact: true })).toBeVisible();
    await app.getByTestId('rail-home').click();
    await app.getByTestId('instance-selector').click();
    await expect(app.getByRole('menuitem', { name: /Weekend build/ })).toBeVisible();
  });

  test('reorder resource packs with the keyboard', async ({ app }) => {
    await app.goto('/#/instances/inst_survival?tab=resourcepacks');
    const list = app.getByTestId('pack-order');
    await expect(list.locator('li').first()).toContainText('Clarity 32x');
    const down = app.getByRole('button', { name: 'Move Clarity 32x down' });
    await down.focus();
    await app.keyboard.press('Enter');
    await expect(list.locator('li').first()).toContainText('Soft UI');
    await expect(list.locator('li').nth(1)).toContainText('Clarity 32x');
    await expect(app.getByText('Clarity 32x moved to position 2 of 2')).toBeAttached();
    // Saved: the order survives switching tabs (refetch from the backend).
    await app.getByTestId('tab-overview').click();
    await app.getByTestId('tab-resourcepacks').click();
    await expect(app.getByTestId('pack-order').locator('li').first()).toContainText('Soft UI');
  });

  test('instance detail tabs and content toggle', async ({ app }) => {
    await app.goto('/#/instances/inst_survival?tab=content');
    const row = app.getByTestId('content-row-ic_torches');
    await expect(row).toBeVisible();
    const sw = row.getByRole('switch', { name: 'Enable Better Torches' });
    await expect(sw).toHaveAttribute('data-state', 'unchecked');
    await sw.click();
    await expect(sw).toHaveAttribute('data-state', 'checked');
    await expect(app.getByText('Incompatible: Built for Java Edition only')).toBeVisible();
    await app.getByTestId('tab-worlds').click();
    await expect(app.getByText('Royaume du Nord')).toBeVisible();
    await app.getByTestId('tab-settings').click();
    await expect(
      app.getByText('Horizon instances only pin a build', { exact: false }),
    ).toBeVisible();
  });
});
