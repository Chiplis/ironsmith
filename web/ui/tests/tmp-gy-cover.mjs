import { createServer } from 'vite';
import { chromium } from 'playwright';
const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'}); await vite.listen();
const browser = await chromium.launch();
const page = await browser.newPage({viewport:{width:1280,height:720}});
await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html`);
const pile = page.locator('[data-local-zone-piles="true"] [data-zone-pile="graveyard"]');
await pile.waitFor();
await page.getByRole('button', {name:'Target graveyard cards'}).click();
for (const wait of [350, 900]) {
  await page.waitForTimeout(wait);
  console.log(wait, JSON.stringify(await pile.evaluate((element) => {
    const slot = element.parentElement; const b = slot.getBoundingClientRect();
    const misses = [];
    for (const y of [0.05, 0.25, 0.5, 0.9]) for (const x of [0.1, 0.5, 0.9]) { const e = document.elementFromPoint(b.left + b.width * x, b.top + b.height * y); if (!slot.contains(e)) misses.push([x, y, e?.className?.toString().slice(0, 60)]); }
    return { b: [Math.round(b.left), Math.round(b.top), Math.round(b.width), Math.round(b.height)], misses, shift: document.querySelector('[data-local-zone-piles]').style.getPropertyValue('--local-zone-rows-shift') };
  })));
}
await page.screenshot({ path: process.argv[2] });
await browser.close(); await vite.close();
