import { expect, seed, test } from './fixtures';

test.describe('servers', () => {
  test('create a local Rustic server in the wizard and open it', async ({ app: page }) => {
    await page.getByTestId('rail-servers').click();
    await expect(page.getByRole('heading', { level: 1, name: 'Servers' })).toBeVisible();
    await page.getByTestId('create-server').click();

    // Step 1: Basics
    await page.getByTestId('wizard-name').fill('Weekend SMP');
    await expect(page.getByTestId('version-line')).toContainText('Rustic Server 0.1 on Horizon');
    await page.getByTestId('wizard-next').click();
    await expect(page.getByText('Accept the EULA to run a server.')).toBeVisible();
    await page.getByTestId('wizard-eula').click();
    await page.getByTestId('wizard-next').click();

    // Step 2: Where to host (This PC is the default)
    await expect(page.getByTestId('hosting-cloud')).toContainText('Coming soon');
    await expect(page.getByTestId('requirements')).toContainText('Game files prepared');
    await page.getByTestId('wizard-next').click();

    // Step 3: Access, then create
    await expect(page.getByTestId('access-relay')).toContainText('relay.example.invalid');
    await expect(page.getByTestId('summary-name')).toHaveText('Weekend SMP');
    await page.getByTestId('wizard-next').click();

    await expect(page.getByTestId('local-progress')).toBeVisible();
    await expect(page.getByText('Weekend SMP is running')).toBeVisible({ timeout: 20_000 });
    await page.getByTestId('open-server').click();
    await expect(page).toHaveURL(/#\/servers\/srv_[a-z0-9]+/);
  });

  test('no rustic-server: the toast says so in French and opens its Settings row', async ({
    page,
  }) => {
    await seed(page, { language: 'fr' });
    await page.addInitScript(() =>
      window.localStorage.setItem('rustic.mock.no-rustic-server', 'true'),
    );
    await page.goto('/');
    await expect(page.getByTestId('rail-home')).toBeVisible({ timeout: 30_000 });
    await page.goto('/#/servers/new');
    await page.getByTestId('wizard-name').fill('Survie Rustic');
    await page.getByTestId('wizard-eula').click();
    await page.getByTestId('wizard-next').click();
    await page.getByTestId('wizard-next').click();
    await page.getByTestId('wizard-next').click();

    const toasts = page.getByRole('region', { name: /Notifications/ });
    await expect(toasts.getByText('Le serveur n’a pas pu être créé')).toBeVisible();
    await expect(
      toasts.getByText('Le programme rustic-server est introuvable. Choisissez-le dans Paramètres'),
    ).toBeVisible();
    await toasts.getByRole('button', { name: 'Ouvrir les paramètres' }).click();
    // Leaving the filled-in wizard still asks first.
    await page.getByRole('button', { name: 'Abandonner', exact: true }).click();
    await expect(page).toHaveURL(/section=game/);
    await expect(page).toHaveURL(/focus=rustic-server/);
    const card = page.getByTestId('settings-rustic-server');
    await expect(card.getByTestId('rustic-server-origin')).toHaveText('Introuvable');
    await expect(card).toBeInViewport();

    // Choosing the program fixes it.
    await card.getByRole('button', { name: 'Choisir…' }).click();
    await expect(card.getByTestId('rustic-server-origin')).toHaveText('Choisi manuellement');
  });

  test('VPS install happy path (mock)', async ({ app: page }) => {
    await page.goto('/#/servers/new');
    await page.getByTestId('wizard-name').fill('Survie VPS');
    await page.getByTestId('wizard-eula').click();
    await page.getByTestId('wizard-next').click();

    await page.getByTestId('hosting-vps').click();
    await expect(page.getByTestId('vps-setup')).toBeVisible();
    await page.getByRole('radio', { name: 'Add a VPS' }).click();
    await page.getByTestId('vps-host').fill('203.0.113.10');
    // Next is refused until the VPS is connected and checked.
    await page.getByTestId('wizard-next').click();
    await expect(page.getByText('Connect your VPS and pass the checks first.')).toBeVisible();

    await page.getByTestId('vps-generate-key').click();
    await expect(page.getByText('Run this once in your VPS console')).toBeVisible();
    await page.getByTestId('vps-test').click();

    const dialog = page.getByRole('dialog');
    await expect(dialog).toContainText('Trust 203.0.113.10?');
    await expect(page.getByTestId('vps-fingerprint')).toContainText('SHA256:');
    await page.getByTestId('vps-trust').click();
    await expect(page.getByTestId('vps-preflight')).toContainText('Your VPS is ready');
    await expect(page.getByTestId('vps-preflight')).toContainText('Ubuntu 24.04');
    await page.getByTestId('wizard-next').click();

    await expect(page.getByTestId('summary-name')).toHaveText('Survie VPS');
    const install = page.getByTestId('wizard-next');
    await expect(install).toHaveText(/Install on 203\.0\.113\.10/);
    await install.click();

    const progress = page.getByTestId('vps-progress');
    await expect(progress).toBeVisible();
    await expect(page.getByText('Survie VPS is running on 203.0.113.10')).toBeVisible({
      timeout: 30_000,
    });
    const states = await progress
      .locator('li[data-state]')
      .evaluateAll((els) => els.map((e) => e.getAttribute('data-state')));
    expect(states).toHaveLength(9);
    expect(states.every((s) => s === 'done' || s === 'skipped')).toBe(true);
    await progress.getByRole('button', { name: /Show log/ }).click();
    await expect(page.getByRole('log')).toContainText('systemctl start');

    await page.getByTestId('open-server').click();
    await expect(page).toHaveURL(/#\/servers\/srv_[a-z0-9]+/);
  });

  test('favourites in the multiplayer list', async ({ app: page }) => {
    await page.goto('/#/servers?tab=joined');
    const favorites = page.getByTestId('mp-favorites');
    const others = page.getByTestId('mp-others');
    await expect(favorites.getByText('Les Copains SMP', { exact: true })).toBeVisible();
    await expect(others.getByText('Skyreach', { exact: true })).toBeVisible();

    await page.getByTestId('mp-mp_sky').getByTestId('mp-favorite').click();
    await expect(favorites.getByText('Skyreach', { exact: true })).toBeVisible();
    await expect(page.getByTestId('mp-mp_sky').getByTestId('mp-favorite')).toHaveAttribute(
      'aria-pressed',
      'true',
    );

    await page.getByTestId('mp-mp_copains').getByTestId('mp-favorite').click();
    await expect(others.getByText('Les Copains SMP', { exact: true })).toBeVisible();

    // Ping and MOTD come from the server.
    await expect(page.getByTestId('mp-mp_arena')).toContainText('Ranked duels');
    await expect(page.getByTestId('mp-mp_old')).toContainText('Offline');
  });

  test('add a server to the list and join it', async ({ app: page }) => {
    await page.goto('/#/servers?tab=joined');
    await page.getByRole('button', { name: 'Add a server' }).click();
    await page.getByRole('textbox', { name: 'Server name' }).fill('My test');
    await page.getByRole('textbox', { name: 'Server address' }).fill('not a valid address');
    await page.getByRole('button', { name: 'Add', exact: true }).click();
    await expect(page.getByText('Use a host name or IP')).toBeVisible();
    await page.getByRole('textbox', { name: 'Server address' }).fill('mc.example.org:25570');
    await page.getByRole('button', { name: 'Add', exact: true }).click();
    await expect(page.getByTestId('mp-others').getByText('mc.example.org:25570')).toBeVisible();
    await page.getByRole('button', { name: 'Join My test' }).click();
    await expect(page.getByTestId('running-pill')).not.toContainText('0 instances', {
      timeout: 15_000,
    });
  });
});
