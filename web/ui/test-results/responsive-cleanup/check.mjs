import {chromium} from 'playwright';
import assert from 'node:assert/strict';
const browser=await chromium.launch({args:['--enable-unsafe-swiftshader']});
const page=await browser.newPage();
const errors=[];page.on('pageerror',e=>errors.push(e.message));
const puzzle={version:1,players:[{name:'Forge',life:20,zones:{battlefield:['Forest','Island','Llanowar Elves'],hand:['Forest','Grizzly Bears'],library:['Forest']}},{name:'Opponent',life:20,zones:{battlefield:['Mountain','Grizzly Bears'],library:['Mountain']}}]};
await page.goto('http://127.0.0.1:5173/?puzzle='+Buffer.from(JSON.stringify(puzzle)).toString('base64url'));
await page.waitForSelector('.arena-permanent',{timeout:90000});
for(const [width,height] of [[834,1112],[962,700],[844,390],[667,320],[568,255]]) {
 await page.setViewportSize({width,height});await page.waitForTimeout(900);
 assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`no horizontal overflow ${width}`);
 if(width<721 || height<=540){
  const dock=await page.locator('.mobile-phase-dock').boundingBox();
  const cards=await page.locator('.mobile-mtga-battlefield-band--self .arena-permanent').evaluateAll(els=>els.map(e=>e.getBoundingClientRect().bottom));
  assert.ok(cards.every(y=>y<=dock.y+2),`phase dock clears cards ${width}: ${cards} ${dock.y}`);
  const stop=page.locator('.mobile-phase-dock button').filter({hasText:'Draw'}).first();
  await stop.click();assert.equal(await stop.getAttribute('aria-pressed'),'true');await stop.click();await stop.click();
  await page.getByRole('button',{name:'View hand',exact:false}).click();
  await page.locator('.mobile-mtga-hand-fullscreen').waitFor({state:'visible'});
  await page.screenshot({path:new URL(`hand-${width}-after.png`,import.meta.url).pathname});
  await page.getByRole('button',{name:/Close hand/i}).click();
 }else{
  const header=await page.locator('.battlefield-panel-header--merged').boundingBox();
  assert.ok(header.width>150 && header.y>height*.6,'tablet identity stays visible at bottom');
 }
 console.log('PASS',width,height);
}
assert.deepEqual(errors,[]);await browser.close();
