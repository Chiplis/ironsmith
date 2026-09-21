import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

// Ice Age printing 2307fb16-8b77-45b5-8a02-51a13214791d, pinned Scryfall scans.
test('mixed ink uses pale name/type fills, dark rules, and removes initial glyphs', {timeout:90000}, async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
    const result=await page.evaluate(async()=>{
      const {sampleCardFramePixels,sectionInk}=await import('/src/lib/card-frame-colors.js');
      const {cardTypography}=await import('/src/lib/card-typography.js');
      const {manaTemplates}=await import('/src/lib/card-mana-match.js');
      const {maskRegisteredRegion}=await import('/src/lib/card-region-mask.js');
      const read=async url=>{
        const image=new Image();image.src=url;await image.decode();
        const c=document.createElement('canvas');c.width=image.width;c.height=image.height;
        const ctx=c.getContext('2d');ctx.drawImage(image,0,0);return ctx;
      };
      const url='/tests/fixtures/frame-mask/arnjlot-normal.jpg';
      const ctx=await read(url),art=await read('/tests/fixtures/frame-mask/arnjlot-art.jpg');
      const printing={"name": "Arnjlot's Ascent", "type_line": "Enchantment", "mana_cost": "{1}{U}{U}", "frame": "1993", "oracle_text": "Cumulative upkeep {U} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)\n{1}: Target creature gains flying until end of turn.", "flavor_text": "\"The dreams of a child fulfilled:\nthe wind on my brow,\nthe air 'neath my feet.\"\n\u2014Arnjlot Olasson, Sky Mage", "artist": "Drew Tucker"};
      const typography=cardTypography(printing);
      await Promise.all(['title','type','rules'].map(n=>document.fonts.load(`400 40px ${typography[n]}`)));
      const style=await sampleCardFramePixels({fullScan:ctx.getImageData(0,0,488,680),artScan:art.getImageData(0,0,art.canvas.width,art.canvas.height),printing,typography,icons:await manaTemplates(printing.mana_cost)});
      const fields=[
        {kind:'name',text:printing.name,bounds:{x:36/488,y:27/680,width:186/488,height:24/680}},
        {kind:'type',text:printing.type_line,bounds:{x:39/488,y:382/680,width:139/488,height:21/680}},
        {kind:'rule',text:'Cumulative Upkeep:',bounds:{x:69/488,y:428/680,width:205/488,height:23/680}},
      ];
      const inks=[];
      for(const f of fields) {
        const patch=await maskRegisteredRegion(url,{...f,lines:[{...f.bounds,text:f.text}]},typography[f.kind==='name'?'title':f.kind==='rule'?'rules':'type']);
        inks.push(patch.ink);
      }
      const clean=style['--source-frame-image']?await read(style['--source-frame-image'].slice(5,-2)):null;
      const residuals=[];
      if(clean)for(const [x,y,w,h] of [[37,28,185,24],[40,383,138,20]]) {
        const before=ctx.getImageData(x,y,w,h).data,after=clean.getImageData(x,y,w,h).data;
        let ink=0,left=0;
        for(let i=0;i<before.length;i+=4)if(Math.min(...before.subarray(i,i+3))>195&&Math.max(...before.subarray(i,i+3))-Math.min(...before.subarray(i,i+3))<45) {
          ink++;if(Math.min(...after.subarray(i,i+3))>195)left++;
        }
        residuals.push({ink,left});
      }
      const controls=[];
      for(const [paper,fill,outline] of [['#b6cbd1','#111',false],['#15394e','#fff',false],['#37a3c0','#fff',true],['#d6be77','#111',false]]) {
        const c=document.createElement('canvas');c.width=280;c.height=40;const g=c.getContext('2d');
        g.fillStyle=paper;g.fillRect(0,0,280,40);g.font='24px Georgia';g.fillStyle=fill;
        if(outline){g.strokeStyle='#111';g.lineWidth=3;g.strokeText('Enchantment',8,29);}
        g.fillText('Enchantment',8,29);controls.push(sectionInk(g.getImageData(0,0,280,40)));
      }
      return {reason:style['--source-frame-fallback-reason'],status:style['--source-frame-status'],colors:['title','type','rules'].map(n=>style[`--sampled-${n}-ink`]),title:JSON.parse(style['--printed-title-text-bounds']||'null'),type:JSON.parse(style['--printed-type-text-bounds']||'null'),inks,residuals,controls};
    });
    assert.equal(result.status,'masked',JSON.stringify(result));
    assert.deepEqual(result.colors,['rgb(255,255,255)','rgb(255,255,255)','rgb(0,0,0)']);
    assert.ok(result.title.x<=40&&result.type.x<=44,JSON.stringify(result));
    assert.deepEqual(result.inks.slice(0,2),['white','white']);
    assert.ok(result.inks[2]?.startsWith('rgb(')&&result.inks[2].match(/\d+/g).every(n=>Number(n)<100),JSON.stringify(result.inks));
    assert.equal(result.residuals.length,2);
    for(const r of result.residuals)assert.ok(r.ink>50&&r.left/r.ink<.05,JSON.stringify(r));
    assert.deepEqual(result.controls,[[0,0,0],[255,255,255],[255,255,255],[0,0,0]]);
  } finally {await browser.close();await vite.close();}
});
