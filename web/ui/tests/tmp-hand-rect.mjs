import { chromium } from 'playwright';
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1600, height: 900 } });
await page.goto('http://localhost:5174/');
await page.waitForSelector('.game-card.hand-card', { timeout: 90000 });
await page.waitForTimeout(2000);
console.log(JSON.stringify(await page.evaluate(() => {
  const r = (e) => { const b = e.getBoundingClientRect(); return [Math.round(b.left), Math.round(b.top), Math.round(b.width), Math.round(b.height)]; };
  const card = document.querySelector('.game-card.hand-card');
  const chain = []; let e = card; for (let i = 0; i < 9 && e; i++, e = e.parentElement) chain.push([e.className.toString().slice(0, 90), r(e)]);
  return { chain, multi: document.querySelector('.table-shell')?.dataset, cards: [...document.querySelectorAll('.game-card.hand-card')].map(r) };
}), null, 1));
await browser.close();
