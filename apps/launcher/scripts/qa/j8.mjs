import { connect, go, shot } from './cdp.mjs';
const { browser, page } = await connect();
await go(page, '#/store');
await page.waitForTimeout(3000);
await shot(page, '78-store');
const PID = process.argv[2];
console.log('products', await page.locator('[data-testid^="product-"]').count());
await page.keyboard.press('Escape');
await go(page, '#/store/' + PID);
await page.waitForTimeout(2000);
await page.getByTestId('product-buy').click();
await page.waitForTimeout(1500);
const dlg = page.getByTestId('purchase-dialog');
for (const id of ['consent-immediate', 'consent-waiver']) {
  if (await dlg.getByTestId(id).count()) await dlg.getByTestId(id).click();
}
await shot(page, '78-store-purchase-dialog');
await page.getByTestId('purchase-pay').click();
await page.waitForTimeout(4000);
await shot(page, '78-store-purchase-error');
console.log(
  (
    await page
      .getByRole('dialog')
      .innerText()
      .catch(() => '')
  ).slice(0, 800),
);
console.log(
  'toasts',
  await page.locator('[role="status"], [data-sonner-toast], [role="alert"]').allInnerTexts(),
);
await browser.close();
