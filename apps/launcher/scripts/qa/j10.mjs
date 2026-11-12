// After the 15-minute access token expired: the relaunched app must refresh once and keep working.
import { connect, go, shot } from './cdp.mjs';
const { browser, page } = await connect();
const errors = [];
page.on('console', (m) => m.type() === 'error' && errors.push(m.text().slice(0, 120)));
await page.reload();
await page.waitForTimeout(3000);
await page.getByTestId('open-social').click();
await page.waitForTimeout(4000);
await go(page, '#/');
await page.waitForTimeout(3000);
const text = await page.locator('section[aria-labelledby="aside-friends"]').innerText();
console.log('friends panel:', JSON.stringify(text.slice(0, 200)));
console.log('401 errors:', errors.filter((e) => /401|Authentication/.test(e)).length);
await shot(page, '72-rustic-session-after-refresh');
await browser.close();
