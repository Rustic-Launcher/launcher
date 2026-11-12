import { expect, seed, test } from './fixtures';

test.describe('first launch', () => {
  test('goes through every onboarding step to Home', async ({ page }) => {
    await seed(page, { onboarded: false });
    await page.goto('/');
    await expect(page).toHaveURL(/#\/onboarding/);
    const next = page.getByTestId('onboarding-next');

    // 1. Welcome: language + look.
    await expect(page.getByTestId('onboarding-step-welcome')).toBeVisible();
    await page.getByTestId('palette-verdigris').click();
    await expect(page.locator('html')).toHaveAttribute('data-palette', 'verdigris');
    await page.getByTestId('palette-ember').click();
    await next.click();

    // 2. Microsoft: the mock already has an account that owns the game.
    await expect(page.getByTestId('onboarding-step-microsoft')).toBeVisible();
    await expect(page.getByTestId('ms-already')).toBeVisible();
    await expect(next).toBeEnabled();
    await next.click();

    // 3. Rustic account: optional, skip.
    await expect(page.getByTestId('onboarding-step-rustic')).toBeVisible();
    await next.click();

    // 4. Privacy: everything off by default, explicit opt-in.
    await expect(page.getByTestId('onboarding-step-privacy')).toBeVisible();
    for (const id of ['consent-usage', 'consent-performance', 'consent-crash']) {
      await expect(page.getByTestId(id)).toHaveAttribute('aria-checked', 'false');
    }
    await page.getByTestId('consent-crash').click();
    await expect(page.getByTestId('consent-crash')).toHaveAttribute('aria-checked', 'true');
    await next.click();

    // 5. Game files: detection, then preparation with live progress.
    await expect(page.getByTestId('onboarding-step-game')).toBeVisible();
    await expect(page.getByTestId('game-found')).toBeVisible();
    await page.getByTestId('prepare-start').click();
    await expect(page.getByTestId('prepare-progress')).toBeVisible();
    await next.click();

    // 6. Ready: create a first instance and open the launcher.
    await expect(page.getByTestId('onboarding-step-ready')).toBeVisible();
    await page.getByLabel('Instance name').fill('Onboarding World');
    await next.click();

    await expect(page.getByTestId('play-button')).toBeVisible();
    await expect(page).not.toHaveURL(/onboarding/);
    await page.getByTestId('instance-selector').click();
    await expect(page.getByRole('menuitem', { name: /Onboarding World/ })).toBeVisible();
    await page.keyboard.press('Escape');

    // The consent choice was saved.
    await page.goto('/#/settings?section=privacy');
    await expect(page.getByRole('switch', { name: 'Crash reports' })).toHaveAttribute(
      'aria-checked',
      'true',
    );
    await expect(page.getByRole('switch', { name: 'Usage analytics' })).toHaveAttribute(
      'aria-checked',
      'false',
    );
  });

  test('a step waiting on something outside the launcher can be skipped', async ({ page }) => {
    await seed(page, { onboarded: false, fresh: true });
    await page.goto('/#/onboarding');
    const next = page.getByTestId('onboarding-next');
    const skip = page.getByTestId('onboarding-skip');
    await expect(page.getByTestId('onboarding-step-welcome')).toBeVisible();
    await expect(skip).toHaveCount(0);
    await next.click();
    // No Microsoft account yet (or Mojang has not approved the app): Continue waits, skip moves on.
    await expect(page.getByTestId('onboarding-step-microsoft')).toBeVisible();
    await expect(next).toBeDisabled();
    await skip.click();
    // Optional steps: nothing to skip where Continue is already allowed.
    for (const step of ['rustic', 'privacy']) {
      await expect(page.getByTestId(`onboarding-step-${step}`)).toBeVisible();
      await expect(skip).toHaveCount(0);
      await next.click();
    }
    // Game files not prepared yet: same.
    await expect(page.getByTestId('onboarding-step-game')).toBeVisible();
    await expect(next).toBeDisabled();
    await skip.click();
    await expect(page.getByTestId('onboarding-step-ready')).toBeVisible();
  });

  test('is keyboard operable', async ({ page }) => {
    await seed(page, { onboarded: false });
    await page.goto('/#/onboarding');
    await expect(page.getByRole('heading', { level: 1, name: 'Welcome to Rustic' })).toBeFocused();
    await page.getByTestId('onboarding-next').focus();
    await page.keyboard.press('Enter');
    await expect(
      page.getByRole('heading', { level: 1, name: 'Sign in with Microsoft' }),
    ).toBeFocused();
  });
});
