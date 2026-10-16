import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('pregame actions float above Keep hand, navigation is ordered, and compact dragging preserves the card face', async () => {
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen(); const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:1600,height:900}});
    await page.route('https://**/*',r=>r.fulfill({status:404,body:''}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?decisionDock&utilities&pregame`);
    const pregame=page.locator('.action-strip-pregame-button'); await pregame.waitFor();
    await page.waitForTimeout(600);
    const a=await pregame.boundingBox(); const keep=await page.getByRole('button',{name:'Keep hand',exact:true}).boundingBox();
    assert.ok(a.y+a.height<=keep.y && keep.y-a.y-a.height<=12);
    assert.equal(a.width,keep.width);
    assert.equal(await page.locator('.battlefield-human-decision-panel').evaluate(n=>getComputedStyle(n).backgroundColor),'rgba(0, 0, 0, 0)');
    assert.match(await pregame.evaluate(n=>getComputedStyle(n).backgroundImage),/57, 125, 196/);
    assert.deepEqual(await page.locator('.home-navigation button').allTextContents(),['Play','Build']);
    const menu=await page.getByRole('button',{name:'Open navigation menu',exact:true}).boundingBox();
    const brand=await page.locator('.navigation-utility-controls h1').boundingBox();
    const discord=await page.getByRole('link',{name:'Ironsmith on Discord'}).boundingBox();
    const learn=await page.getByRole('button',{name:'Learn',exact:true}).boundingBox();
    assert.ok(menu.x<brand.x && learn.x>discord.x);
    assert.ok(await page.getByText('Controlling',{exact:true}).isVisible());
    const row=page.locator('[data-battlefield-drop-grid="true"]').first();
    const slots=await row.evaluate(n=>{const counts={};for(const c of n.querySelectorAll(':scope > .battlefield-row-card')){const r=c.style.gridRow;counts[r]=(counts[r]||0)+1;}return {columns:Number(n.dataset.battlefieldGridColumns),counts:Object.values(counts)};});
    assert.ok(slots.counts.length && slots.counts.every(n=>n<slots.columns));
    const card=row.locator('.battlefield-row-card').first(); const bounds=await card.boundingBox();
    await page.mouse.move(bounds.x+bounds.width/2,bounds.y+bounds.height/2);await page.mouse.down();
    await page.mouse.move(bounds.x+bounds.width/2+35,bounds.y+bounds.height/2+20,{steps:5});
    await page.locator('[data-compact-drag="true"] .battlefield-artwork-card').waitFor();
    assert.equal(await page.locator('[data-compact-drag="true"] > img').count(),0);
    await page.mouse.up();
    const initialWidth=await row.evaluate(n=>parseFloat(getComputedStyle(n).getPropertyValue('--bf-card-width')));
    await page.setViewportSize({width:1050,height:700});
    await page.waitForTimeout(70);
    const resizing=await row.evaluate(n=>n.getAnimations().some(a=>a.transitionProperty==='--bf-card-width' && a.playState==='running'));
    assert.ok(resizing,'viewport compression animates card width');
    await page.waitForTimeout(600);
    const finalWidth=await row.evaluate(n=>parseFloat(getComputedStyle(n).getPropertyValue('--bf-card-width')));
    assert.ok(finalWidth<initialWidth,'the row shrinks to fit available space');
    await page.emulateMedia({reducedMotion:'reduce'});
    await page.setViewportSize({width:1600,height:900});
    await page.waitForTimeout(70);
    assert.equal(await row.evaluate(n=>n.getAnimations().filter(a=>a.transitionProperty==='--bf-card-width').length),0);

  } finally {await browser.close();await vite.close();}
});
