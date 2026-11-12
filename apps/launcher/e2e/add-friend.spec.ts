import type { Page } from '@playwright/test';
import { expect, test } from './fixtures';

async function openDialog(app: Page) {
  await app.getByTestId('open-social').click();
  await app.getByTestId('add-friend').click();
  const dialog = app.getByRole('dialog', { name: 'Add a friend' });
  await expect(dialog).toBeVisible();
  return {
    dialog,
    input: dialog.getByTestId('add-friend-input'),
    card: dialog.getByTestId('add-friend-result'),
    error: dialog.getByTestId('add-friend-error'),
  };
}

/** The "Add a friend" dialog (ADR 0145) against the browser mock. */
test.describe('add a friend', () => {
  // Several lookups and requests per test on a cold dev server: the slow-test budget.
  test.slow();

  test('by Rustic name, then by Minecraft name: a result card, then the request', async ({
    app,
  }) => {
    const { dialog, input, card } = await openDialog(app);
    await expect(input).toBeFocused();
    await input.fill('ninasky');
    await input.press('Enter');
    await expect(card).toContainText('Nina');
    await expect(card).toContainText('@NinaSky');
    await expect(card).toContainText('Nina_Sky');
    await card.getByRole('button', { name: 'Send request' }).click();
    await expect(card.getByRole('status')).toHaveText('Request sent');

    await dialog.getByRole('radio', { name: 'Minecraft name' }).click();
    await expect(input).toHaveAttribute('placeholder', 'e.g. LeaBuilds');
    await input.fill('RayanTheBold');
    await dialog.getByRole('button', { name: 'Find' }).click();
    await expect(card).toContainText('@RayanPvP');
    await card.getByRole('button', { name: 'Send request' }).click();
    await expect(card.getByRole('status')).toHaveText('Request sent');
  });

  test('errors, and an incoming request offered for acceptance', async ({ app }) => {
    const { dialog, input, card, error } = await openDialog(app);
    await dialog.getByRole('radio', { name: 'Minecraft name' }).click();
    await input.fill('Nobody_Here');
    await input.press('Enter');
    await expect(error).toHaveText('No Rustic player has linked this Minecraft name.');
    await expect(input).toHaveAttribute('aria-invalid', 'true');
    await dialog.getByRole('radio', { name: 'Rustic name' }).click();
    await input.fill('a b');
    await input.press('Enter');
    await expect(error).toHaveText('Rustic names are 3 to 20 letters, digits or _.');
    await input.fill('kev_grief');
    await input.press('Enter');
    await expect(error).toHaveText('This player can’t receive friend requests.');
    await input.fill('samcrafte');
    await input.press('Enter');
    await expect(error).toHaveText('That’s you!');

    await input.fill('HugoMines');
    await input.press('Enter');
    await expect(card).toContainText('They already asked to be your friend');
    await card.getByRole('button', { name: 'Accept' }).click();
    await expect(card).toContainText('Already friends');
  });

  test('suggestions: why and when, "Request sent", gone after reopening', async ({ app }) => {
    const { dialog } = await openDialog(app);
    const played = dialog.getByTestId('suggestions-played-with');
    const alex = played.getByTestId('suggestion-alex_rs');
    await expect(alex).toContainText('In your party');
    await expect(played.getByTestId('suggestion-yanis_b')).toContainText('On Les Copains SMP');
    await expect(dialog.getByTestId('suggestions-crossed')).toContainText('On Skyreach');
    await alex.getByRole('button', { name: 'Send a friend request to Alex' }).click();
    await expect(alex.getByRole('status')).toHaveText('Request sent');

    // Escape closes; reopening drops the player already asked.
    await app.keyboard.press('Escape');
    await expect(dialog).toBeHidden();
    await app.getByTestId('add-friend').click();
    await expect(dialog.getByTestId('suggestion-yanis_b')).toBeVisible();
    await expect(dialog.getByTestId('suggestion-alex_rs')).toHaveCount(0);
  });

  test('opens from the command menu; suggestions follow the privacy setting', async ({ app }) => {
    await app.keyboard.press('Control+k');
    await app.getByPlaceholder('Search content, friends, servers, settings…').fill('add a friend');
    await app.getByRole('option', { name: /Add a friend/ }).click();
    const dialog = app.getByRole('dialog', { name: 'Add a friend' });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByTestId('suggestion-alex_rs')).toBeVisible();
    await app.keyboard.press('Escape');

    await app.goto('/#/settings?section=privacy');
    const toggle = app.getByTestId('settings-discoverable');
    // The settings chunk is compiled on first visit by the dev server: allow for it.
    await expect(toggle).toBeChecked({ timeout: 45_000 });
    await toggle.click();
    await expect(toggle).not.toBeChecked();

    await app.keyboard.press('Control+k');
    await app.getByPlaceholder('Search content, friends, servers, settings…').fill('add a friend');
    await app.getByRole('option', { name: /Add a friend/ }).click();
    await expect(dialog.getByTestId('suggestions-off')).toContainText('Suggestions are off');
    await expect(dialog.getByTestId('suggestion-alex_rs')).toHaveCount(0);
  });
});
