import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import { mkdir } from 'node:fs/promises';

test('hover Submit covers grouped searches, required choices, modes, and legality', {timeout:120000}, async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen(); const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1200,height:900},deviceScaleFactor:2});
    const errors=[]; page.on('pageerror',e=>errors.push(e.message));
    await page.route('**/api.scryfall.com/**',r=>r.abort());
    const open=async query=>{
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/choice-submit.html?${query}`);
      await page.locator('.decision-option-row').first().waitFor();
    };
    const commands=()=>page.evaluate(()=>window.__commands);
    for(const min of [0,1]) {
      await open(`max=1&min=${min}`);
      const forest=page.locator('[data-decision-option-object="10"]');
      await forest.hover();
      assert.deepEqual(await commands(),[]);
      const submit=page.getByRole('button',{name:'Select Forest and submit',exact:true});
      if(min===0) {
        await mkdir('../../output/optional-target-submit',{recursive:true});
        await page.locator('.battlefield-human-decision-panel').screenshot({path:'../../output/optional-target-submit/library-search-hover.png'});
      }
      const bounds=async()=>submit.evaluate(button=>{
        const row=button.closest('.decision-target-option');
        const panel=button.closest('.battlefield-human-decision-panel');
        const rect=element=>{const {x,y,width,height}=element.getBoundingClientRect();return {x,y,width,height};};
        return {button:rect(button),row:rect(row),panel:rect(panel),scrollHeight:panel.scrollHeight};
      });
      const before=await bounds();
      await submit.hover();
      await page.waitForTimeout(200);
      assert.deepEqual(await bounds(),before,'hovering Submit must not move it or enlarge the decision panel');
      assert.ok(before.button.y>=before.row.y && before.button.y+before.button.height<=before.row.y+before.row.height,
        'Submit stays inside the choice row');
      await submit.focus();
      assert.deepEqual(await bounds(),before,'focusing Submit must not change layout');
      await submit.click();
      assert.deepEqual(await commands(),[{type:'select_objects',object_ids:[10]}]);
    }
    await open('max=2');
    assert.equal(await page.locator('.decision-target-quick-submit').count(),0);
    await page.locator('[data-decision-option-object="10"]').click();
    await page.getByRole('button',{name:'Select Forest and submit',exact:true}).click();
    assert.deepEqual(await commands(),[{type:'select_objects',object_ids:[10,20]}]);
    await open('restricted=1');
    assert.equal(await page.getByRole('button',{name:'Select Forest and submit',exact:true}).count(),0);
    await page.getByRole('button',{name:'Select Gingerbread Cabin and submit',exact:true}).focus();
    await page.keyboard.press('Enter');
    assert.deepEqual(await commands(),[{type:'select_objects',object_ids:[40]}]);
    for(const min of [0,1]) {
      await open(`kind=options&max=1&min=${min}`);
      await page.locator('.decision-option-row').filter({hasText:'Draw a card'}).hover();
      await page.getByRole('button',{name:'Select Draw a card and submit',exact:true}).click();
      assert.deepEqual(await commands(),[{type:'select_options',option_indices:[0]}]);
    }
    await open('kind=options&max=2');
    assert.equal(await page.locator('.decision-target-quick-submit').count(),0);
    await page.locator('.decision-option-row').filter({hasText:'Draw a card'}).click();
    assert.equal(await page.getByRole('button',{name:'Select Unavailable mode and submit',exact:true}).count(),0);
    await page.locator('.decision-option-row').filter({hasText:'Gain 3 life'}).hover();
    await page.getByRole('button',{name:'Select Gain 3 life and submit',exact:true}).click();
    assert.deepEqual(await commands(),[{type:'select_options',option_indices:[0,1]}]);
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await vite.close();}
});
