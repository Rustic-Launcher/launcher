import { connect, shot } from './cdp.mjs';
const { browser, page } = await connect();
await page.getByRole('button', { name: 'Sign in with Microsoft' }).first().click();
await page.waitForTimeout(3000);
await shot(page, '70-onboarding-3-microsoft-gated');
console.log((await page.locator('main').innerText()).slice(200, 900));
await browser.close();
