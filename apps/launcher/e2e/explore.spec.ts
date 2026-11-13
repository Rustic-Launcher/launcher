import { expect, seed, test } from './fixtures';

test.describe('Explore: Discover, Browse, Mine', () => {
  test('Discover shows a featured carousel and themed rows; See all opens Browse', async ({
    app,
  }) => {
    await app.goto('/#/content');
    const hero = app.getByTestId('discover-hero');
    await expect(hero).toBeVisible();
    // Rustic's verified projects lead the carousel.
    await expect(hero.getByRole('heading', { name: 'Sky Islands' })).toBeVisible();
    await expect(hero.getByRole('button', { name: /Install Sky Islands/ })).toBeVisible();
    await expect(app.getByTestId('discover-row-new')).toBeVisible();
    await expect(app.getByTestId('discover-row-horizon')).toBeVisible();
    const verified = app.getByTestId('discover-row-verified');
    await expect(verified.getByTestId('content-card-rustic:sky_islands')).toBeVisible();
    await expect(app.getByTestId('discover-row-yours')).toContainText('For Survie entre amis');

    await verified.getByTestId('discover-row-verified-see-all').click();
    await expect(app.getByTestId('explore-tab-browse')).toHaveAttribute('aria-selected', 'true');
    await expect(app.getByTestId('active-filters')).toContainText('Verified by Rustic only');
    await expect(app.getByTestId('content-row-rustic:sky_islands')).toBeVisible();
    await expect(app.getByTestId('content-row-p_minimap')).toHaveCount(0);
    await expect(app.getByTestId('active-filters')).toContainText('Rustic');
    // Removing the chip lists Rustic's other projects again.
    await app.getByRole('button', { name: 'Remove the filter Verified by Rustic only' }).click();
    await expect(app.getByTestId('content-row-p_hud')).toBeVisible();
    await expect(app.getByTestId('content-row-p_minimap')).toHaveCount(0);
  });

  test('the carousel works with the keyboard and can be paused', async ({ app }) => {
    await app.goto('/#/content');
    const hero = app.getByTestId('discover-hero');
    await expect(hero).toBeVisible();
    // The current dot names the slide shown (the leaving one may still be in the DOM).
    const slide = hero.locator('button[aria-current="true"]');
    // Focus inside stops the rotation, so the steps below are the keyboard's only.
    await app.getByTestId('discover-hero-next').focus();
    await expect(slide).toHaveAttribute('aria-label', /^\d+ of \d+$/);
    await app.keyboard.press('Home');
    await expect(slide).toHaveAttribute('aria-label', /^1 of \d+$/);
    await app.keyboard.press('ArrowRight');
    await expect(slide).toHaveAttribute('aria-label', /^2 of \d+$/);
    await app.keyboard.press('ArrowRight');
    await expect(slide).toHaveAttribute('aria-label', /^3 of \d+$/);
    await app.keyboard.press('ArrowLeft');
    await expect(slide).toHaveAttribute('aria-label', /^2 of \d+$/);
    await app.keyboard.press('End');
    const total = (await slide.getAttribute('aria-label'))!.split(' of ')[1];
    await expect(slide).toHaveAttribute('aria-label', `${total} of ${total}`);
    await app.keyboard.press('Home');
    await expect(slide).toHaveAttribute('aria-label', `1 of ${total}`);
    await hero.getByRole('button', { name: 'Pause the slideshow' }).click();
    await expect(hero.getByRole('button', { name: 'Play the slideshow' })).toBeVisible();
    // The slide picked from the dots is announced as current.
    await hero.getByRole('button', { name: `2 of ${total}` }).click();
    await expect(slide).toHaveAttribute('aria-label', `2 of ${total}`);
  });

  test('Browse filters by source, price and category, and pages with Load more', async ({
    app,
  }) => {
    await app.goto('/#/content?kind=resourcepack&tab=browse');
    await expect(app.getByTestId('content-row-p_rp_faithful')).toBeVisible();
    await app.getByTestId('content-filters').click();
    await app.getByTestId('price-paid').click();
    await app.keyboard.press('Escape');
    const paid = app.getByTestId('content-row-rustic:cozy_lanterns');
    await expect(paid).toBeVisible();
    await expect(app.getByTestId('content-row-p_rp_faithful')).toHaveCount(0);
    // A paid project shows its price and never gets a working buy or install button.
    await expect(paid.getByText('€2.99')).toBeVisible();
    await expect(paid.getByRole('button', { name: 'Coming soon' })).toHaveAttribute(
      'aria-disabled',
      'true',
    );

    await app.getByTestId('content-filters').click();
    await app.getByTestId('price-all').click();
    await app.getByTestId('source-modrinth').click();
    await app.getByTestId('source-horizon').click();
    await app.keyboard.press('Escape');
    await expect(app.getByTestId('active-filters')).toContainText('Rustic');
    await expect(app.getByTestId('content-row-p_rp_faithful')).toHaveCount(0);
    await expect(app.getByTestId('content-row-p_rp_ui')).toBeVisible();

    // Paging: mods, everything listed (12 a page).
    await app.goto('/#/content?kind=mod&tab=browse');
    await app.getByTestId('content-filters').click();
    await app.getByTestId('compatible-only').click();
    await app.keyboard.press('Escape');
    const rows = app.getByTestId('browse-list').locator('li');
    await expect(rows).toHaveCount(12);
    await app.getByTestId('load-more').click();
    await expect.poll(() => rows.count()).toBeGreaterThan(12);
    await expect(app.getByTestId('load-more')).toHaveCount(0);
  });

  test('Browse switches to a grid and keeps the choice', async ({ app }) => {
    await app.goto('/#/content?tab=browse');
    await app.getByRole('radio', { name: 'Grid' }).click();
    await expect(app.getByTestId('browse-grid')).toBeVisible();
    await expect(app.getByTestId('content-card-p_minimap')).toBeVisible();
    await app.reload();
    await expect(app.getByTestId('browse-grid')).toBeVisible();
    await app.getByRole('radio', { name: 'List' }).click();
    await expect(app.getByTestId('browse-list')).toBeVisible();
  });

  test('a native mod installs after its trust notice, Rust and a build on the PC', async ({
    app,
  }) => {
    await app.goto('/#/content?kind=mod&tab=browse');
    await app.getByTestId('content-search').fill('fast paths');
    const row = app.getByTestId('content-row-rustic:fast_paths');
    await expect(row.getByText('Builds on your PC')).toBeVisible();
    await row.getByRole('button', { name: /Install Fast Paths into Survie entre amis/ }).click();

    // The trust notice: nothing happens before the player ticks it and accepts.
    const trust = app.getByTestId('native-trust-dialog');
    await expect(trust).toBeVisible();
    await expect(trust).toContainText('not sandboxed');
    await expect(app.getByTestId('native-trust-accept')).toBeDisabled();
    await app.getByTestId('native-trust-check').check();
    await app.getByTestId('native-trust-accept').click();

    // The mod is installed, then Rust is missing: the launcher offers to install it.
    const rust = app.getByTestId('native-toolchain-dialog');
    await expect(rust).toBeVisible({ timeout: 15_000 });
    await expect(rust).toContainText('static.rust-lang.org');
    await app.getByTestId('native-toolchain-install').click();
    await expect(
      app.getByText('Building native mods for Survie entre amis', { exact: true }),
    ).toBeVisible({
      timeout: 15_000,
    });

    // The instance shows its build, then its up-to-date client.
    await app.goto('/#/instances/inst_survival?tab=content');
    const panel = app.getByTestId('native-build-panel');
    await expect(panel).toContainText('fast_paths 0.3.0');
    await expect(panel.getByTestId('native-build-state')).toHaveText('Up to date', {
      timeout: 30_000,
    });
    await expect(panel).toContainText('Built');
    // A rebuild no longer asks for the notice or Rust.
    await panel.getByTestId('native-build-button').click();
    await expect(panel.getByTestId('native-build-state')).toHaveText('Building…');
    await expect(panel.getByTestId('native-build-state')).toHaveText('Up to date', {
      timeout: 30_000,
    });
  });

  test('a native mod is not installed when the trust notice is declined', async ({ app }) => {
    await app.goto('/#/content?kind=mod&tab=browse');
    await app.getByTestId('content-search').fill('fast paths');
    const row = app.getByTestId('content-row-rustic:fast_paths');
    await row.getByRole('button', { name: /Install Fast Paths into Survie entre amis/ }).click();
    await expect(app.getByTestId('native-trust-dialog')).toBeVisible();
    await app.getByRole('button', { name: 'Cancel' }).click();
    await expect(app.getByTestId('native-trust-dialog')).toBeHidden();
    await app.goto('/#/instances/inst_survival?tab=content');
    await expect(app.getByText('Fast Paths')).toHaveCount(0);
    await expect(app.getByTestId('native-build-panel')).toHaveCount(0);
  });

  test('My content: installed items, the Rustic library and own creations', async ({ app }) => {
    await app.goto('/#/content?tab=mine');
    const installed = app.getByTestId('mine-installed');
    await expect(installed).toContainText('Installed on Survie entre amis');
    await expect(installed.getByText('Atlas Minimap')).toBeVisible();
    // The search field filters the lists.
    await app.getByTestId('content-search').fill('lumen');
    await expect(installed.getByText('Atlas Minimap')).toHaveCount(0);
    await expect(installed.getByText('Lumen Renderer')).toBeVisible();
    await app.getByTestId('content-search').fill('');

    const library = app.getByTestId('mine-library');
    await expect(library.getByTestId('library-row-sky_islands')).toBeVisible();
    const creations = app.getByTestId('mine-creations');
    await expect(creations.getByTestId('creation-row-sam_tools')).toContainText(
      'Waiting for review',
    );
    const popup = app.waitForEvent('popup');
    await creations.getByTestId('publish-on-site').click();
    expect((await popup).url()).toBe('https://rustic-launcher.com/marketplace/publish');
  });

  test('a free Rustic project goes into the library from its page', async ({ app }) => {
    await app.goto('/#/content/rustic:golden_hour_plus');
    await expect(app.getByRole('heading', { name: 'Golden Hour+' })).toBeVisible();
    const toggle = app.getByTestId('library-toggle');
    await expect(toggle).toHaveText('Add to my library');
    await toggle.click();
    await expect(toggle).toHaveText('In my library');
    await expect(toggle).toHaveAttribute('aria-pressed', 'true');
    await app.goto('/#/content?tab=mine');
    await expect(app.getByTestId('library-row-golden_hour_plus')).toBeVisible();
  });

  test('a paid project shows its price and "Coming soon", never a buy button', async ({ app }) => {
    await app.goto('/#/content/rustic:cozy_lanterns');
    await expect(app.getByTestId('detail-price')).toContainText('€2.99');
    await expect(app.getByRole('button', { name: 'Coming soon' })).toHaveAttribute(
      'aria-disabled',
      'true',
    );
    await expect(app.getByRole('button', { name: /buy/i })).toHaveCount(0);
    await expect(app.getByTestId('library-toggle')).toHaveCount(0);
  });
});

test.describe('Install links from the website', () => {
  test('a link that starts the launcher opens the project with the install prompt', async ({
    page,
  }) => {
    await seed(page);
    await page.addInitScript(() => {
      if (window.sessionStorage.getItem('rx-dl')) return;
      window.sessionStorage.setItem('rx-dl', '1');
      window.localStorage.setItem(
        'rustic.mock.deeplink',
        'rustic://content/rustic/sky_islands?version=sky_islands%401.1.9',
      );
    });
    await page.goto('/');
    const prompt = page.getByTestId('install-prompt');
    await expect(prompt).toBeVisible({ timeout: 30_000 });
    await expect(prompt.getByRole('heading', { name: 'Install Sky Islands?' })).toBeVisible();
    // The asked version is preselected; nothing installs until the player confirms.
    await expect(prompt.getByRole('combobox', { name: 'Version' })).toContainText('1.1.9');
    await expect(page.getByTestId('breadcrumb')).toContainText('Explore/Details');
    await prompt
      .getByRole('button', { name: /Install Sky Islands into Survie entre amis/ })
      .click();
    await expect(
      page.getByText('Installing Sky Islands into Survie entre amis', { exact: true }),
    ).toBeVisible({ timeout: 15_000 });
    await prompt.getByRole('button', { name: 'Not now' }).click();
    await expect(prompt).toHaveCount(0);
  });

  test('a link while the launcher runs opens the page; bad links are ignored', async ({ app }) => {
    expect(
      await app.evaluate(() => window.rusticMockDeepLink?.('rustic://content/rustic/../admin')),
    ).toBe(false);
    expect(
      await app.evaluate(() => window.rusticMockDeepLink?.('rustic://content/modrinth/AANobbMI')),
    ).toBe(false);
    await expect(app.getByTestId('install-prompt')).toHaveCount(0);
    expect(
      await app.evaluate(() => window.rusticMockDeepLink?.('rustic://content/rustic/arena_kit')),
    ).toBe(true);
    const prompt = app.getByTestId('install-prompt');
    await expect(prompt.getByRole('heading', { name: 'Install Arena Kit?' })).toBeVisible();
  });

  test('an unknown or private project says it is not available', async ({ app }) => {
    expect(
      await app.evaluate(() => window.rusticMockDeepLink?.('rustic://content/rustic/sam_tools')),
    ).toBe(true);
    await expect(app.getByText('This project is not available')).toBeVisible();
    await app.getByRole('link', { name: 'Back to Explore' }).click();
    await expect(app.getByTestId('explore-tab-discover')).toHaveAttribute('aria-selected', 'true');
  });
});
