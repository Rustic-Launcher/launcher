/**
 * Product videos for the website (project "screenshots"; `pnpm videos` records, then encodes with
 * scripts/encode-videos.mjs into apps/web/public/media). Six journeys on the browser mock with its
 * sample data, at 1920 × 1080: launch in two clicks, install a mod, create a server and bring a
 * friend, buy and equip a cosmetic, friends live (presence, messages, a party invite, joining a
 * friend), and a Rustic Server console (a sandboxed plugin that failed, updated and reloaded while
 * the server runs). Nothing in them comes from the game.
 */
import type { Page } from '@playwright/test';
import { expect, seed, test } from './fixtures';
import { Recorder } from './video-kit';

test.use({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 });
test.describe.configure({ mode: 'serial' });

test('journey: launch in two clicks', async ({ page }) => {
  // The longest recording (the launch runs to "running" under the recorder): 3 min was too short.
  test.setTimeout(300_000);
  const r = new Recorder(page, 'launch', 11);
  await r.prepare();
  await seed(page, { latency: 60, speed: 10 });
  await page.goto('/');
  await expect(page.getByTestId('play-button')).toBeVisible({ timeout: 30_000 });
  await page.waitForTimeout(1200);
  await r.start();
  await r.title('rustic launcher · 01', 'Play in <em>two</em> clicks.');
  await r.hover(page.getByTestId('hero-continue'), 900);
  await r.click(page.getByTestId('instance-selector'));
  await r.hold(700);
  await r.click(page.getByRole('menuitem').first());
  await r.hold(500);
  await r.click(page.getByTestId('play-button'), 900);
  await expect(page.getByTestId('play-button')).toHaveAttribute('data-state', 'running', {
    timeout: 60_000,
  });
  await r.hold(900);
  await r.zoom(page.getByTestId('running-pill'), 1.6, 1400);
  await r.glide(1500, 700, 900);
  await r.hold(600);
  await r.stop();
});

test('journey: install a mod from Explore', async ({ page }) => {
  test.setTimeout(180_000);
  const r = new Recorder(page, 'mod', 23);
  await r.prepare();
  await seed(page, { latency: 70, speed: 3 });
  await page.goto('/');
  await expect(page.getByTestId('rail-home')).toBeVisible({ timeout: 30_000 });
  await page.waitForTimeout(1000);
  await r.start();
  await r.title('rustic launcher · 02', 'Mods, <em>checked</em> for you.');
  await r.click(page.getByTestId('rail-content'));
  await expect(page.getByTestId('content-search')).toBeVisible();
  await r.hold(900);
  await r.type(page.getByTestId('content-search'), 'shulker');
  const row = page.getByTestId('content-row-p_shulker');
  await expect(row).toBeVisible();
  await r.hover(page.getByTestId('content-filters'), 800);
  await r.hover(row.getByText('Builds on your Horizon'), 800);
  await r.click(row.getByRole('button', { name: /Install Shulker Peek/ }));
  await expect(row.getByRole('button', { name: 'Installed' })).toBeVisible({ timeout: 20_000 });
  await r.hold(500);
  await r.zoom(row, 1.3, 1300);
  await r.stop();
});

test('journey: create a server and bring a friend', async ({ page }) => {
  test.setTimeout(240_000);
  const r = new Recorder(page, 'server', 37);
  await r.prepare();
  await seed(page, { latency: 60, speed: 5 });
  await page.goto('/');
  await expect(page.getByTestId('rail-home')).toBeVisible({ timeout: 30_000 });
  await page.waitForTimeout(1000);
  await r.start();
  await r.title('rustic launcher · 03', 'Your server, your <em>friends</em>.');
  await r.click(page.getByTestId('rail-servers'));
  await r.hold(700);
  await r.click(page.getByTestId('create-server'));
  await r.type(page.getByTestId('wizard-name'), 'Weekend SMP');
  await r.click(page.getByTestId('wizard-next'));
  await r.click(page.getByTestId('wizard-eula'));
  await r.click(page.getByTestId('wizard-next'));
  await r.hold(600);
  await r.hover(page.getByTestId('hosting-cloud'), 700);
  await r.click(page.getByTestId('wizard-next'));
  await r.hold(600);
  await r.hover(page.getByTestId('access-relay'), 700);
  await r.click(page.getByTestId('wizard-next'));
  await expect(page.getByText('Weekend SMP is running')).toBeVisible({ timeout: 30_000 });
  await r.hold(800);
  await r.cut(async () => {
    await page.getByTestId('rail-home').click();
    await page.waitForTimeout(400);
  });
  await r.click(page.getByTestId('open-social'));
  const social = page.getByTestId('social-overlay');
  await expect(social).toBeVisible();
  await r.click(social.getByTestId('social-tab-chats'));
  await r.click(social.getByTestId('chat-lea_builds'));
  await r.type(social.getByTestId('chat-input'), 'Weekend SMP is up, come play!');
  await page.keyboard.press('Enter');
  await expect(social.getByTestId('chat-pane')).toContainText('👍', { timeout: 15_000 });
  await r.hold(1200);
  await page.keyboard.press('Escape');
  await r.hold(700);
  const joinFriend = page.getByRole('button', { name: /^Join / }).first();
  await r.click(joinFriend);
  await r.hold(1800);
  await r.stop();
});

/** The browser mock's scripted social activity (`window.__rusticSocialMock`, cloud/mock/social.ts). */
interface SocialScript {
  presence(id: string, presence: string, activity: Record<string, unknown> | null): void;
  receive(from: string, body: string): void;
  replies(...bodies: string[]): void;
}
type WithScript = { __rusticSocialMock: SocialScript };

function socialMock(page: Page) {
  return {
    presence: (id: string, presence: string, activity: Record<string, unknown> | null = null) =>
      page.evaluate(
        (a) =>
          (window as unknown as WithScript).__rusticSocialMock.presence(
            a.id,
            a.presence,
            a.activity,
          ),
        { id, presence, activity },
      ),
    receive: (from: string, body: string, reply: string) =>
      page.evaluate(
        (a) => {
          const m = (window as unknown as WithScript).__rusticSocialMock;
          m.replies(a.reply);
          m.receive(a.from, a.body);
        },
        { from, body, reply },
      ),
  };
}

test('journey: friends, live', async ({ page }) => {
  test.setTimeout(180_000);
  const r = new Recorder(page, 'social', 61);
  await r.prepare();
  await seed(page, { latency: 50, speed: 4 });
  await page.goto('/');
  await expect(page.getByTestId('rail-home')).toBeVisible({ timeout: 30_000 });
  await page.waitForFunction(() => '__rusticSocialMock' in window, null, { timeout: 30_000 });
  await page.waitForTimeout(1000);
  const social = socialMock(page);
  await r.start();
  await r.title('rustic launcher · 05', 'Your friends, <em>live</em>.');
  await r.click(page.getByTestId('open-social-link'));
  const overlay = page.getByTestId('social-overlay');
  await expect(overlay).toBeVisible();
  await r.hold(900);
  // Emma comes online, then Jade starts playing on Les Copains SMP.
  await social.presence('u_emma', 'online', { kind: 'launcher', joinable: false });
  await expect(overlay.getByTestId('friend-emma.w')).toBeVisible();
  await r.hover(overlay.getByTestId('friend-emma.w'), 800);
  await r.hold(700);
  await social.presence('u_jade', 'in-game', {
    kind: 'server',
    server_name: 'Les Copains SMP',
    server_address: 'copains.relay.example.invalid',
    joinable: true,
  });
  await r.hover(overlay.getByTestId('friend-jade.dev'), 800);
  await r.hold(900);
  // Invite Max to the party: he accepts a moment later.
  const max = overlay.getByTestId('friend-MaxRedstone');
  await r.hover(max, 600);
  await r.click(max.getByRole('button', { name: /More actions|Plus/ }), 500);
  await r.click(page.getByRole('menuitem', { name: /Invite to party/ }), 600);
  await expect(overlay.getByTestId('party-card')).toContainText('3/', { timeout: 15_000 });
  await r.hover(overlay.getByTestId('party-card'), 700);
  await r.hold(700);
  // A message arrives from Noah.
  await social.receive('u_noah', 'Tu viens sur Arena Nord ? 1v1 ?', 'Go, je t’attends au spawn');
  await r.hold(900);
  await r.click(overlay.getByTestId('social-tab-chats'));
  await r.click(overlay.getByTestId('chat-noah.pvp'));
  await r.hold(600);
  await r.type(overlay.getByTestId('chat-input'), 'J’arrive, je lance mon instance');
  await page.keyboard.press('Enter');
  await expect(overlay.getByTestId('chat-pane')).toContainText('spawn', { timeout: 15_000 });
  await r.hold(1300);
  // Back to friends, and join Noah's server.
  await r.click(overlay.getByTestId('social-tab-friends'));
  await r.hold(500);
  await r.click(overlay.getByRole('button', { name: /^Join Noah/ }));
  await r.hold(1800);
  await r.stop();
});

test('journey: a Rustic Server, its console and its plugins', async ({ page }) => {
  test.setTimeout(180_000);
  const r = new Recorder(page, 'console', 71);
  await r.prepare();
  await seed(page, { latency: 50, speed: 4 });
  await page.goto('/#/servers/srv_copains?tab=console');
  await expect(page.getByTestId('console-log')).toContainText('trapped', { timeout: 30_000 });
  await page.waitForTimeout(1000);
  await r.start();
  await r.title('rustic server · 06', 'Plugins that <em>can’t</em> crash it.');
  // The log: a plugin trapped at start-up, was disabled, and the server went on.
  await r.zoom(page.getByTestId('console-log').getByText(/trapped/), 1.5, 1500);
  const input = page.getByTestId('console-input');
  await r.type(input, 'rustic pl');
  await input.press('Tab');
  await r.hold(400);
  await input.press('Enter');
  await expect(page.getByTestId('console-log')).toContainText('Plugins (', { timeout: 10_000 });
  await r.hold(1200);
  // Plugins: the faulty one says why; update it and it reloads while the server runs.
  await r.click(page.getByTestId('server-tab-plugins'));
  const list = page.getByTestId('plugins-list');
  await expect(list).toBeVisible();
  await r.hold(700);
  const faulty = list.locator('li', { hasText: 'Mini Game' });
  await r.hover(faulty.getByText(/fuel exhausted/), 900);
  await r.hold(600);
  await r.click(faulty.getByRole('button', { name: /Update to/ }));
  await expect(faulty.getByRole('switch')).toHaveAttribute('aria-checked', 'true', {
    timeout: 20_000,
  });
  await r.hold(900);
  await r.click(page.getByTestId('server-tab-console'));
  await expect(page.getByTestId('console-log')).toContainText('Reloaded mini-game');
  await r.hold(600);
  await r.zoom(page.getByTestId('console-log').getByText(/Reloaded mini-game/), 1.45, 1300);
  await r.stop();
});
