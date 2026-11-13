import { expect, test } from './fixtures';

test.describe('settings', () => {
  test('switches palette and theme live', async ({ app }) => {
    await app.goto('/#/settings?section=appearance');
    const radios = app.getByRole('radiogroup', { name: 'Palette' }).getByRole('radio');
    await expect(radios).toHaveCount(3);
    const third = radios.nth(2);
    const id = await third.getAttribute('data-palette-id');
    expect(id).not.toBe('ember');
    await third.click();
    await expect(third).toHaveAttribute('aria-checked', 'true');
    await expect(app.locator('html')).toHaveAttribute('data-palette', id!);

    await app.getByRole('radio', { name: 'Light' }).click();
    await expect(app.locator('html')).toHaveAttribute('data-theme', 'light');
    await app.getByRole('radio', { name: 'Dark' }).click();
    await expect(app.locator('html')).toHaveAttribute('data-theme', 'dark');
  });

  test('switches the language to French at once', async ({ app }) => {
    await app.goto('/#/settings?section=launcher');
    await expect(app.getByTestId('rail-home')).toHaveAccessibleName('Launchpad');
    await app.getByTestId('language-select').click();
    await app.getByTestId('language-select-fr_fr').click();
    await expect(app.getByTestId('rail-home')).toHaveAccessibleName('Accueil');
    await expect(app.getByRole('heading', { level: 1, name: 'Paramètres' })).toBeVisible();
    await expect(app.locator('html')).toHaveAttribute('lang', 'fr-FR');
  });

  test('lists every game language with flags, searchable, translated ones first', async ({
    app,
  }) => {
    await app.goto('/#/settings?section=launcher');
    const select = app.getByTestId('language-select');
    await expect(select).toHaveAccessibleName(/English \(United States\)/);
    await select.click();
    const list = app.getByTestId('language-select-list');
    await expect(list.getByRole('option')).toHaveCount(143);
    // Translated first; a language shown in English says so.
    const groups = list.locator('[cmdk-group-heading]');
    await expect(groups).toHaveText(['Translated', 'Other languages']);
    await expect(app.getByTestId('language-select-ja_jp')).toContainText('in English for now');
    await expect(app.getByTestId('language-select-de_de').locator('[data-flag] svg')).toBeVisible();
    // Search by English name, native name or code, accents ignored.
    await app.getByTestId('language-select-search').fill('portugues');
    await expect(list.getByRole('option')).toHaveCount(2);
    await app.getByTestId('language-select-search').fill('german');
    await app.keyboard.press('Enter');
    await expect(app.getByTestId('rail-home')).toHaveAccessibleName('Startseite');
    await expect(app.locator('html')).toHaveAttribute('lang', /^de/);
    await expect(app.getByRole('heading', { level: 1, name: 'Einstellungen' })).toBeVisible();
  });

  test('a language without a translation shows English, right to left when it is written so', async ({
    app,
  }) => {
    await app.goto('/#/settings?section=launcher');
    await app.getByTestId('language-select').click();
    await app.getByTestId('language-select-search').fill('arabic');
    await app.getByTestId('language-select-ar_sa').click();
    await expect(app.locator('html')).toHaveAttribute('dir', 'rtl');
    await expect(app.locator('html')).toHaveAttribute('lang', 'ar-SA');
    await expect(app.getByRole('heading', { level: 1, name: 'Settings' })).toBeVisible();
    await app.getByTestId('language-select').click();
    await app.getByTestId('language-select-en_us').click();
    await expect(app.locator('html')).toHaveAttribute('dir', 'ltr');
  });

  test('offline play is the owner itself, under its official name', async ({ app }) => {
    await app.goto('/#/settings?section=accounts');
    const offline = app.getByTestId('settings-offline');
    // No free-name profile: the verified account plays offline, under its own name.
    await expect(offline.getByRole('textbox')).toHaveCount(0);
    // The mock's active Microsoft account.
    await expect(app.getByTestId('account-switcher')).toContainText('SamCrafte');
    await expect(offline.getByTestId('offline-identity')).toContainText('SamCrafte');
    await expect(offline.getByTestId('offline-identity')).toContainText(
      'Offline play available until',
    );
  });

  test('adds a Microsoft account with a device code', async ({ app }) => {
    await app.goto('/#/settings?section=accounts');
    const ms = app.getByTestId('settings-microsoft');
    await ms.getByRole('button', { name: 'Add a Microsoft account' }).click();
    await ms.getByTestId('ms-login-start').click();
    await expect(ms.getByTestId('ms-device-code')).toBeVisible();
    await expect(ms.getByTestId('ms-device-code')).toContainText('RX7-K2M');
    await expect(app.getByText('Signed in as SamCrafte').first()).toBeVisible();
    await expect(ms.getByTestId('ms-device-code')).toBeHidden();
  });

  test('Rustic Server: the program in use, a chosen one, then automatic again', async ({ app }) => {
    await app.goto('/#/settings?section=game');
    const card = app.getByTestId('settings-rustic-server');
    const origin = card.getByTestId('rustic-server-origin');
    await expect(origin).toHaveText('Shipped with build v0.2.15');
    await expect(card.getByTestId('rustic-server-path')).toContainText(
      'builds/v0.2.15/rustic-server.exe',
    );
    await expect(card.getByText('Rustic 0.1.0 (rustic:api 0.1)')).toBeVisible();
    await expect(card.getByRole('button', { name: 'Use automatic' })).toBeHidden();

    await card.getByRole('button', { name: 'Choose…' }).click();
    await expect(origin).toHaveText('Chosen manually');
    await expect(card.getByTestId('rustic-server-path')).toHaveText(
      'C:/Users/Sam/Rustic/rustic-server.exe',
    );
    await card.getByRole('button', { name: 'Use automatic' }).click();
    await expect(origin).toHaveText('Shipped with build v0.2.15');
    await expect(card.getByRole('button', { name: 'Use automatic' })).toBeHidden();
  });

  test('Native mods: Rust, the compile cache and its limit', async ({ app }) => {
    await app.goto('/#/settings?section=game');
    const card = app.getByTestId('settings-native');
    await expect(card).toContainText('Native mods');
    await expect(card.getByTestId('native-rust-state')).toHaveText('Not installed');
    await expect(card.getByTestId('native-cache-clear')).toBeDisabled();
    const limit = card.getByTestId('native-cache-limit');
    await expect(limit).toHaveValue('30');
    await limit.fill('12');
    await limit.press('Enter');
    await expect(app.getByText('Cache limit saved', { exact: true })).toBeVisible();
    await limit.fill('9999');
    await limit.press('Enter');
    await expect(
      app.getByText('The limit is a whole number of GiB from 0 to 4096.', { exact: true }),
    ).toBeVisible();
    await expect(limit).toHaveValue('12');
    // Installing Rust from Settings.
    await card.getByRole('button', { name: 'Install Rust' }).click();
    await app.getByTestId('native-toolchain-install').click();
    await expect(card.getByTestId('native-rust-state')).toHaveText('Ready', { timeout: 15_000 });
  });

  test('sub-navigation is deep-linkable and keyboard navigable', async ({ app }) => {
    await app.goto('/#/settings?section=privacy');
    await expect(app.getByTestId('settings-nav-privacy')).toHaveAttribute('aria-current', 'page');
    await app.getByTestId('settings-nav-privacy').focus();
    await app.keyboard.press('ArrowDown');
    await expect(app).toHaveURL(/section=parental/);
    await expect(app.getByTestId('settings-nav-parental')).toBeFocused();
  });

  test('Rustic account: Microsoft sign-in first, email below; linked Microsoft can be unlinked', async ({
    app,
  }) => {
    await app.goto('/#/settings?section=accounts');
    const linked = app.getByTestId('rustic-linked-microsoft');
    await expect(linked).toContainText('sam@example.invalid');
    await app.getByTestId('rustic-unlink-microsoft').click();
    await expect(app.getByTestId('rustic-link-microsoft')).toBeVisible();
    await app.getByTestId('settings-rustic').getByRole('button', { name: 'Sign out' }).click();
    const form = app.getByTestId('rustic-auth-form');
    await expect(form.getByTestId('rustic-microsoft-signin')).toHaveText('Continue with Microsoft');
    await expect(form.getByText('or with an email')).toBeVisible();
    const ms = await form.getByTestId('rustic-microsoft-signin').boundingBox();
    const email = await form.getByTestId('rustic-email').boundingBox();
    expect(ms!.y).toBeLessThan(email!.y);
  });
});
