import { chromium } from 'playwright';
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
const errs = [];
page.on('console', m => { if (m.type() === 'error') errs.push(m.text().slice(0, 120)); });
await page.goto('http://localhost:1420/document-preview.html?doc=pdf', { waitUntil: 'networkidle' });
const snap = async (label) => {
  const info = await page.evaluate(() => {
    const q = (s) => { const e = document.querySelector(s); return e ? `${Math.round(e.clientWidth)}x${Math.round(e.clientHeight)}` : 'missing'; };
    return {
      zoom: document.querySelector('.zoom-label')?.textContent,
      pane: q('.document-viewer-canvas'),
      canvases: [...document.querySelectorAll('.pdf-page canvas')].map(c => `${c.clientWidth}x${c.clientHeight}`).join(' '),
    };
  });
  console.log(label, JSON.stringify(info));
};
for (const t of [500, 1500, 3000]) { await page.waitForTimeout(t === 500 ? 500 : 1000); await snap(`t=${t}`); }
await page.getByRole('button', { name: /two up/i }).click();
await page.waitForTimeout(1500);
await snap('two-up');
console.log('errors:', errs.slice(0, 3));
await browser.close();
