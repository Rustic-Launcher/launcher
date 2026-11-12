/* global document */
import { connect, go, shot } from './cdp.mjs';
const { browser, page } = await connect();
await page.keyboard.press('Escape');
await go(page, '#/settings?section=appearance');
await page.waitForTimeout(1500);
const attrs = () =>
  page.evaluate(() => [
    document.documentElement.dataset.theme,
    document.documentElement.dataset.palette,
  ]);
console.log('start', await attrs());
await page
  .getByRole('radio', { name: /^Light/ })
  .first()
  .click();
await page.waitForTimeout(800);
await page
  .getByRole('radio', { name: /Verdigris/ })
  .first()
  .click();
await page.waitForTimeout(1000);
console.log('light+verdigris', await attrs());
await shot(page, '79-settings-light-verdigris');
await page.getByRole('radio', { name: /^Dark/ }).first().click();
await page
  .getByRole('radio', { name: /Heather/ })
  .first()
  .click();
await page.waitForTimeout(1000);
console.log('dark+heather', await attrs());
await shot(page, '79-settings-dark-heather');
// persisted across a reload?
await page.reload();
await page.waitForTimeout(2500);
console.log('after reload', await attrs());
await page.getByRole('radio', { name: /Ember/ }).first().click();
await page.waitForTimeout(800);
console.log('back to ember', await attrs());
await shot(page, '79-settings-dark-ember');
await browser.close();
