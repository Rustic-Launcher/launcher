#!/usr/bin/env node
/**
 * Exports launcher screenshots (docs/screenshots, captured by `pnpm screenshots` on the browser
 * mock with its sample data) as AVIF + WebP for the website (apps/web/public/media/shots), at
 * 1440 px and 720 px wide. Needs ffmpeg with libaom-av1 and libwebp.
 *
 *   node scripts/export-stills.mjs
 */
import { execFileSync } from 'node:child_process';
import { mkdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SRC = path.resolve(HERE, '../../../docs/screenshots');
const OUT = path.resolve(HERE, '../../web/public/media/shots');
/** Screenshot → published name. */
const STILLS = {
  '01-home.png': 'launchpad',
  '06-content-mods.png': 'explore',
  '13-server-overview.png': 'server',
  '20-locker.png': 'locker',
  '28-social-chat.png': 'social',
};

const ff = (args) => execFileSync('ffmpeg', ['-hide_banner', '-loglevel', 'error', '-y', ...args]);
mkdirSync(OUT, { recursive: true });
for (const [file, name] of Object.entries(STILLS)) {
  const input = path.join(SRC, file);
  for (const w of [1440, 720]) {
    const base = path.join(OUT, `${name}-${w}`);
    const scale = `scale=${w}:-2:flags=lanczos`;
    ff(['-i', input, '-vf', scale, '-c:v', 'libwebp', '-quality', '84', `${base}.webp`]);
    ff([
      '-i',
      input,
      '-vf',
      `${scale},format=yuv420p`,
      '-frames:v',
      '1',
      '-c:v',
      'libaom-av1',
      '-still-picture',
      '1',
      '-crf',
      '30',
      '-cpu-used',
      '6',
      `${base}.avif`,
    ]);
    const kb = (f) => `${Math.round(statSync(f).size / 1024)} KB`;
    console.log(`${name}-${w}: webp ${kb(`${base}.webp`)}, avif ${kb(`${base}.avif`)}`);
  }
}
