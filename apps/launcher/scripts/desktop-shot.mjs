#!/usr/bin/env node
/* global window */
// Screenshots the real desktop build (WebView2) over CDP.
// Start the app with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222, then:
//   node scripts/desktop-shot.mjs <out.png> [hash]
import { chromium } from '@playwright/test';

const [out = 'desktop-launchpad.png', hash = ''] = process.argv.slice(2);
const browser = await chromium.connectOverCDP('http://127.0.0.1:9222');
const ctx = browser.contexts()[0];
let page = ctx?.pages().find((p) => !p.url().includes('#/social')) ?? ctx?.pages()[0];
for (let i = 0; !page && i < 20; i++) {
  await new Promise((r) => setTimeout(r, 500));
  page = browser.contexts()[0]?.pages()[0];
}
if (!page) throw new Error('no WebView2 page over CDP');
if (hash) await page.evaluate((h) => (window.location.hash = h), hash);
await page.waitForTimeout(4000);
console.log('url', page.url(), 'title', await page.title());
console.log('native', await page.evaluate(() => '__TAURI_INTERNALS__' in window));
await page.screenshot({ path: out });
console.log('saved', out);
await browser.close();
