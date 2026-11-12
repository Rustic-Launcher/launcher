import { expect, test } from './fixtures';

// WebGL is software-rendered in headless Chromium: keep these journeys serial.
test.describe.configure({ mode: 'serial' });

test.describe('Locker and store', () => {
  test('the store opens the website, never a page in the launcher', async ({ app: page }) => {
    await page.getByTestId('rail-home').click();
    const here = page.url();
    const popup = page.waitForEvent('popup');
    await page.getByTestId('rail-store').click();
    expect((await popup).url()).toMatch(/^https:\/\/rustic-launcher\.com\/store\/?$/);
    // The launcher stays where it was.
    expect(page.url()).toBe(here);
    // Alt+6 does the same.
    const again = page.waitForEvent('popup');
    await page.keyboard.press('Alt+6');
    expect((await again).url()).toContain('/store');
    expect(page.url()).toBe(here);
  });

  test('locker categories, equip and unequip, outfits', async ({ app: page }) => {
    await page.getByTestId('rail-locker').click();
    await expect(page.getByTestId('horizon-cosmetics-note')).toBeVisible();

    await page.getByTestId('chip-head').click();
    await expect(page.getByTestId('cosmetic-cap_explorer')).toBeVisible();
    await expect(page.getByTestId('cosmetic-backpack_trail')).toHaveCount(0);

    await page.getByTestId('cosmetic-cap_explorer').click();
    await expect(page.getByTestId('cosmetic-cap_explorer')).toHaveAttribute('aria-pressed', 'true');
    await expect(page.getByTestId('cosmetic-beanie_ember')).toHaveAttribute(
      'aria-pressed',
      'false',
    );

    await page.getByTestId('chip-back').click();
    await expect(page.getByTestId('cosmetic-backpack_trail')).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    await page.getByTestId('cosmetic-nothing').click();
    await expect(page.getByTestId('cosmetic-backpack_trail')).toHaveAttribute(
      'aria-pressed',
      'false',
    );
    await expect(page.getByTestId('cosmetic-nothing')).toHaveAttribute('aria-pressed', 'true');

    await page.getByTestId('chip-nametag').click();
    await expect(page.getByTestId('nametag-preview')).toHaveText('SamCrafte');

    await page.getByPlaceholder('Name this look').fill('Weekend');
    await page.getByRole('button', { name: 'Save current look' }).click();
    await expect(page.getByText('Weekend', { exact: true })).toBeVisible();
  });
});
