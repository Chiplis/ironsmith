import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';
test('short text keeps its size, spacing shrinks first, and long text has a readable floor',async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();const browser=await chromium.launch();
  try {
    const page=await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-font-fit.html`);
    await page.waitForFunction(()=>document.querySelector('[data-sample="long"] [data-text-overflow="true"]'));
    const metrics=await page.locator('[data-sample]').evaluateAll(nodes=>Object.fromEntries(nodes.map(n=>{
      const box=n.querySelector('[data-fit-text]');const flavor=n.querySelector('.inspector-flavor-text');
      return [n.dataset.sample,{font:parseFloat(getComputedStyle(n.querySelector('.interactive-card-frame__rule-line')).fontSize),flavor:flavor&&parseFloat(getComputedStyle(flavor).fontSize),spacing:Number(box.style.getPropertyValue('--card-rules-spacing-scale')),overflow:box.dataset.textOverflow,scroll:getComputedStyle(box).overflowY}];
    })));
    const reserved=await page.locator('[data-sample="reserved"] [data-fit-text]').evaluate(box=>{
      const range=document.createRange();range.selectNodeContents(box.querySelector('.interactive-card-frame__rule-line'));
      return {text:range.getBoundingClientRect().bottom,bottom:box.getBoundingClientRect().bottom-paddingBottom(box)};
      function paddingBottom(e){return parseFloat(getComputedStyle(e).paddingBottom)||0;}
    });
    assert.ok(reserved.text<=reserved.bottom+1,JSON.stringify(reserved));
    const measured=await page.evaluate(async()=>{
      const {measureRulesFirstLine,measureFlavorFirstLine,measureReminderText}=await import('/src/lib/card-frame-colors.js');
      const canvas=document.createElement('canvas');canvas.width=400;canvas.height=180;
      const ctx=canvas.getContext('2d');ctx.fillStyle='white';ctx.fillRect(0,0,400,180);ctx.fillStyle='black';
      ctx.font='18px Georgia';ctx.fillText('Draw three cards.',12,32);
      ctx.font='italic 24px Georgia';ctx.fillText('As patient as nature.',12,90);
      const box={x:0,y:0,width:400,height:180};
      const rules=measureRulesFirstLine(ctx,box,'Draw three cards.','Georgia')?.size;
      const flavor=measureFlavorFirstLine(ctx,box,'As patient as nature.','Georgia')?.size;
      ctx.fillStyle='white';ctx.fillRect(0,0,400,180);ctx.fillStyle='black';
      ctx.font='18px Georgia';ctx.fillText('Protection from Humans.',12,30);
      ctx.fillText('Draw three cards.',12,62);
      ctx.fillText('Draw three cards.',12,84);
      ctx.fillText('Draw three cards.',12,106);
      const paragraphLeading=measureRulesFirstLine(ctx,box,'Protection from Humans.','Georgia')?.lineHeight;
      ctx.fillStyle='white';ctx.fillRect(0,0,400,180);ctx.fillStyle='black';
      ctx.font='italic 24px Georgia';ctx.fillText('“As patient as nature.',12,70);
      const quotedFlavor=measureFlavorFirstLine(ctx,box,'"As patient as nature.','Georgia');
      ctx.fillStyle='white';ctx.fillRect(0,0,400,180);ctx.fillStyle='black';
      ctx.font='20px Georgia';ctx.fillText('Proliferate.',12,30);
      ctx.font='italic 16px Georgia';ctx.fillText('(Choose',130,30);
      ctx.fillText('any number of permanents and players,',12,52);
      ctx.fillText('then give each another counter.)',12,74);
      const reminder=measureReminderText(ctx,box,'Proliferate. (Choose any number of permanents and players, then give each another counter.)','Georgia');
      ctx.fillStyle='#121820';ctx.fillRect(0,0,400,180);
      // A coloured art streak joins every row under the white printed ink.
      ctx.fillStyle='#6090b0';ctx.fillRect(150,8,24,155);
      ctx.fillStyle='white';ctx.font='18px Georgia';
      ctx.fillText('Whenever you discard a card,',12,32);
      ctx.fillText('you may pay to draw a card.',12,54);
      ctx.font='italic 24px Georgia';ctx.fillText('As patient as nature.',12,110);
      const translucent=measureRulesFirstLine(ctx,box,'Whenever you discard a card, you may pay {2}.','Georgia');
      return {rules,flavor,translucent,paragraphLeading,quotedFlavor,reminder};
    });
    assert.ok(Math.abs(measured.rules-18)<2,JSON.stringify(measured));
    assert.ok(Math.abs(measured.translucent?.size-18)<2,JSON.stringify(measured));
    assert.equal(measured.translucent.line,'Whenever you discard a card,');
    assert.ok(measured.translucent.y<32,'measures rules rather than later flavor');
    assert.ok(Math.abs(measured.flavor-24)<2,JSON.stringify(measured));
    assert.ok(Math.abs(measured.paragraphLeading-22)<=2, 'a paragraph gap must not become wrapped-line leading');
    assert.ok(Math.abs(measured.quotedFlavor?.size-24)<2, 'straight metadata quotes match curly printed quotes');
    assert.ok(Math.abs(measured.reminder?.size-16)<1, 'measure reminder size independently of its roman prefix');
    assert.equal(measured.reminder.line, 'any number of permanents and players,');
    assert.equal(metrics.registered.font,22,'registered fields keep the printed size without the preview clamp');
    assert.equal(metrics.registered.overflow,'false','flush text is not overflow');
    assert.equal(metrics.short.font,16.5);
    assert.equal(metrics.short.flavor,18);
    assert.equal(metrics.spacing.font,metrics.short.font);
    assert.ok(metrics.spacing.spacing<1);
    assert.equal(metrics.long.font,metrics.short.font*.75);
    assert.equal(metrics.long.overflow,'true');assert.equal(metrics.long.scroll,'auto');
  } finally {await browser.close();await vite.close();}
});
