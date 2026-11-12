import { connect, shot } from './cdp.mjs';
const { browser, page } = await connect();
await page.getByTestId('rustic-email').fill(process.env.QA_EMAIL ?? 'qa.sam@example.com');
await page.getByTestId('rustic-password').fill('Rustic-QA-pass-2026');
await page.getByTestId('rustic-auth-submit').click();
await page.waitForTimeout(4000);
await shot(page, '72-rustic-signed-in');
console.log((await page.getByTestId('settings-rustic').innerText()).slice(0, 300));
await browser.close();
