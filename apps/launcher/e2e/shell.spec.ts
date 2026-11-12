import { expect, expectRunning, seed, test } from './fixtures';

test.describe('app shell', () => {
  test('keyboard navigation of the rail', async ({ app }) => {
    // The skip link is the first tab stop.
    await app.keyboard.press('Tab');
    await expect(app.getByRole('link', { name: 'Skip to content' })).toBeFocused();
    const home = app.getByTestId('rail-home');
    await home.focus();
    await expect(home).toBeFocused();

    // Roving focus: arrows move between items without leaving the rail.
    await app.keyboard.press('ArrowDown');
    await expect(app.getByTestId('rail-instances')).toBeFocused();
    await app.keyboard.press('ArrowDown');
    await expect(app.getByTestId('rail-content')).toBeFocused();
    await app.keyboard.press('End');
    await expect(app.getByTestId('rail-settings')).toBeFocused();
    await app.keyboard.press('Home');
    await expect(home).toBeFocused();
    await app.keyboard.press('ArrowUp');
    await expect(app.getByTestId('rail-settings')).toBeFocused();

    // Enter activates: the store is the website's (a browser tab), the others are pages
    // whose aria-current and breadcrumb follow.
    await app.keyboard.press('ArrowUp'); // store
    const store = app.waitForEvent('popup');
    await app.keyboard.press('Enter');
    expect((await store).url()).toContain('/store');
    await app.getByTestId('rail-store').focus();
    await app.keyboard.press('ArrowUp'); // locker
    await app.keyboard.press('Enter');
    await expect(app).toHaveURL(/#\/locker/);
    await expect(app.getByTestId('rail-locker')).toHaveAttribute('aria-current', 'page');
    await expect(app.getByTestId('breadcrumb')).toContainText('Locker');

    // Alt+digit shortcuts from anywhere.
    await app.keyboard.press('Alt+4');
    await expect(app).toHaveURL(/#\/servers/);
    await expect(app.getByTestId('breadcrumb')).toContainText('Servers/Featured');
    await app.keyboard.press('Alt+1');
    await expect(app).toHaveURL(/#\/$/);
  });

  test('title bar: breadcrumb, product switcher and new-profile button', async ({ app }) => {
    await app.goto('/#/content?kind=shader');
    await expect(app.getByTestId('breadcrumb')).toContainText('Explore/Shaders');
    await app.getByTestId('product-switcher').click();
    await expect(app.getByRole('menuitem', { name: /Rustic Launcher/ })).toBeVisible();
    await app.keyboard.press('Escape');
    await app.getByTestId('rail-new').click();
    await expect(app.getByRole('dialog', { name: 'New instance' })).toBeVisible();
  });

  test('global search with Ctrl+K reaches settings and friends', async ({ app }) => {
    await app.keyboard.press('Control+k');
    const input = app.getByPlaceholder('Search content, friends, servers, settings…');
    await expect(input).toBeFocused();
    await input.fill('parental');
    await app.keyboard.press('Enter');
    await expect(app).toHaveURL(/#\/settings\?section=parental/);

    // A friend result opens their conversation in the Social view.
    await app.keyboard.press('Control+k');
    await app.getByPlaceholder('Search content, friends, servers, settings…').fill('Léa');
    await app.getByRole('option', { name: /Léa/ }).click();
    await expect(app.getByTestId('social-overlay')).toBeVisible();
    await expect(app.getByTestId('chat-pane')).toContainText('Léa');
  });

  test('social window: friends, chats and the unread badge', async ({ app }) => {
    // 3 unread chat messages + 1 incoming request.
    await expect(app.getByTestId('open-social')).toContainText('4');
    await app.getByTestId('open-social').click();
    const social = app.getByTestId('social-overlay');
    await expect(social).toBeVisible();
    await expect(social.getByTestId('chat-empty')).toContainText('Select a conversation');

    await social.getByTestId('social-tab-chats').click();
    await social.getByTestId('chat-lea_builds').click();
    await expect(social.getByTestId('chat-pane')).toContainText('Tu viens sur Les Copains');
    await social.getByTestId('chat-input').fill('J’arrive !');
    await social.getByTestId('chat-input').press('Enter');
    await expect(social.getByTestId('chat-pane')).toContainText('J’arrive !');
    // Léa answers a moment later (realtime).
    await expect(social.getByTestId('chat-pane')).toContainText('👍', { timeout: 10_000 });

    await app.keyboard.press('Escape');
    await expect(social).toHaveCount(0);
  });

  test('invite a friend to the party', async ({ app }) => {
    await app.getByTestId('open-social-link').click();
    const row = app.getByTestId('friend-noah.pvp');
    await row.hover();
    await row.getByRole('button', { name: 'More actions for Noah' }).click();
    await app.getByRole('menuitem', { name: 'Invite to party' }).click();
    await expect(app.getByText('Noah was invited to your party', { exact: true })).toBeVisible();
    // Online friends accept after a moment: the party grows to 3.
    await expect(app.getByTestId('party-card')).toContainText('3/8', { timeout: 10_000 });
  });

  test('send a friend request and accept an incoming one', async ({ app }) => {
    test.slow(); // goes through the Add a friend dialog
    await app.getByTestId('open-social').click();
    await app.getByTestId('add-friend').click();
    const dialog = app.getByRole('dialog', { name: 'Add a friend' });
    await dialog.getByLabel('Rustic username').fill('NinaSky');
    await dialog.getByRole('button', { name: 'Find' }).click();
    await dialog.getByRole('button', { name: 'Send request' }).click();
    await expect(dialog.getByTestId('add-friend-result').getByRole('status')).toHaveText(
      'Request sent',
    );
    await app.keyboard.press('Escape');
    await expect(dialog).toBeHidden();

    await app.getByRole('button', { name: 'Accept' }).first().click();
    await expect(app.getByTestId('friend-HugoMines')).toBeVisible();
  });

  test('no docked friends panel on pages; the launchpad keeps its column', async ({ app }) => {
    await expect(app.getByRole('heading', { name: /Friends \(5 online\)/ })).toBeVisible();
    await app.getByTestId('rail-content').click();
    await expect(app.getByTestId('breadcrumb')).toContainText('Explore');
    await expect(app.getByRole('heading', { name: /Friends \(/ })).toHaveCount(0);
    await expect(app.getByTestId('open-social')).toBeVisible();
  });

  test('launch from the launchpad: update then launch, cancel works', async ({ page: app }) => {
    // Real-time download speed so the progress states are observable.
    await seed(app, { speed: 1 });
    await app.goto('/');
    const play = app.getByTestId('play-button');
    await expect(play).toHaveAttribute('data-state', 'update');
    await play.click();
    await expect(play).toHaveAttribute('data-state', 'progress');
    await expect(app.getByTestId('global-progress')).toBeVisible();
    await expect(play).toHaveAccessibleName('Cancel');
    await play.click();
    await expect(play).toHaveAttribute('data-state', 'update');
    // Second try goes all the way; the running pill counts it.
    await play.click();
    await expectRunning(app);
    await expect(play).toHaveAttribute('data-state', 'running');
    // While the game runs every animation rests (CSS, 3D viewers, the hero video).
    await expect(app.locator('html')).toHaveAttribute('data-paused', 'game');
    await expect(app.getByTestId('hero-video')).toHaveAttribute('data-state', 'paused');
    await expect
      .poll(() =>
        app
          .getByTestId('hero-video')
          .locator('video')
          .evaluate((v: HTMLVideoElement) => v.paused),
      )
      .toBe(true);
    await app.getByTestId('running-pill').click();
    const stop = app.getByRole('button', { name: 'Stop', exact: true });
    await expect(stop).toBeVisible();
    await stop.click();
    await expect(app.locator('html')).not.toHaveAttribute('data-paused', /.+/, { timeout: 15_000 });
  });
});
