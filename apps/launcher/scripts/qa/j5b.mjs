import { connect, go, shot } from './cdp.mjs';
const { browser, page } = await connect();
await go(page, '#/instances/qa-survie?tab=resourcepacks');
await page.waitForTimeout(2500);
await shot(page, '75-instance-resourcepack-installed');
await browser.close();
