import { connect, go, shot } from './cdp.mjs';
const { browser, page } = await connect();
await go(page, '#/servers/qa-horizon');
await page.waitForTimeout(8000);
await shot(page, '76-server-running-overview');
await browser.close();
