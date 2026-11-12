import { connect, go } from './cdp.mjs';
const { browser, page } = await connect();
await page.keyboard.press('Escape');
await go(page, '#/settings?section=accounts');
await page.waitForTimeout(1500);
await browser.close();
