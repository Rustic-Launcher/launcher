#!/usr/bin/env node
/**
 * Encodes the recorded journeys (<tmp>/rustic-journeys/<name>/frames.ffconcat, written by
 * e2e/video-kit.ts) into the website's media: 1920 × 1080, 60 fps constant frame rate
 * (screencast frames held until the next paint), H.264 MP4 (+faststart) and VP9 WebM, each kept
 * under 8 MB by raising the CRF if needed, a WebP poster, and 960 px AVIF + WebP posters for
 * phones. Needs ffmpeg on PATH.
 *
 *   node scripts/encode-videos.mjs [name ...]
 */
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, statSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
// Same default as e2e/video-kit.ts.
const FRAMES = process.env.RUSTIC_JOURNEYS_DIR ?? path.join(os.tmpdir(), 'rustic-journeys');
const OUT = path.resolve(HERE, '../../web/public/media');
const LIMIT = 8 * 1024 * 1024;
/** Where the poster is taken (seconds): after the title card, on the product. */
const POSTER_AT = { launch: 6.5, mod: 15, server: 40, cosmetic: 12, social: 63, console: 8 };

const ff = (args) => execFileSync('ffmpeg', ['-hide_banner', '-loglevel', 'error', '-y', ...args]);
const size = (f) => statSync(f).size;
const mb = (n) => `${(n / 1024 / 1024).toFixed(2)} MB`;

const input = (dir) => [
  '-f',
  'concat',
  '-safe',
  '0',
  '-i',
  path.join(dir, 'frames.ffconcat'),
  '-vf',
  // The screencast frames are full-range BT.601 JPEGs. Hardware VP9 decoders (D3D11 in Chrome on
  // Windows) refuse full-range streams with a decode error, and the browser does not then try
  // the next <source>: the video turns black. So: limited range, BT.709, tagged as such.
  'scale=1920:1080:flags=lanczos:in_range=pc:out_range=tv:in_color_matrix=bt601:out_color_matrix=bt709,fps=60,format=yuv420p',
  '-color_range',
  'tv',
  '-colorspace',
  'bt709',
  '-color_primaries',
  'bt709',
  '-color_trc',
  'bt709',
  '-an',
];

function mp4(dir, out) {
  for (const crf of [24, 27, 30, 33]) {
    ff([
      ...input(dir),
      '-c:v',
      'libx264',
      '-preset',
      'slow',
      '-crf',
      String(crf),
      '-tune',
      'animation',
      '-profile:v',
      'high',
      '-movflags',
      '+faststart',
      out,
    ]);
    if (size(out) <= LIMIT) return crf;
  }
  throw new Error(`${out} stays over 8 MB`);
}

function webm(dir, out) {
  for (const crf of [36, 40, 44, 48]) {
    ff([
      ...input(dir),
      '-c:v',
      'libvpx-vp9',
      '-crf',
      String(crf),
      '-b:v',
      '0',
      '-row-mt',
      '1',
      '-deadline',
      'good',
      '-cpu-used',
      '3',
      out,
    ]);
    if (size(out) <= LIMIT) return crf;
  }
  throw new Error(`${out} stays over 8 MB`);
}

mkdirSync(OUT, { recursive: true });
const names = process.argv.slice(2).length
  ? process.argv.slice(2)
  : readdirSync(FRAMES).filter((n) => existsSync(path.join(FRAMES, n, 'frames.ffconcat')));

for (const name of names) {
  const dir = path.join(FRAMES, name);
  const base = path.join(OUT, `journey-${name}`);
  const crfMp4 = mp4(dir, `${base}.mp4`);
  const crfWebm = webm(dir, `${base}.webm`);
  ff([
    '-ss',
    String(POSTER_AT[name] ?? 6),
    '-i',
    `${base}.mp4`,
    '-frames:v',
    '1',
    '-c:v',
    'libwebp',
    '-quality',
    '82',
    `${base}.webp`,
  ]);
  // Small posters for phones (the video loads only when tapped there): AVIF + WebP, 960 px.
  const small = ['-i', `${base}.webp`, '-vf'];
  ff([
    ...small,
    'scale=960:-2:flags=lanczos,format=yuv420p',
    '-frames:v',
    '1',
    '-c:v',
    'libaom-av1',
    '-still-picture',
    '1',
    '-crf',
    '32',
    '-cpu-used',
    '6',
    `${base}-960.avif`,
  ]);
  ff([
    ...small,
    'scale=960:-2:flags=lanczos',
    '-c:v',
    'libwebp',
    '-quality',
    '80',
    `${base}-960.webp`,
  ]);
  console.log(
    `${name}: mp4 ${mb(size(`${base}.mp4`))} (crf ${crfMp4}), webm ${mb(size(`${base}.webm`))} ` +
      `(crf ${crfWebm}), poster ${mb(size(`${base}.webp`))}`,
  );
}
