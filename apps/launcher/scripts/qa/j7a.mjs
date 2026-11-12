import { connect, go, shot } from './cdp.mjs';
const { browser, page } = await connect();
await go(page, '#/');
await page.waitForTimeout(2000);
await page.getByTestId('open-social').click();
await page.waitForTimeout(2500);
const pages = browser.contexts().flatMap((c) => c.pages());
console.log(
  'pages',
  pages.map((p) => p.url()),
);
const social = pages.find((p) => p.url().includes('#/social'));
const target = social ?? page;
await target.waitForTimeout(1500);
await target.getByTestId('add-friend').click();
await target.getByPlaceholder('Rustic username').fill('QaLea');
await target.getByRole('button', { name: 'Send', exact: true }).click();
await target.waitForTimeout(2500);
await shot(target, '77-social-window-request-sent');
console.log((await target.locator('body').innerText()).slice(0, 700));
await browser.close();
