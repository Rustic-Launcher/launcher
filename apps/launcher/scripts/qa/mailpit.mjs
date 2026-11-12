import { chromium } from '@playwright/test';
import path from 'node:path';
import { SHOTS } from './cdp.mjs';
const b = await chromium.launch();
const p = await b.newPage({ viewport: { width: 1280, height: 800 } });
await p.goto('http://localhost:8025/');
await p
  .getByText(process.env.SUBJECT ?? 'Verify your Rustic account')
  .first()
  .click();
await p.waitForTimeout(1500);
await p.screenshot({ path: path.join(SHOTS, process.env.OUT ?? '71-mailpit-verification.png') });
await b.close();
