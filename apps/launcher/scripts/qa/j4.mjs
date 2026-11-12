import { connect, go, shot } from './cdp.mjs';
const { browser, page } = await connect();
await go(page, '#/');
await page.waitForTimeout(2500);
await shot(page, '74-launchpad-ready');
await page.getByTestId('play-button').click();
await page.waitForTimeout(5000);
await shot(page, '74-launch-gated');
console.log(
  (await page.locator('[role=region] li, ol li').allInnerTexts()).join(' | ').slice(0, 600),
);
console.log('status', await page.getByTestId('play-status').innerText());
await browser.close();
