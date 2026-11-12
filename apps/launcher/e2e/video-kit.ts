/**
 * Recording kit for the product videos (`journeys-video.spec.ts`, encoded by
 * `scripts/encode-videos.mjs`). Records the page through the DevTools screencast (every painted
 * frame, timestamped), draws a synthetic cursor (smooth Bézier moves, hover, click ripple), types
 * like a person, zooms on details and shows title cards in the website's style. Everything shown
 * is the launcher's browser mock with its sample data; nothing comes from the game.
 */
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { CDPSession, Locator, Page } from '@playwright/test';

const HERE = path.dirname(fileURLToPath(import.meta.url));
/** Raw frames, outside test-results (Playwright empties that folder on every run). */
export const FRAMES_ROOT =
  process.env.RUSTIC_JOURNEYS_DIR ?? path.join(os.tmpdir(), 'rustic-journeys');
const MARK = readFileSync(path.resolve(HERE, '../../../assets/brand/rustic-mark.svg'), 'utf8');
/** The website's serif (OFL-1.1), inlined so the title cards match the site. */
const SERIF = readFileSync(
  path.resolve(
    HERE,
    '../../../packages/ui/node_modules/@fontsource/instrument-serif/files/instrument-serif-latin-400-italic.woff2',
  ),
).toString('base64');

/** Deterministic randomness: the same journey records the same way. */
function rng(seed: number) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Injected before the app: the cursor, the click ripple, title cards and zoom helpers. */
function overlayScript({ mark, serif }: { mark: string; serif: string }) {
  const install = () => {
    if (document.getElementById('rx-cursor')) return;
    const style = document.createElement('style');
    style.textContent = `
      @font-face{font-family:'Instrument Serif';font-style:italic;font-weight:400;
        src:url(data:font/woff2;base64,${serif}) format('woff2')}
      #rx-cursor{position:fixed;left:0;top:0;z-index:2147483647;pointer-events:none;
        transform:translate(-100px,-100px);transition:opacity .25s;filter:drop-shadow(0 2px 4px rgba(0,0,0,.55))}
      #rx-cursor svg{display:block;transition:transform .12s ease-out;transform-origin:3px 3px}
      #rx-cursor.down svg{transform:scale(.86)}
      .rx-ripple{position:fixed;z-index:2147483646;pointer-events:none;width:44px;height:44px;margin:-22px 0 0 -22px;
        border-radius:50%;border:2px solid #FF6A00;background:rgba(255,106,0,.18);animation:rx-ripple .6s cubic-bezier(.16,1,.3,1) forwards}
      @keyframes rx-ripple{from{transform:scale(.2);opacity:.9}to{transform:scale(1.6);opacity:0}}
      #rx-title{position:fixed;inset:0;z-index:2147483645;display:grid;place-items:center;pointer-events:none;
        background:#050505 radial-gradient(rgba(255,255,255,.06) 1px,transparent 1px) 0 0/24px 24px;
        opacity:0;transition:opacity .7s cubic-bezier(.16,1,.3,1);font-family:Inter,'Inter Variable',system-ui,sans-serif;color:#f3f1ee}
      #rx-title.on{opacity:1}
      #rx-title .in{display:flex;flex-direction:column;align-items:center;gap:22px;transform:translateY(14px);
        transition:transform 1s cubic-bezier(.16,1,.3,1)}
      #rx-title.on .in{transform:none}
      #rx-title .label{font:13px 'JetBrains Mono Variable',ui-monospace,monospace;color:#8c8882;letter-spacing:.02em;
        border:1px solid rgba(255,255,255,.07);border-radius:999px;padding:6px 12px;display:flex;gap:8px;align-items:center}
      #rx-title .dot{width:6px;height:6px;border-radius:50%;background:#FF6A00;box-shadow:0 0 10px #FF6A00}
      #rx-title h1{margin:0;font-size:76px;font-weight:650;letter-spacing:-.035em;line-height:1}
      #rx-title em{font-family:'Instrument Serif',Georgia,serif;font-weight:400;font-style:italic;letter-spacing:-.01em}
      #rx-title .ex{font:12px 'JetBrains Mono Variable',ui-monospace,monospace;color:#4c4844}
      #rx-root-zoom{transition:transform 1.1s cubic-bezier(.65,0,.35,1)}
    `;
    document.documentElement.appendChild(style);
    const cursor = document.createElement('div');
    cursor.id = 'rx-cursor';
    // An original arrow (not a system cursor asset).
    cursor.innerHTML =
      '<svg width="26" height="30" viewBox="0 0 26 30"><path d="M3 3 L3 24 L8.6 18.6 L12.6 27.4 L16.4 25.7 L12.5 17.1 L20.5 17.1 Z" fill="#fff" stroke="#0b0603" stroke-width="1.6" stroke-linejoin="round"/></svg>';
    document.documentElement.appendChild(cursor);
    window.addEventListener(
      'mousemove',
      (e) => (cursor.style.transform = `translate(${e.clientX - 3}px, ${e.clientY - 3}px)`),
      true,
    );
    window.addEventListener(
      'mousedown',
      (e) => {
        cursor.classList.add('down');
        const r = document.createElement('div');
        r.className = 'rx-ripple';
        r.style.left = `${e.clientX}px`;
        r.style.top = `${e.clientY}px`;
        document.documentElement.appendChild(r);
        setTimeout(() => r.remove(), 700);
      },
      true,
    );
    window.addEventListener('mouseup', () => cursor.classList.remove('down'), true);
    const title = document.createElement('div');
    title.id = 'rx-title';
    title.innerHTML = `<div class="in"><div style="width:64px;height:64px">${mark}</div>
      <span class="label"><span class="dot"></span><span data-k="label"></span></span>
      <h1 data-k="title"></h1><span class="ex" data-k="ex"></span></div>`;
    document.documentElement.appendChild(title);
  };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', install);
  else install();
}

export class Recorder {
  private cdp!: CDPSession;
  private frames: { file: string; t: number }[] = [];
  private dir: string;
  private pos = { x: 960, y: 620 };
  private rand: () => number;

  constructor(
    readonly page: Page,
    readonly name: string,
    seed = 7,
  ) {
    this.dir = path.join(FRAMES_ROOT, name);
    this.rand = rng(seed);
  }

  /** Call before the first navigation. */
  async prepare() {
    await this.page.addInitScript(overlayScript, { mark: MARK, serif: SERIF });
  }

  async start() {
    rmSync(this.dir, { recursive: true, force: true });
    mkdirSync(this.dir, { recursive: true });
    this.cdp = await this.page.context().newCDPSession(this.page);
    this.cdp.on('Page.screencastFrame', (f) => {
      const file = `f${String(this.frames.length).padStart(5, '0')}.jpg`;
      writeFileSync(path.join(this.dir, file), Buffer.from(f.data, 'base64'));
      this.frames.push({ file, t: f.metadata.timestamp ?? Date.now() / 1000 });
      void this.cdp.send('Page.screencastFrameAck', { sessionId: f.sessionId }).catch(() => {});
    });
    await this.cdp.send('Page.startScreencast', {
      format: 'jpeg',
      quality: 92,
      maxWidth: 1920,
      maxHeight: 1080,
      everyNthFrame: 1,
    });
    await this.page.mouse.move(this.pos.x, this.pos.y);
  }

  /** Stops and writes the ffmpeg concat list (each frame lasts until the next one). */
  async stop(tail = 0.6) {
    await this.hold(400);
    await this.cdp.send('Page.stopScreencast');
    const lines: string[] = ['ffconcat version 1.0'];
    this.frames.forEach((f, i) => {
      const next = this.frames[i + 1];
      const d = next ? Math.max(1 / 120, next.t - f.t) : tail;
      lines.push(`file '${f.file}'`, `duration ${d.toFixed(4)}`);
    });
    // The concat demuxer ignores the last duration unless the last file is repeated.
    const last = this.frames.at(-1);
    if (last) lines.push(`file '${last.file}'`);
    writeFileSync(path.join(this.dir, 'frames.ffconcat'), lines.join('\n') + '\n');
    return { frames: this.frames.length, dir: this.dir };
  }

  hold(ms: number) {
    return this.page.waitForTimeout(ms);
  }

  /** Moves the cursor along a gentle Bézier curve with ease-in-out timing. */
  async glide(x: number, y: number, ms = 750) {
    const from = { ...this.pos };
    const dx = x - from.x;
    const dy = y - from.y;
    const dist = Math.hypot(dx, dy);
    if (dist < 2) return;
    const bend = (this.rand() - 0.5) * Math.min(220, dist * 0.35);
    const nx = -dy / dist;
    const ny = dx / dist;
    const c1 = { x: from.x + dx * 0.3 + nx * bend, y: from.y + dy * 0.3 + ny * bend };
    const c2 = { x: from.x + dx * 0.75 + nx * bend * 0.4, y: from.y + dy * 0.75 + ny * bend * 0.4 };
    const steps = Math.max(12, Math.round(ms / 16));
    const ease = (t: number) => (t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2);
    for (let i = 1; i <= steps; i++) {
      const t = ease(i / steps);
      const u = 1 - t;
      const px = u ** 3 * from.x + 3 * u * u * t * c1.x + 3 * u * t * t * c2.x + t ** 3 * x;
      const py = u ** 3 * from.y + 3 * u * u * t * c1.y + 3 * u * t * t * c2.y + t ** 3 * y;
      await this.page.mouse.move(px, py);
      await this.page.waitForTimeout(8);
    }
    this.pos = { x, y };
  }

  private async centre(target: Locator) {
    await target.scrollIntoViewIfNeeded();
    const box = await target.boundingBox();
    if (!box) throw new Error(`no box for ${String(target)}`);
    return {
      x: box.x + box.width * (0.42 + this.rand() * 0.16),
      y: box.y + box.height * (0.4 + this.rand() * 0.2),
    };
  }

  async hover(target: Locator, ms = 700) {
    const c = await this.centre(target);
    await this.glide(c.x, c.y, ms);
    await this.hold(250);
  }

  async click(target: Locator, ms = 750) {
    const c = await this.centre(target);
    await this.glide(c.x, c.y, ms);
    await this.hold(140 + this.rand() * 120);
    await this.page.mouse.down();
    await this.hold(90);
    await this.page.mouse.up();
    await this.hold(260);
  }

  /** Clicks into a field and types with human, slightly uneven rhythm. */
  async type(target: Locator, text: string) {
    await this.click(target);
    for (const ch of text) {
      await this.page.keyboard.type(ch);
      await this.hold(ch === ' ' ? 110 + this.rand() * 90 : 45 + this.rand() * 85);
    }
    await this.hold(300);
  }

  /** Zooms the app on a region (cursor hidden meanwhile), holds, then zooms back. */
  async zoom(target: Locator, scale = 1.35, holdMs = 1600) {
    await target.scrollIntoViewIfNeeded();
    const box = await target.boundingBox();
    if (!box) return;
    const cx = box.x + box.width / 2;
    const cy = box.y + box.height / 2;
    await this.page.evaluate(
      ({ cx, cy, scale }) => {
        const root = document.getElementById('root');
        const cursor = document.getElementById('rx-cursor');
        if (!root || !cursor) return;
        root.id = 'root';
        root.style.transition = 'transform 1.1s cubic-bezier(.65,0,.35,1)';
        root.style.transformOrigin = `${cx}px ${cy}px`;
        root.style.transform = `scale(${scale})`;
        cursor.style.opacity = '0';
      },
      { cx, cy, scale },
    );
    await this.hold(1100 + holdMs);
    await this.page.evaluate(() => {
      const root = document.getElementById('root');
      const cursor = document.getElementById('rx-cursor');
      if (root) root.style.transform = '';
      if (cursor) cursor.style.opacity = '1';
    });
    await this.hold(1150);
  }

  /** A title card in the website's style, faded in over the app and out again. */
  async title(label: string, title: string, holdMs = 1900) {
    await this.page.evaluate(
      ({ label, title }) => {
        const el = document.getElementById('rx-title');
        if (!el) return;
        const set = (k: string, v: string) => {
          const n = el.querySelector(`[data-k="${k}"]`);
          if (n) n.innerHTML = v;
        };
        set('label', label);
        set('title', title);
        set('ex', 'example data · Rustic Launcher preview');
        el.classList.add('on');
      },
      { label, title },
    );
    await this.hold(700 + holdMs);
    await this.page.evaluate(() => document.getElementById('rx-title')?.classList.remove('on'));
    await this.hold(750);
  }

  /** Instantly covers the screen with the title background (scene cut), then reveals. */
  async cut(fn: () => Promise<void>) {
    await this.page.evaluate(() => {
      const el = document.getElementById('rx-title');
      if (!el) return;
      el.querySelector('.in')?.setAttribute('style', 'opacity:0');
      el.classList.add('on');
    });
    await this.hold(750);
    await fn();
    await this.page.evaluate(() => {
      const el = document.getElementById('rx-title');
      el?.classList.remove('on');
      el?.querySelector('.in')?.removeAttribute('style');
    });
    await this.hold(750);
  }
}
