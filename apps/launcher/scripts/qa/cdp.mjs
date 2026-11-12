// QA helpers: drive the real desktop build (WebView2) over CDP. Start the app with
// WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 (see docs/QA_REPORT.md).
/* global window */
import { chromium } from '@playwright/test';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const SHOTS = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '../../../../docs/screenshots',
);

export async function connect() {
  const browser = await chromium.connectOverCDP('http://127.0.0.1:9222');
  const pages = () => browser.contexts().flatMap((c) => c.pages());
  let page;
  for (let i = 0; i < 40 && !page; i++) {
    page = pages().find((p) => !p.url().includes('#/social'));
    if (!page) await new Promise((r) => setTimeout(r, 500));
  }
  if (!page) throw new Error('no WebView2 page over CDP');
  page.setDefaultTimeout(20_000);
  return { browser, page, pages };
}

export async function go(page, hash) {
  await page.evaluate((h) => (window.location.hash = h), hash);
  await page.waitForTimeout(1200);
}

export async function shot(page, name) {
  await page.waitForTimeout(600);
  const file = path.join(SHOTS, `${name}.png`);
  await page.screenshot({ path: file });
  console.log('shot', file);
}

/** Calls a Tauri command directly (setup steps that have no UI, e.g. skipping onboarding). */
export function ipc(page, cmd, args) {
  return page
    .evaluate(
      ([c, a]) =>
        window.__TAURI_INTERNALS__
          .invoke(c, a === undefined ? undefined : { args: a })
          .catch((e) => ({ __error: e })),
      [cmd, args],
    )
    .then((r) => {
      if (r && typeof r === 'object' && '__error' in r) throw new Error(JSON.stringify(r.__error));
      return r;
    });
}
