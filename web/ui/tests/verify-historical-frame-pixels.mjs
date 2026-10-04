// Check the art interiors using the same PNG decoder as the browser renderer.
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {chromium} from 'playwright';
import {createServer} from 'vite';
const root=new URL('../test-results/historical-frames/',import.meta.url);
const rows=JSON.parse(await readFile(new URL('baseline/results.json',root),'utf8')).filter(r=>r.mode==='masked');
assert.equal(rows.length,39);
const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
const browser=await chromium.launch();
try{
 const page=await browser.newPage();await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
 const results=await page.evaluate(async slugs=>{
  const read=async url=>{const image=new Image();image.src=url;await image.decode();const c=document.createElement('canvas');c.width=image.width;c.height=image.height;c.getContext('2d').drawImage(image,0,0);return c.getContext('2d').getImageData(80,150,250,150).data;};
  const results=[];
  for(const slug of slugs){
   const [before,after]=await Promise.all(['baseline','reworked'].map(phase=>read(`/test-results/historical-frames/${phase}/${slug}-render.png`)));
   let changed=0;for(let p=0;p<before.length;p+=4)if([0,1,2].some(c=>before[p+c]!==after[p+c]))changed++;
   results.push({slug,changed});
  }
  return results;
 },rows.map(r=>r.slug));
 for(const r of results)assert.equal(r.changed,0,r.slug+' art interior');
 await writeFile(new URL('art-preservation.json',root),JSON.stringify({maskedCards:results.length,changedArtPixels:0,interior:[80,150,330,300],verified:results.map(r=>r.slug)},null,2)+'\n');
 console.log('Verified identical art interiors in all',results.length,'masked renders.');
}finally{await browser.close();await vite.close();}
