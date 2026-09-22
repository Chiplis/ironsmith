import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('mana discs do not determine the title ink on a gold frame', {timeout:90000}, async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
    const result=await page.evaluate(async()=>{
      const {sampleCardFramePixels,sectionInk}=await import('/src/lib/card-frame-colors.js');
      const {cardTypography}=await import('/src/lib/card-typography.js');
      const {manaTemplates}=await import('/src/lib/card-mana-match.js');
      const read=async url=>{
        const image=new Image();image.src=url;await image.decode();
        const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;
        const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0);return ctx;
      };
      const ctx=await read('/tests/fixtures/frame-mask/bounding-krasis-normal.jpg');
      const art=await read('/tests/fixtures/frame-mask/bounding-krasis-art.jpg');
      const printing={name:'Bounding Krasis',mana_cost:'{1}{G}{U}',frame:'2015',type_line:'Creature — Fish Lizard',power:'3',toughness:'3',oracle_text:'Flash (You may cast this spell any time you could cast an instant.)\nWhen Bounding Krasis enters the battlefield, you may tap or untap target creature.',flavor_text:'Unpredictable as a storm and destructive as a tidal wave.'};
      const typography=cardTypography(printing);
      await Promise.all(['title','type','rules'].map(n=>document.fonts.load(`400 40px ${typography[n]}`)));
      const style=await sampleCardFramePixels({fullScan:ctx.getImageData(0,0,488,680),artScan:art.getImageData(0,0,art.canvas.width,art.canvas.height),printing,typography,icons:await manaTemplates(printing.mana_cost)});
      return {
        // The whole bar is ambiguous: the mana symbols look like white ink.
        wholeBar:sectionInk(ctx.getImageData(37,34,416,36)),
        title:style['--sampled-title-ink'],type:style['--sampled-type-ink'],
        symbols:JSON.parse(style['--printed-mana-symbols']||'null'),
      };
    });
    assert.deepEqual(result.wholeBar,[255,255,255]);
    assert.equal(result.symbols?.symbols.length,3);
    assert.equal(result.title,'rgb(0,0,0)');
    assert.equal(result.type,'rgb(0,0,0)');
  } finally {await browser.close();await vite.close();}
});
