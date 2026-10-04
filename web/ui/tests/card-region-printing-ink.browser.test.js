import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('registered legacy replacements keep pale gold lettering separate from dark rules', {timeout:30000},async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
 const browser=await chromium.launch();
 try{
  const page=await browser.newPage();
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
  const result=await page.evaluate(async()=>{
   const {cardTypography}=await import('/src/lib/card-typography.js');
   const {maskRegisteredRegion}=await import('/src/lib/card-region-mask.js');
   const corpus=await fetch('/tests/fixtures/historical-frames/corpus.json').then(r=>r.json());
   const printing=corpus.find(c=>c.slug==='all-106').printing,typography=cardTypography(printing);
   await Promise.all(['title','rules'].map(n=>document.fonts.load(`400 40px ${typography[n]}`)));
   const fields=[
    {kind:'name',text:'Energy Arc',bounds:{x:34/488,y:25/680,width:180/488,height:30/680}},
    {kind:'rule',text:'Untap any number of target creatures.',bounds:{x:68/488,y:419/680,width:330/488,height:26/680}},
   ];
   const patches=[];
   for(const field of fields){
    field.lines=[{...field.bounds,text:field.text}];
    patches.push(await maskRegisteredRegion('/tests/fixtures/historical-frames/all-106-normal.jpg',field,typography[field.kind==='name'?'title':'rules'],typography.profile));
   }
   return patches.map(p=>({ink:p.ink,bounds:p.bounds,image:p.image.slice(0,22)}));
  });
  assert.equal(result[0].ink,'white');assert.equal(result[1].ink,'rgb(0,0,0)');
  for(const patch of result){assert.ok(patch.bounds.width>0);assert.equal(patch.image,'data:image/png;base64,');}
 }finally{await browser.close();await vite.close();}
});
