import { connect, go, shot } from './cdp.mjs';
const { browser, page } = await connect();
await go(page, '#/content?kind=resourcepack');
await page.waitForTimeout(1500);
// Modrinth only.
for (const s of ['horizon', 'rustic']) {
  const b = page.getByTestId(`source-${s}`);
  if ((await b.getAttribute('aria-pressed')) === 'true') await b.click();
}
await page.getByTestId('content-search').fill(process.env.Q ?? 'faithful');
await page.waitForTimeout(5000);
await shot(page, '75-explore-modrinth-resourcepacks');
const rows = page.locator('[data-testid^="content-row-"]');
console.log('rows', await rows.count());
console.log(
  (
    await rows
      .first()
      .innerText()
      .catch(() => 'none')
  ).slice(0, 300),
);
const install = rows.first().getByRole('button', { name: /^Install/ });
console.log(
  'install btns',
  await install.count(),
  await install
    .first()
    .isEnabled()
    .catch(() => false),
);
await install.first().click();
await page.waitForTimeout(8000);
await shot(page, '75-explore-installed');
console.log((await rows.first().innerText()).slice(0, 300));
await browser.close();
