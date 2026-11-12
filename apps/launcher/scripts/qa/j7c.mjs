// Social journey on the real Social window: open it, then report what it shows (live updates).
import { connect, shot } from './cdp.mjs';
const { browser, page, pages } = await connect();
const [step, name] = process.argv.slice(2);
let social = pages().find((p) => p.url().includes('#/social'));
if (!social) {
  await page.getByTestId('open-social').click();
  for (let i = 0; i < 20 && !social; i++) {
    await page.waitForTimeout(500);
    social = pages().find((p) => p.url().includes('#/social'));
  }
  await social.waitForTimeout(2500);
}
if (step === 'request') {
  await social.getByTestId('add-friend').click();
  await social.getByPlaceholder('Rustic username').fill('QaLea');
  await social.getByRole('button', { name: 'Send', exact: true }).click();
  await social.waitForTimeout(2000);
}
if (step === 'chat') {
  await social.getByTestId('social-tab-chats').click();
  await social.waitForTimeout(800);
  await social.getByTestId('chat-QaLea').click();
  await social.waitForTimeout(1500);
}
if (step === 'wait') await social.waitForTimeout(4000);
if (step === 'reply') {
  await social.getByTestId('chat-input').fill(name);
  await social.getByTestId('chat-input').press('Enter');
  await social.waitForTimeout(2500);
}
if (name && step !== 'reply') await shot(social, name);
console.log((await social.locator('body').innerText()).slice(0, 700));
await browser.close();
