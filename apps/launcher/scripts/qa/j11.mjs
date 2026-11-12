// Release build smoke test: the installed app starts on onboarding, then the Launchpad renders.
import { connect, ipc, shot } from './cdp.mjs';
const { browser, page } = await connect();
await page.waitForTimeout(2000);
await shot(page, '80-release-installed-first-run');
console.log(
  'first screen:',
  (await page.locator('body').innerText()).slice(0, 160).replace(/\n/g, ' | '),
);
console.log(
  'version:',
  JSON.stringify(await ipc(page, 'app_info').catch((e) => e.message)).slice(0, 200),
);
await browser.close();
