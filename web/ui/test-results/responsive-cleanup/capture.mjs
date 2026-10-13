import { chromium } from 'playwright';
const stage = process.argv[2] || 'before';
const browser = await chromium.launch({args:['--enable-unsafe-swiftshader']});
const puzzle = {version:1,players:[{name:'Forge',life:20,zones:{battlefield:['Forest','Island','Llanowar Elves'],hand:['Forest','Grizzly Bears','Counterspell','Lightning Bolt'],library:['Forest','Forest']}},{name:'Opponent',life:20,zones:{battlefield:['Mountain','Mountain','Grizzly Bears'],library:['Mountain']}}]};
const page = await browser.newPage({viewport:{width:1440,height:900}});
page.on('pageerror', e=>console.log(e.message));
await page.goto(`http://127.0.0.1:5173/?puzzle=${Buffer.from(JSON.stringify(puzzle)).toString('base64url')}`);
await page.waitForSelector('.arena-permanent', {timeout:120000});
for (const [name,width,height] of [['desktop',1440,900],['tablet',834,1112],['phone',844,390],['portrait',390,844]]) {
 await page.setViewportSize({width,height});
 await page.waitForSelector('.forge-board canvas',{timeout:90000});
 await page.waitForSelector('.arena-permanent', {timeout:90000});
 await page.waitForTimeout(2000);
 await page.screenshot({timeout:120000,path:new URL(`${name}-${stage}.png`,import.meta.url).pathname});
 console.log(name,await page.locator('body').innerText().then(s=>s.slice(0,600)));

}
await browser.close();
