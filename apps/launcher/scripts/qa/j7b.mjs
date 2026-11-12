import { connect, shot } from './cdp.mjs';
const { browser, pages } = await connect();
const social = pages().find((p) => p.url().includes('#/social'));
if (!social) throw new Error('social window not open');
const step = process.argv[2];
if (step === 'friends') {
  await social.waitForTimeout(3000);
  await shot(social, '77-social-friend-accepted');
  console.log((await social.locator('body').innerText()).slice(0, 500));
}
if (step === 'open-chat') {
  await social.getByText('QaLea', { exact: true }).first().click();
  await social.waitForTimeout(2000);
  console.log((await social.locator('body').innerText()).slice(0, 600));
}
if (step === 'dm') {
  await social.waitForTimeout(4000);
  await shot(social, '77-social-dm-live');
  console.log((await social.locator('body').innerText()).slice(0, 800));
}
await browser.close();
