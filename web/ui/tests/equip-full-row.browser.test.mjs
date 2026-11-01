import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('Equip activates from blank space at the far right of its highlighted row', {timeout:180000}, async () => {
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});
  await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:440,height:570}});
    const errors=[];
    page.on('pageerror',error=>errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html`);
    await page.waitForFunction(()=>window.changeGlintCard);
    await page.evaluate(async()=>{
      const {interactiveRulesView}=await import('/src/lib/inspector-ability-lines.js');
      const text='Equipped creature gets +1/+1.\nEquip {1}';
      window.changeGlintCard({name:'Test Equipment',type_line:'Artifact — Equipment',oracle_text:text,mana_cost:'{1}'});
      window.setGlintRules(interactiveRulesView(text,text,[{
        id:'equip',kind:'activate_ability',label:'Activate Test Equipment: Equip {1}',mana_payment_available:true,
      }]));
    });
    await page.locator('[data-card-glint-frame="ready"]').waitFor({timeout:150000});
    const button=page.getByRole('button',{name:'Test Equipment: Equip {1}',exact:true});
    const rect=await button.boundingBox();
    assert.ok(rect.width>300,'Equip spans the rules row, well beyond its short text');
    await button.hover({position:{x:rect.width-6,y:rect.height/2}});
    assert.equal(await button.getAttribute('data-ability-state'),'available');
    await button.click({position:{x:rect.width-6,y:rect.height/2}});
    assert.equal(await page.evaluate(()=>window.glintActivated),'equip');
    await page.screenshot({path:'test-results/equip-full-row.png'});
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await vite.close();}
});
