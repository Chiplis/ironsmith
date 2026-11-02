import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile,mkdir} from 'node:fs/promises';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('Tezzeret uses the painted planeswalker frame at battlefield inspector size', {timeout:90000},async()=>{
  const cards=JSON.parse(await readFile(new URL('../../../cards.json',import.meta.url),'utf8'));
  const source=cards.find(card=>card.name==='Tezzeret, Cruel Captain');
  const card={...source,oracle_text:[
    'Whenever an artifact you control enters, put a loyalty counter on Tezzeret.',
    "0: Untap target artifact or creature. If it's an artifact creature, put a +1/+1 counter on it.",
    '−3: Search your library for an artifact card with mana value 1 or less, reveal it, put it into your hand, then shuffle.',
    '−7: You get an emblem with "At the beginning of combat on your turn, put three +1/+1 counters on target artifact you control. If it\'s not a creature, it becomes a 0/0 Robot artifact creature."',
    'Enters with loyalty counters equal to its printed loyalty number',
  ].join('\n')};
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:430,height:580},deviceScaleFactor:2});
    page.on('console',message=>{if(message.type()==='error')console.error(message.text());});
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
    await page.waitForFunction(()=>window.changeGlintCard);
    await page.evaluate(async card=>{
      const {interactiveRulesView}=await import('/src/lib/inspector-ability-lines.js');
      const lines=card.oracle_text.split('\n');
      window.changeGlintCard(card);
      window.setGlintRules(interactiveRulesView(card.oracle_text,card.oracle_text,[1,2,3].map(i=>({id:String(i),ability_text:lines[i]}))));
      window.setGlintArt(card.image_uris.normal);
      window.showGlintFrame();
    },card);
    await page.locator('[data-card-glint-frame=ready]').waitFor({timeout:60000});
    assert.equal(await page.locator('.simple-card-frame').count(),0);
    assert.ok(!(await page.locator('[data-card-glint-frame=ready]').getAttribute('aria-label')).includes('printed loyalty number'));
    await page.mouse.move(420,570);
    await mkdir('test-results',{recursive:true});
    await page.locator('.card-frame-presentation').screenshot({path:'test-results/tezzeret-painted.png'});
    for(const [prefix,id] of [['0:','1'],['−3:','2'],['−7:','3']]){
      await page.getByRole('button',{name:`Tezzeret, Cruel Captain: ${prefix}`,exact:false}).first().click();
      assert.equal(await page.evaluate(()=>window.glintActivated),id);
    }

    await page.locator('#root>div').evaluate(node=>{node.style.width='300px';node.style.height='418px';});
    await page.locator('.card-glint-scrollable__rules').waitFor({state:'visible'});
    assert.equal(await page.locator('.simple-card-frame').count(),0);
    await page.locator('#root>div').evaluate(node=>{node.style.width='366px';node.style.height='510px';});
    await page.locator('.card-glint-scrollable__rules').waitFor({state:'detached'});
    await page.evaluate(async card=>{
      const {interactiveRulesView}=await import('/src/lib/inspector-ability-lines.js');
      const lines=card.oracle_text.split('\n');
      lines.splice(1,0,...Array.from({length:8},()=> 'Whenever you draw a card, you gain 1 life.'));
      const text=lines.join('\n');
      window.changeGlintCard({...card,oracle_text:text});
      window.setGlintRules(interactiveRulesView(text,text,lines.filter(line=>/^[0−]/.test(line)).map((line,i)=>({id:String(i+1),ability_text:line}))));
    },card);
    const scroll=page.locator('.card-glint-scrollable__rules');
    await scroll.waitFor({state:'visible'});
    assert.equal(await page.locator('.simple-card-frame').count(),0);
    assert.ok(await scroll.evaluate(node=>node.scrollHeight>node.clientHeight));
    assert.ok(await scroll.evaluate(node=>parseFloat(getComputedStyle(node).fontSize)>=12));
    await page.locator('.card-frame-presentation').screenshot({path:'test-results/tezzeret-scrollable.png'});
    await scroll.getByRole('button',{name:'Tezzeret, Cruel Captain: −7:',exact:false}).click();
    assert.equal(await page.evaluate(()=>window.glintActivated),'3');
    assert.ok(await scroll.evaluate(node=>node.scrollTop>0));

  }finally{await browser.close();await vite.close();}
});
