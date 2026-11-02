import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('planeswalker fallback reserves room for usable loyalty rows', {timeout:90000}, async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});
  await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:440,height:570}});
    await page.route('**/card-glint/**/magic-presentation.js',route=>route.abort());
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
    await page.waitForFunction(()=>window.changeGlintCard);
    for(const creature of [false,true]) {
      await page.evaluate(async creature=>{
        const {interactiveRulesView}=await import("/src/lib/inspector-ability-lines.js");
        const lines=[
          'Whenever an artifact you control enters, put a loyalty counter on Tezzeret.',
          '0: Untap target artifact or creature. If it is an artifact creature, put a +1/+1 counter on it.',
          '−3: Search your library for an artifact card with mana value 1 or less, reveal it, put it into your hand, then shuffle.',
          '−7: You get an emblem with "At the beginning of combat on your turn, put three +1/+1 counters on target artifact you control. If it is not a creature, it becomes a 0/0 Robot artifact creature."',
          'Enters with loyalty counters equal to its printed loyalty number',
        ];
        window.changeGlintCard({name:'Tezzeret, Cruel Captain',type_line:creature?'Artifact Creature':'Legendary Planeswalker — Tezzeret',mana_cost:'{3}',loyalty:'4',oracle_text:lines.join('\n')});
        window.setGlintRules(interactiveRulesView(lines.join('\n'),lines.join('\n'),[1,2,3].map((index)=>({id:['zero','minus','ultimate'][index-1],ability_text:lines[index]}))));
        window.showGlintFrame();
      },creature);
      const frame=page.locator('[data-frame-layout=planeswalker]');
      await frame.waitFor();
      const art=await frame.locator('.simple-card-frame__art').boundingBox();
      const rules=await frame.locator('.simple-card-frame__rules').boundingBox();
      assert.ok(rules.height>art.height*1.5,'rules receive more space than art');
      assert.equal(await frame.locator('.scrollable-loyalty-badge').count(),3);
      assert.ok(!(await frame.innerText()).includes('printed loyalty number'));
      const font=await frame.locator('button').first().evaluate(node=>getComputedStyle(node).fontFamily);
      assert.ok(font.includes('MPlantin'),font);
      for(const [prefix,id] of [['0:','zero'],['−3:','minus'],['−7:','ultimate']]) {
        await frame.getByRole('button',{name:`Tezzeret, Cruel Captain: ${prefix}`,exact:false}).click();
        assert.equal(await page.evaluate(()=>window.glintActivated),id);
      }
      await frame.locator('.simple-card-frame__rules').evaluate(node=>{node.scrollTop=0;});
      assert.equal(await frame.locator('.simple-card-frame__rules').evaluate(node=>node.scrollTop),0);
      if(!creature) await page.screenshot({path:'/tmp/tezzeret-fallback-fixed.png'});
    }
  } finally {await browser.close();await vite.close();}
});
