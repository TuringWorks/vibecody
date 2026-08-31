import { chromium } from 'playwright';
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
const errors = [];
page.on('pageerror', e => errors.push(String(e)));
page.on('console', m => { if (m.type() === 'error') errors.push(m.text().slice(0, 200)); });

const [doc, out1, out2] = process.argv.slice(2);
await page.goto(`http://localhost:1420/document-preview.html?doc=${doc}`, { waitUntil: 'networkidle' });
await page.waitForTimeout(2500);
await page.screenshot({ path: out1 });
const toggle = page.getByRole('button', { name: /two up/i });
if (await toggle.count()) {
  await toggle.click();
  await page.waitForTimeout(1500);
  await page.screenshot({ path: out2 });
}
console.log('errors:', errors.slice(0, 6));
await browser.close();
