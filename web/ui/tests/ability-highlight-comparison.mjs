import {createServer} from 'vite';
import {chromium} from 'playwright';
import {readFileSync, mkdirSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
const root=fileURLToPath(new URL('../',import.meta.url));
const output=process.env.HIGHLIGHT_OUTPUT;
const fixtures=process.env.CARD_FRAME_FIXTURES || '/tmp/ironsmith-frame-comparison';
const version=process.argv[2] || 'after';
if(!output) throw new Error('Set HIGHLIGHT_OUTPUT to the screenshot directory.');
mkdirSync(output,{recursive:true});
const vite=await createServer({root,server:{host:'127.0.0.1',port:0},logLevel:'silent'});
await vite.listen();const browser=await chromium.launch();
try {
  for(const slug of ['yawgmoth-thran-physician','myr-moonvessel']) {
    const printing=JSON.parse(readFileSync(`${fixtures}/${slug}.json`));
    const page=await browser.newPage({viewport:{width:520,height:900},deviceScaleFactor:2});
    const errors=[];page.on('pageerror',error=>errors.push(error.message));
    await page.addInitScript(printing=>window.__highlightPrinting=printing,printing);
    await page.route('**/api.scryfall.com/**',r=>r.fulfill({json:printing}));
    await page.route('**/cards/*.json',r=>r.fulfill({json:{scryfall:printing}}));
    await page.route('**/cards.scryfall.io/**',r=>r.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:readFileSync(`${fixtures}/${slug}-${r.request().url().includes('/art_crop/')?'art_crop':'normal'}.jpg`)}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/ability-highlight-comparison.html`);
    await page.waitForSelector('[data-render-ready="true"][data-card-colors="sampled"]');
    await page.evaluate(()=>document.fonts.ready);
    const highlighted=page.locator('.inspector-ability-section[data-stack-highlighted="true"]');
    assert.equal(await highlighted.count(),1);
    const line=await highlighted.textContent();
    assert.ok(slug.startsWith('yawgmoth')?line.includes('Pay 1 life'):line.includes('dies'));
    await page.mouse.move(510,890);
    await page.waitForTimeout(200);
    await page.locator('[data-highlight-case]').screenshot({path:`${output}/${slug}-${version}.png`,animations:'disabled'});
    if (process.env.HIGHLIGHT_BASELINE_CSS) {
      // Capture the previous highlight on this same mounted frame so unrelated
      // frame/layout changes cannot distort the visual comparison.
      await page.addStyleTag({content:readFileSync(process.env.HIGHLIGHT_BASELINE_CSS,'utf8')});
      await page.locator('[data-highlight-case]').screenshot({path:`${output}/${slug}-before.png`,animations:'disabled'});
    }
    console.log(JSON.stringify({version,slug,highlight:line,errors}));
    assert.deepEqual(errors,[]);
    await page.close();
  }
} finally {await browser.close();await vite.close();}
