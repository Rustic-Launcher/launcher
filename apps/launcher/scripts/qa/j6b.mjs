import { connect, go, shot } from './cdp.mjs';
const { browser, page } = await connect();
await go(page, '#/servers/qa-horizon');
await page.waitForTimeout(1500);
await page.getByTestId('server-start').click();
for (let i = 0; i < 90; i++) {
  const s = await page
    .getByTestId('server-state')
    .innerText()
    .catch(() => '');
  if (/online|running|en ligne/i.test(s)) break;
  await page.waitForTimeout(1000);
}
await page.waitForTimeout(12000);
console.log('state', await page.getByTestId('server-state').innerText());
await shot(page, '76-server-running-overview');
console.log((await page.locator('main').innerText()).slice(0, 500));
await browser.close();
