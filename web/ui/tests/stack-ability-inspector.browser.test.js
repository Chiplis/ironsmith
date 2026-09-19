import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('stack clicks select the precise source ability and share the enabled-hover separator glow', {timeout:60000}, async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1400,height:1000}});
    const errors=[];page.on('pageerror',error=>errors.push(error.message));
    const art='https://cards.scryfall.io/art_crop/front/a/b/aaaaaaaa-bbbb-cccc-dddd-000000000099.jpg';
    const printing={name:'Ability Inspector Fixture',frame:'2015',type_line:'Artifact',image_uris:{art_crop:art,normal:art.replace('/art_crop/','/normal/')}};
    await page.route('**/api.scryfall.com/**',route=>route.fulfill({json:printing}));
    await page.route('**/cards/*.json',route=>route.fulfill({json:{scryfall:printing}}));
    await page.route('**/cards.scryfall.io/**',route=>route.fulfill({contentType:'image/svg+xml',headers:{'access-control-allow-origin':'*'},body:'<svg xmlns="http://www.w3.org/2000/svg" width="488" height="680"><rect width="488" height="680" fill="#dac9a8"/></svg>'}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/stack-ability-inspector.html`);
    const preview=page.locator('[data-card-hover-preview][data-visible="true"]');
    for(const [id,index] of [[103,1],[101,0],[105,2]]) {
      await page.locator(`.stack-card[data-object-id="${id}"]`).click();
      await preview.waitFor();
      await page.waitForFunction(id=>document.querySelector('[data-card-hover-preview][data-visible="true"]')?.dataset.previewObjectId===String(id),id);
      const sections=preview.locator('.inspector-ability-section');
      assert.equal(await sections.count(),3,'inspector retains the whole card');
      assert.equal(await sections.nth(index).getAttribute('data-stack-highlighted'),'true');
      assert.equal(await preview.locator('[data-stack-highlighted="true"]').count(),1);
      assert.equal(await preview.locator('.interactive-card-frame-stage').getAttribute('data-inspected-object-id'),'10','stack ID collision must not inspect another card');
    }
    await page.locator('.stack-card[data-object-id="107"]').click();
    await page.waitForFunction(()=>document.querySelector('[data-card-hover-preview][data-visible="true"]')?.dataset.previewObjectId==='107');
    assert.equal(await preview.locator('[data-stack-highlighted="true"]').count(),0,'unknown source identity must not guess from shared effects');
    // Select the disabled second ability, then hover the enabled first ability.
    await page.locator('.stack-card[data-object-id="103"]').click();await preview.waitFor();
    const fallbackDetails=preview.locator('.original-card-details');
    if(await fallbackDetails.count())await fallbackDetails.locator('summary').click();
    const sections=preview.locator('.inspector-ability-section');
    const glow=section=>section.evaluate(node=>{const s=getComputedStyle(node,'::before');return {opacity:s.opacity,color:s.backgroundColor,shadow:s.boxShadow};});
    // The ability line itself, not a rules helper inside it.
    await sections.nth(0).locator('.inspector-oracle-line-action').hover();
    await page.waitForTimeout(150);
    const hoverGlow=await glow(sections.nth(0));
    assert.equal(hoverGlow.opacity,'1');
    assert.deepEqual(await glow(sections.nth(1)),hoverGlow,'selected unavailable ability uses identical glow');
    await page.getByRole('button',{name:'Toggle availability'}).click();
    await sections.nth(0).hover();await page.waitForTimeout(150);
    assert.equal((await glow(sections.nth(0))).opacity,'0','disabled abilities do not light up on hover');
    assert.equal((await glow(sections.nth(1))).opacity,'1','selection survives loss of availability');
    await page.screenshot({path:'/tmp/stack-ability-inspector.png'});

    // A stack object's frame still opens the rules helpers for game terms,
    // including inside an ability that cannot be activated right now: an
    // unavailable line is marked, never form-disabled, because a disabled
    // control swallows every click inside it.
    await page.locator('.stack-card[data-object-id="101"]').click();
    await preview.waitFor();
    await page.waitForFunction(() => document.querySelectorAll('[data-card-hover-preview][data-visible="true"] .mtg-keyword-helper').length >= 3);
    const helpers = preview.locator('.mtg-keyword-helper');
    assert.equal(await helpers.count(), 3, 'every rules line keeps its helpers');
    for (let index = 0; index < 3; index += 1) {
      const helper = helpers.nth(index);
      const line = await helper.evaluate(node => ({
        text: node.textContent,
        unavailable: node.closest('button')?.getAttribute('aria-disabled') === 'true',
        formDisabled: node.closest('button')?.disabled ?? false,
      }));
      assert.equal(line.formDisabled, false, `helper ${index} sits in a clickable line: ${JSON.stringify(line)}`);
      // A real pointer click, because Playwright's actionability check refuses
      // to click inside an aria-disabled ancestor even though the browser
      // dispatches it -- which is the whole point of the marked-not-disabled
      // ability line.
      const box = await helper.boundingBox();
      await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
      await page.waitForSelector('[data-ui-layer="tooltip"]', {timeout: 5000});
      assert.equal(await page.locator('[data-ui-layer="tooltip"]').count(), 1, `helper ${index} opened its rules tooltip`);
      // Reading a term neither activates the ability nor closes the card.
      assert.equal(await preview.count(), 1);
      await page.keyboard.press('Escape');
      await page.waitForFunction(() => document.querySelectorAll('[data-ui-layer="tooltip"]').length === 0);
    }
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await vite.close();}
});
