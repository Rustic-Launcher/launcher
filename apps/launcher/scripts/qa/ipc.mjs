import { connect, ipc } from './cdp.mjs';
const { browser, page } = await connect();
const [cmd, json] = process.argv.slice(2);
try {
  console.log(
    JSON.stringify(await ipc(page, cmd, json ? JSON.parse(json) : undefined), null, 1).slice(
      0,
      3000,
    ),
  );
} catch (e) {
  console.log('ERR', e.message.slice(0, 1500));
}
await browser.close();
