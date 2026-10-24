import { chromium } from 'playwright';
const browser = await chromium.launch();
const page = await browser.newPage({viewport:{width:1600,height:1000},reducedMotion:'reduce'});
page.on('pageerror',e=>console.error(e.message));
const version=process.argv[2] || 'before';
const scenario=process.argv[3] || 'progenitus';
for (const kind of ['priority','mana_payment']) {
 await page.goto(`http://127.0.0.1:5173/tests/diagnostics-layout.html?kind=${kind}&scenario=${scenario}`);
 await page.waitForTimeout(1800);
 await page.screenshot({path:`../../output/mana-strip/${version}-${kind}.png`});
 console.log(kind, await page.locator('.navigation-utility-controls, .table-phase-divider, .mana-payment-editor').evaluateAll(els=>els.map(el=>({class:el.className,rect:el.getBoundingClientRect().toJSON()}))));
 if(kind==='mana_payment') {
 await page.locator('.desktop-mana-payment .mana-payment-editor').screenshot({path:`../../output/mana-strip/${version}-prompt.png`});
 await page.locator('.mana-payment-pip-row .mana-payment-source-picker').first().click();
 await page.waitForTimeout(400);
 await page.screenshot({path:`../../output/mana-strip/${version}-dropdown.png`});
 }
}
await browser.close();
