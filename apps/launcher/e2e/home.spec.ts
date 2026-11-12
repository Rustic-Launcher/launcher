import { expect, expectRunning, test } from './fixtures';

test.describe('home', () => {
  test('renders the launchpad: greeting, launch hero, news and friends column', async ({ app }) => {
    await expect(app.getByRole('heading', { level: 1 })).toContainText('Welcome back,');
    await expect(app.getByRole('heading', { level: 1 })).toContainText('SamCrafte');
    await expect(app.getByTestId('breadcrumb')).toContainText('Launchpad');
    await expect(app.getByTestId('play-button')).toContainText('Update & launch');
    await expect(app.getByTestId('play-button')).toContainText('Horizon 0.2.14 (Rustic edition)');
    await expect(app.getByTestId('hero-continue')).toContainText('Continue: Survie entre amis');
    await expect(app.getByRole('heading', { name: /Latest news \(4 unread\)/ })).toBeVisible();
    await app.getByTestId('news-n1').click();
    await expect(app.getByRole('heading', { name: /Latest news \(3 unread\)/ })).toBeVisible();
    await expect(app.getByTestId('featured-fs2')).toContainText('Arena Nord');
    await expect(app.getByRole('heading', { name: /Friends \(5 online\)/ })).toBeVisible();
    // No docked friends panel: friends live in this column and the Social window.
    await expect(app.locator('#friends-panel')).toHaveCount(0);
  });

  test('the app shell keeps the window fixed (panes scroll, not the page)', async ({ app }) => {
    // The website scrolls normally; the launcher opts into the fixed shell with `rx-app-shell`.
    await expect(app.locator('html')).toHaveClass(/rx-app-shell/);
    expect(await app.evaluate(() => getComputedStyle(document.body).overflow)).toBe('hidden');
  });

  test('switch instance from the launch caret', async ({ app }) => {
    await app.getByTestId('instance-selector').click();
    await app.getByRole('menuitem', { name: /PvP Practice/ }).click();
    await expect(app.getByTestId('play-button')).toContainText('Launch game');
    await expect(app.getByTestId('play-button')).toContainText('Horizon 0.2.15');
  });

  test('join a featured server in one click', async ({ app }) => {
    await app.getByTestId('join-fs2').click();
    await expectRunning(app);
    // The instance remembers the server for "Rejoin".
    await app.goto('/#/instances/inst_survival');
    await expect(app.getByText('arena-nord.fr')).toBeVisible();
  });

  test('resume the last server from the hero', async ({ app }) => {
    await app.getByTestId('hero-resume-server').click();
    await expectRunning(app);
  });
});
