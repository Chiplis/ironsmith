import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('faint printed lines require a second line at the same size before replacing the height fallback', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-font-fit.html`);
    const result = await page.evaluate(async () => {
      await import('/src/styles/card-typography.css');
      await document.fonts.load('400 100px MPlantin');
      await document.fonts.load('italic 400 100px MPlantin');
      const {detectPanelBounds, measureRulesFirstLine, measureFlavorFirstLine} = await import('/src/lib/card-frame-colors.js');
      const image = new Image();
      image.src = '/tests/fixtures/font-fit/weapons-manufacturing-eoe-168.jpg';
      await image.decode();
      const canvas = document.createElement('canvas');
      canvas.width = image.width; canvas.height = image.height;
      const ctx = canvas.getContext('2d');
      ctx.drawImage(image, 0, 0);
      const box = detectPanelBounds(ctx.getImageData(0, 0, canvas.width, canvas.height), 'rules');
      const text = 'Whenever a nontoken artifact you control enters, create a colorless artifact token named Munitions with "When this token leaves the battlefield, it deals 2 damage to any target."';
      const measure = () => measureRulesFirstLine(ctx, box, text, 'MPlantin', {geometryFallback:true});
      const rules = measure();
      const strict = measureRulesFirstLine(ctx, box, text, 'MPlantin');
      const flavor = measureFlavorFirstLine(ctx, box, '"Soon we\'ll have enough firepower to rid Evendo of its bug infestation."\n—General Tekvu, Kavaron Memorial Navy', 'MPlantin');
      // Preserve the first line but remove all possible corroborating lines.
      ctx.fillStyle = 'white';
      ctx.fillRect(box.x, 453, box.width, box.y + box.height - 453);
      const isolated = measure();
      // The same continuation at a different size is not corroboration.
      ctx.drawImage(image, 43, 455, 342, 14, 43, 455, 280, 14);
      const mismatched = measure();
      return {rules, strict, flavor, isolated, mismatched};
    });
    assert.equal(result.rules?.line, 'Whenever a nontoken artifact you control');
    assert.ok(Math.abs(result.rules.size - 20.61) < .2, JSON.stringify(result));
    assert.ok(result.rules.confidence > .3 && result.rules.confidence < .4,
      'this printing must exercise corroboration rather than a strong single-line match');
    assert.equal(result.strict, null, 'uncorroborated searches retain the stronger confidence threshold');
    assert.ok(Math.abs(result.flavor?.size - 20.77) < .2, JSON.stringify(result));
    for (const measurement of [result.isolated, result.mismatched]) {
      assert.equal(measurement?.confidence, undefined, 'unconfirmed candidates must use the geometry fallback');
      assert.ok(Math.abs(measurement.size - 16.84) < .2, JSON.stringify(measurement));
    }
  } finally { await browser.close(); await vite.close(); }
});

test('rules keep natural size, short rules grow to the cap, and overflow scrolls',async()=>{
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
    const growth=await page.locator('[data-sample="grow"]').evaluate(node=>{
      const text=node.querySelector('.interactive-card-frame__rule-line');
      return {font:parseFloat(getComputedStyle(text).fontSize),reminder:parseFloat(getComputedStyle(node.querySelector('.rules-reminder-text')).fontSize),flavor:parseFloat(getComputedStyle(node.querySelector('.inspector-flavor-text')).fontSize)};
    });
    assert.ok(growth.font>12&&growth.font<=16.2+.01,JSON.stringify(growth));
    assert.equal(growth.reminder,growth.font);
    assert.equal(growth.flavor,growth.font);
    const scrolled=await page.locator('[data-sample="long"] [data-fit-text]').evaluate(box=>{
      box.scrollTop=box.scrollHeight;
      return {top:box.scrollTop,remaining:box.scrollHeight-box.clientHeight-box.scrollTop};
    });
    assert.ok(scrolled.top>0);
    assert.ok(scrolled.remaining<=1);
    assert.equal(metrics.reserved.font,metrics.short.font);
    // A card that gained abilities must stay readable as separate paragraphs.
    // Closing UI spacing is the fitter's first move, but the printing separates
    // its paragraphs too, so the gap has a floor and the type gives way instead.
    const paragraphs=await page.locator('[data-sample="paragraphs"] [data-fit-text]').evaluate(box=>{
      const lines=[...box.querySelectorAll('.interactive-card-frame__rule')];
      const rects=lines.map(line=>line.getBoundingClientRect());
      const gaps=rects.slice(1).map((rect,index)=>rect.top-rects[index].bottom);
      return {gaps,font:parseFloat(getComputedStyle(lines[0]).fontSize),
        spacing:Number(box.style.getPropertyValue('--card-rules-spacing-scale')),
        bottom:rects.at(-1).bottom-box.getBoundingClientRect().bottom};
    });
    assert.equal(paragraphs.spacing,1,`the sample should need the fitter: ${JSON.stringify(paragraphs)}`);
    assert.ok(Math.min(...paragraphs.gaps)>=paragraphs.font*.3,
      `paragraphs must keep a printed-sized gap: ${JSON.stringify(paragraphs)}`);
    assert.equal(paragraphs.font,metrics.short.font,'granted abilities preserve natural size');
    assert.equal(metrics.paragraphs.overflow,'true');
    assert.equal(metrics.paragraphs.scroll,'auto');
    await page.waitForSelector('.registered-card-frame__column');
    const column=await page.locator('.registered-card-frame__column').evaluate(node=>{
      const fonts=[...node.querySelectorAll('.interactive-card-frame__rule-line')].map(line=>parseFloat(getComputedStyle(line).fontSize));
      node.scrollTop=node.scrollHeight;
      const last=node.querySelector('[data-field-kind="flavor"]').getBoundingClientRect();
      return {fonts,scrolled:node.scrollTop,lastBottom:last.bottom,bottom:node.getBoundingClientRect().bottom,top:node.getBoundingClientRect().top};
    });
    assert.equal(column.fonts[0],column.fonts[1],'registered rules and flavor share a natural size');
    assert.ok(column.scrolled>0,'registered overflow remains reachable');
    assert.ok(column.lastBottom<=column.bottom+1&&column.lastBottom>column.top,'can scroll to the final flavor text');
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
    assert.equal(metrics.short.flavor,metrics.short.font);
    assert.equal(metrics.spacing.font,metrics.short.font);
    assert.equal(metrics.spacing.spacing,1);
    assert.equal(metrics.long.font,metrics.short.font);
    assert.equal(metrics.long.overflow,'true');assert.equal(metrics.long.scroll,'auto');
  } finally {await browser.close();await vite.close();}
});
