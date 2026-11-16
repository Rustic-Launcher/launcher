/**
 * Dev only (browser mock): a LiveKit access token signed in the page for a local
 * `livekit-server --dev` (key `devkey`, secret `secret`), so the voice engine can be tried and
 * tested end to end without the cloud. Set `VITE_VOICE_DEV_URL=ws://localhost:7880`. The real
 * cloud signs tokens on the server (`services/api/src/livekit.rs`); this never ships a secret
 * of any real deployment.
 */

const b64url = (bytes: Uint8Array): string =>
  btoa(String.fromCharCode(...bytes))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');

const enc = (v: unknown) => b64url(new TextEncoder().encode(JSON.stringify(v)));

/** An HS256 JWT with LiveKit's claims for `room`. */
export async function devLiveKitToken(opts: {
  key: string;
  secret: string;
  room: string;
  identity: string;
  name: string;
  ttlSeconds?: number;
}): Promise<{ token: string; expires_at: string }> {
  const now = Math.floor(Date.now() / 1000);
  const exp = now + (opts.ttlSeconds ?? 600);
  const header = enc({ alg: 'HS256', typ: 'JWT' });
  const payload = enc({
    iss: opts.key,
    sub: opts.identity,
    jti: opts.identity,
    nbf: now - 10,
    exp,
    name: opts.name,
    video: {
      room: opts.room,
      roomJoin: true,
      canPublish: true,
      canSubscribe: true,
      canPublishData: true,
      canPublishSources: ['microphone'],
    },
  });
  const key = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(opts.secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign'],
  );
  const sig = new Uint8Array(
    await crypto.subtle.sign('HMAC', key, new TextEncoder().encode(`${header}.${payload}`)),
  );
  return {
    token: `${header}.${payload}.${b64url(sig)}`,
    expires_at: new Date(exp * 1000).toISOString(),
  };
}
