import { readFileSync } from 'node:fs';
import { expect, seed, test } from './fixtures';

/** An original 64×64 test skin drawn in code (no Mojang texture). */
const SKIN = `data:image/png;base64,${readFileSync(new URL('./assets/sample-skin.png', import.meta.url)).toString('base64')}`;

test.describe('Launchpad hero', () => {
  test('the forest video plays behind the launch button, with the player in their skin', async ({
    page,
  }) => {
    await seed(page);
    await page.addInitScript((url) => {
      window.localStorage.setItem('rustic.mock.skin', JSON.stringify({ url, slim: false }));
    }, SKIN);
    await page.goto('/');
    const hero = page.getByTestId('hero-video');
    await expect(hero).toBeVisible({ timeout: 30_000 });
    const video = hero.locator('video');
    await expect(video).toHaveAttribute('poster', '/media/hero-forest.webp');
    await expect(hero).toHaveAttribute('data-state', 'playing');
    await expect
      .poll(() => video.evaluate((v: HTMLVideoElement) => v.readyState >= 2 && !v.paused), {
        timeout: 15_000,
      })
      .toBe(true);
    await expect(page.getByTestId('play-button')).toBeVisible();
    // The 3D model is a lazy chunk (three.js): give it the same slack as the video.
    await expect(page.getByRole('img', { name: /player/i }).first()).toBeVisible({
      timeout: 30_000,
    });
    // Every avatar of the profile shows the face of the skin (with its hat layer), not initials.
    const face = page.getByTestId('account-switcher').locator('img');
    await expect(face).toBeVisible();
    await expect(face).toHaveAttribute('src', /^data:image\/png;base64,/);
    await expect
      .poll(() => face.evaluate((img: HTMLImageElement) => [img.naturalWidth, img.naturalHeight]))
      .toEqual([8, 8]);
  });

  test('with nothing going on, the friends hint sits in the middle of the aside', async ({
    page,
  }) => {
    await seed(page);
    await page.goto('/');
    await expect(page.getByTestId('play-button')).toBeVisible({ timeout: 30_000 });
    await page.evaluate(() =>
      (window as unknown as { __rusticSocialMock: { quiet(): void } }).__rusticSocialMock.quiet(),
    );
    const hint = page.getByTestId('aside-empty');
    await expect(hint).toBeVisible({ timeout: 30_000 });
    const box = await hint.boundingBox();
    const view = page.viewportSize()!;
    // Roughly centred vertically in the window, not pinned to the bottom.
    expect(Math.abs(box!.y + box!.height / 2 - view.height / 2)).toBeLessThan(view.height * 0.2);
    // Scrolled to the news, the header and the hint stay in view.
    const scrolled = await page.getByTestId('aside-friends-header').evaluate((el) => {
      let p = el.parentElement;
      while (p && getComputedStyle(p).overflowY !== 'auto') p = p.parentElement;
      p!.scrollTop = 600;
      return p!.scrollTop;
    });
    expect(scrolled).toBeGreaterThan(100);
    await expect(page.getByTestId('aside-friends-header')).toBeInViewport();
    await expect(hint).toBeInViewport();
  });
});
