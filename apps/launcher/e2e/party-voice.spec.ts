import type { Browser, Page } from '@playwright/test';
import { expect, seed, test } from './fixtures';

/**
 * Parties and voice (ADR 0176) against the browser mock: the Party tab (members, leader actions,
 * invites, chat, proximity setting) and voice's errors. With a local LiveKit
 * (`livekit-server --dev`, `VITE_VOICE_DEV_URL=ws://localhost:7880` when the dev server starts),
 * two launchers join the party's room for real: they see and hear each other (Chromium's fake
 * microphone), mute shows on the other side, and proximity mixes them from the game's positions.
 */

// Chromium's fake microphone (a beep) for the live test; harmless for the others.
test.use({
  launchOptions: {
    args: ['--use-fake-ui-for-media-stream', '--use-fake-device-for-media-stream'],
  },
});

async function openParty(app: Page) {
  await app.getByTestId('open-social').click();
  const social = app.getByTestId('social-overlay');
  await expect(social).toBeVisible();
  await social.getByTestId('social-tab-party').click();
  return social;
}

test.describe('party', () => {
  test('members, chat, proximity, leave, create, invites', async ({ app }) => {
    const social = await openParty(app);
    await expect(social.getByTestId('party-member-samcrafte')).toContainText('Sam');
    await expect(social.getByTestId('party-member-lea_builds')).toBeVisible();
    // The party chat: Léa's line, then ours.
    const chat = social.getByTestId('party-chat');
    await expect(chat).toContainText('On se retrouve au spawn ?');
    await social.getByTestId('party-chat-input').fill('Je arrive !');
    await social.getByTestId('party-chat-input').press('Enter');
    await expect(chat).toContainText('Je arrive !');
    await app.evaluate(() =>
      (
        window as unknown as { __rusticSocialMock: { partySay: (a: string, b: string) => void } }
      ).__rusticSocialMock.partySay('u_lea', 'Top'),
    );
    await expect(chat).toContainText('Top');
    // The leader turns proximity on: the range appears.
    const proximity = social.getByTestId('party-proximity');
    await proximity.getByRole('switch').click();
    await expect(proximity.getByRole('slider')).toBeVisible();
    // Voice is not available without an SFU (the mock has none by default).
    if (!process.env.VITE_VOICE_DEV_URL) {
      await social.getByTestId('voice-join').click();
      await expect(social.getByTestId('voice-error')).toContainText('not available');
    }
    // Leave, then create a new party.
    await social.getByTestId('party-leave').click();
    await expect(social.getByText('No party yet')).toBeVisible();
    await expect(social.getByTestId('party-chat-empty')).toBeVisible();
    await social.getByTestId('party-create').click();
    await expect(social.getByTestId('party-member-samcrafte')).toContainText('You');
    await social.getByTestId('party-leave').click();
    // A friend's invite arrives: accept it.
    await app.evaluate(() =>
      (
        window as unknown as { __rusticSocialMock: { partyInvite: (a: string) => void } }
      ).__rusticSocialMock.partyInvite('u_max'),
    );
    const invite = social.getByTestId('party-invite-MaxRedstone');
    await expect(invite).toContainText('invites you');
    await invite.getByRole('button', { name: 'Join' }).click();
    await expect(social.getByTestId('party-member-MaxRedstone')).toBeVisible();
    await expect(invite).toHaveCount(0);
  });

  test('settings: voice devices, push-to-talk key and server keys', async ({ app }) => {
    await app.goto('/#/settings?section=voice');
    await expect(app.getByTestId('settings-voice')).toBeVisible();
    await app.getByRole('combobox', { name: 'Input mode' }).click();
    await app.getByRole('option', { name: 'Push to talk' }).click();
    await app.getByTestId('voice-ptt-key').click();
    await expect(app.getByTestId('voice-ptt-key')).toContainText('Press a key');
    await app.keyboard.press('KeyB');
    await expect(app.getByTestId('voice-ptt-key')).toHaveText('B');
    // Registering a server for proximity voice shows its config lines once.
    const servers = app.getByTestId('settings-voice-servers');
    await servers.getByRole('textbox').fill('Cosy SMP');
    await servers.getByRole('button', { name: 'Add' }).click();
    await expect(app.getByTestId('voice-server-key')).toContainText('server_id = "vsrv_');
    await expect(app.getByTestId('voice-server-key')).toContainText('key = "rvk_');
    await app.getByRole('button', { name: 'Done' }).click();
    await expect(servers).toContainText('Cosy SMP');
  });
});

const live = !!process.env.VITE_VOICE_DEV_URL;

/** `RoomService/RemoveParticipant` on the dev server (keys `devkey` / `secret`). */
async function removeParticipant(room: string, identity: string): Promise<number> {
  const { createHmac } = await import('node:crypto');
  const b64 = (v: unknown) => Buffer.from(JSON.stringify(v)).toString('base64url');
  const now = Math.floor(Date.now() / 1000);
  const body = `${b64({ alg: 'HS256', typ: 'JWT' })}.${b64({
    iss: 'devkey',
    sub: 'e2e',
    nbf: now - 10,
    exp: now + 60,
    video: { room, roomAdmin: true },
  })}`;
  const token = `${body}.${createHmac('sha256', 'secret').update(body).digest('base64url')}`;
  const base = process.env.VITE_VOICE_DEV_URL!.replace(/^ws/, 'http');
  const r = await fetch(`${base}/twirp/livekit.RoomService/RemoveParticipant`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ room, identity }),
  });
  return r.status;
}

async function launcher(browser: Browser, identity: string, name: string) {
  const ctx = await browser.newContext({
    permissions: ['microphone'],
    viewport: { width: 1440, height: 900 },
  });
  const page = await ctx.newPage();
  await seed(page);
  await page.addInitScript(
    ([id, n]) => {
      window.localStorage.setItem('rustic.mock.voiceIdentity', id!);
      window.localStorage.setItem('rustic.mock.voiceName', n!);
    },
    [identity, name],
  );
  await page.goto('/');
  await expect(page.getByTestId('rail-home')).toBeVisible({ timeout: 30_000 });
  return page;
}

test.describe('party voice on a local LiveKit', () => {
  test.skip(!live, 'set VITE_VOICE_DEV_URL and run livekit-server --dev');

  test('two launchers talk, mute and mix by distance', async ({ browser }) => {
    test.setTimeout(120_000);
    const sam = await launcher(browser, 'usr_sam', 'Sam');
    const lea = await launcher(browser, 'u_lea', 'Léa');
    const samSocial = await openParty(sam);
    const leaSocial = await openParty(lea);
    await samSocial.getByTestId('voice-join').click();
    await leaSocial.getByTestId('voice-join').click();
    // Each sees the other in the room.
    await expect(samSocial.getByTestId('voice-member-u_lea')).toContainText('Léa', {
      timeout: 30_000,
    });
    await expect(leaSocial.getByTestId('voice-member-usr_sam')).toContainText('Sam', {
      timeout: 30_000,
    });
    // The fake microphone's beeps open the voice gate, and the SFU hears them.
    await expect(samSocial.getByTestId('voice-self-state')).toHaveText('Speaking', {
      timeout: 20_000,
    });
    await expect(leaSocial.getByTestId('voice-member-usr_sam')).toContainText('Speaking', {
      timeout: 20_000,
    });
    // Mute shows on the other side.
    await samSocial.getByTestId('voice-mute').click();
    await expect(leaSocial.getByTestId('voice-member-usr_sam')).toContainText('Muted', {
      timeout: 15_000,
    });
    await samSocial.getByTestId('voice-mute').click();
    // Proximity: the leader turns it on, both launchers are in game on the same server.
    // (Each browser has its own mock cloud: the setting is turned on in both, as the real cloud's
    // party update would do.)
    await samSocial.getByTestId('party-proximity').getByRole('switch').click();
    await leaSocial.getByTestId('party-proximity').getByRole('switch').click();
    const at = (page: Page, x: number) =>
      page.evaluate(
        (px) =>
          (
            window as unknown as { __rusticVoiceGame: (e: Record<string, unknown>) => void }
          ).__rusticVoiceGame({
            place: 'server',
            server: 'play.example.org',
            position: {
              server: 'play.example.org',
              dim: 'minecraft:overworld',
              x: px,
              y: 64,
              z: 0,
              yaw: 0,
            },
          }),
        x,
      );
    await at(sam, 0);
    await at(lea, 500);
    await expect(samSocial.getByTestId('voice-member-u_lea')).toContainText('Out of range', {
      timeout: 15_000,
    });
    await at(lea, 5);
    await expect(samSocial.getByTestId('voice-member-u_lea')).not.toContainText('Out of range', {
      timeout: 15_000,
    });
    // The in-game overlay's state: in the room, positions on, the members listed.
    const overlay = await sam.evaluate(
      () =>
        (window as unknown as { __rusticVoiceOverlay?: Record<string, unknown> })
          .__rusticVoiceOverlay,
    );
    expect(overlay).toMatchObject({ active: true, positions: true, you: 'You' });
    // A kick through LiveKit's server API, as the cloud does it (RoomService/RemoveParticipant
    // with a room-admin token): Léa is out and told why.
    const removed = await removeParticipant('party_party_1', 'u_lea');
    expect(removed).toBe(200);
    await expect(leaSocial.getByTestId('voice-error')).toContainText(
      'removed from the voice room',
      {
        timeout: 15_000,
      },
    );
    await expect(samSocial.getByTestId('voice-member-u_lea')).toHaveCount(0, { timeout: 15_000 });
    await samSocial.getByTestId('voice-leave').click();
    await expect(samSocial.getByTestId('voice-join')).toBeVisible();
  });
});
