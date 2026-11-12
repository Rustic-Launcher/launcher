import { connect, go, shot } from './cdp.mjs';
const { browser, page } = await connect();
const step = process.argv[2];
await go(page, '#/servers/qa-horizon');
await page.waitForTimeout(1500);
const waitState = async (re, secs = 120) => {
  for (let i = 0; i < secs; i++) {
    const s = await page
      .getByTestId('server-state')
      .innerText()
      .catch(() => '');
    if (re.test(s)) return s;
    await page.waitForTimeout(1000);
  }
  return 'timeout';
};
if (step === 'console') {
  await page.getByRole('tab', { name: 'Console' }).click();
  await page.waitForTimeout(1500);
  await page.getByTestId('console-input').fill('list');
  await page.getByTestId('console-input').press('Enter');
  await page.waitForTimeout(2500);
  await shot(page, '76-server-console');
  console.log((await page.getByTestId('console-log').innerText()).slice(-1200));
}
if (step === 'ping') {
  await go(page, '#/servers?tab=multiplayer');
  await page.waitForTimeout(4000);
  await shot(page, '76-server-list-ping');
  console.log((await page.locator('main').innerText()).slice(0, 600));
}
if (step === 'restart') {
  await page.getByTestId('server-restart').click();
  await page.waitForTimeout(1500);
  console.log('during', await page.getByTestId('server-state').innerText());
  console.log('after', await waitState(/running/i));
  await page.waitForTimeout(2000);
  await shot(page, '76-server-restarted');
}
if (step === 'stop') {
  await page.getByTestId('server-stop').click();
  console.log('after', await waitState(/stopped/i));
  await shot(page, '76-server-stopped');
}
if (step === 'backup') {
  await page.getByRole('tab', { name: 'Backups' }).click();
  await page.waitForTimeout(1000);
  await page.getByTestId('backup-now').click();
  await page.waitForTimeout(8000);
  await shot(page, '76-server-backup');
  console.log(
    (
      await page
        .getByTestId('backups-list')
        .innerText()
        .catch((e) => String(e))
    ).slice(0, 600),
  );
}
await browser.close();
