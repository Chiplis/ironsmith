import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('registered cleanup stays on the scan beneath scrolling replacement rules',{timeout:60000},async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  let browser;
  try {
    await vite.listen();browser=await chromium.launch();
    const page=await browser.newPage({viewport:{width:800,height:900}});
    page.on('pageerror',error=>console.error(error));
    const fixture=await page.evaluate(()=>{
      const canvas=document.createElement('canvas');canvas.width=488;canvas.height=680;
      const ctx=canvas.getContext('2d');ctx.fillStyle='#ddddcc';ctx.fillRect(0,0,488,680);
      ctx.fillStyle='#000';ctx.font='20px Georgia';
      const fields=['First printed rule.','Second printed rule.'].map((text,i)=>{
        const y=420+i*35;ctx.fillText(text,50,y+20);
        const bounds={x:50/488,y:y/680,width:240/488,height:25/680};
        return {kind:'rule',face:0,index:i,text,bounds,lines:[{...bounds,text}]};
      });
      return {registration:{id:'synthetic-mask-scroll',source:canvas.toDataURL(),lang:'en',fields},printing:{name:'Mask test',set:'m21',frame:'2015',lang:'en',layout:'normal',colors:[]},liveRules:['Draw a card. '.repeat(140),'Gain 3 life.']};
    });
    await page.addInitScript(fixture=>{window.__regionFixture=fixture;localStorage.setItem('ironsmith.locale','en');},fixture);
    await page.route('https://api.scryfall.com/**',r=>r.fulfill({json:{}}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-live-regions.html`);
    await page.waitForSelector('.registered-card-frame__scan[data-mask-ready="true"]');
    await page.waitForFunction(()=>document.querySelectorAll('[data-field-kind="rule"][data-replaced="true"]').length===2);
    const column=page.locator('.registered-card-frame__column');
    await page.waitForFunction(()=>{const node=document.querySelector('.registered-card-frame__column');return node&&node.scrollHeight>node.clientHeight+20;});
    const before=await page.locator('.registered-card-frame__scan').boundingBox();
    const src=await page.locator('.registered-card-frame__scan').getAttribute('src');
    await column.evaluate(node=>{node.scrollTop=100;});
    const after=await page.locator('.registered-card-frame__scan').boundingBox();
    assert.deepEqual(after,before,'cleanup stays in the original scan coordinates');
    assert.equal(await page.locator('.registered-card-frame__scan').getAttribute('src'),src);
    assert.ok(await column.evaluate(node=>node.scrollTop)>0);
    assert.equal(await column.locator('img').count(),0,'scrolling text has no source-image patches');
    const remaining=await page.evaluate(async()=>{
      const image=document.querySelector('.registered-card-frame__scan');await image.decode();
      const canvas=document.createElement('canvas');canvas.width=488;canvas.height=680;
      const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0);
      let ink=0;
      for(const y of [420,455]) {
        const data=ctx.getImageData(50,y,240,25).data;
        for(let i=0;i<data.length;i+=4)if(data[i]+data[i+1]+data[i+2]<300)ink++;
      }
      return ink;
    });
    assert.equal(remaining,0,'both printed rules are absent from the stationary cleaned scan');
  } finally {await browser?.close();await vite.close();}
});
