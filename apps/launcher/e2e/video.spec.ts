/** Records docs/screenshots/navigation.webm (project "screenshots"). */
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { expect, seed, test } from './fixtures';

const OUT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../docs/screenshots');

test.use({ video: { mode: 'on', size: { width: 1440, height: 900 } } });

test('navigation video', async ({ page }) => {
  test.setTimeout(120_000);
  await seed(page, { latency: 60, speed: 3 });
  await page.goto('/');
  await expect(page.getByTestId('play-button')).toBeVisible({ timeout: 30_000 });
  await page.waitForTimeout(1500);
  for (const id of ['instances', 'content', 'servers', 'locker', 'store', 'settings', 'home']) {
    await page.getByTestId(`rail-${id}`).click();
    await page.waitForTimeout(1600);
  }
  await page.getByTestId('play-button').click();
  await page.waitForTimeout(4000);
  await page.getByTestId('open-social').click();
  await page.waitForTimeout(1200);
  await page.getByTestId('social-tab-chats').click();
  await page.getByTestId('chat-lea_builds').click();
  await page.waitForTimeout(1500);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(600);
  await page.keyboard.press('Control+k');
  await page.keyboard.type('server', { delay: 80 });
  await page.waitForTimeout(1200);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(800);
  const video = page.video();
  await page.close();
  await video?.saveAs(path.join(OUT, 'navigation.webm'));
});
