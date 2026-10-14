import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium} from 'playwright';

const root=fileURLToPath(new URL('../',import.meta.url));
const entry=`import React from 'react';
import {createRoot} from 'react-dom/client';
import {MseRendererProvider} from '/src/context/MseRendererContext.jsx';
import MiniatureCardFrame from '/src/components/cards/MiniatureCardFrame.jsx';
const requests=[],metadata=[];
window.metadata=(url)=>new Promise((resolve,reject)=>metadata.push({url,resolve,reject}));
const session={styles:['magic-m15-altered.mse-style','magic-old.mse-style','magic-m15-showcase-dominaria-stained-glass.mse-style'],dispose(){},renderer:{
 async renderTo(canvas,request,{signal}){
  await new Promise((resolve,reject)=>requests.push({request,signal,resolve,reject}));
  if(signal.aborted)return;
  canvas.width=10;canvas.height=12;canvas.getContext('2d').fillRect(0,0,10,12);
 }
}};
const createSession=async()=>session,root=createRoot(document.getElementById('root'));
window.test={requests,metadata,render(card,imageUrl){root.render(<MseRendererProvider corpusUrl="https://assets.example/mse/" createSession={createSession}>
 <MiniatureCardFrame card={card} imageUrl={imageUrl}/></MseRendererProvider>);}};`;
const server=await createServer({root,configFile:false,logLevel:'error',cacheDir:`node_modules/.vite-mse-printing-${process.pid}`,
  resolve:{alias:{'@':fileURLToPath(new URL('../src',import.meta.url))}},
  esbuild:{jsx:'automatic'},optimizeDeps:{include:['react','react-dom/client']},
  plugins:[{name:'mse-printing-test',resolveId(id){if(id==='/printing-test.jsx')return id;},load(id){
    if(id==='/printing-test.jsx')return entry;
    if(id.endsWith('/components/right-rail/HoverArtOverlay.jsx'))return 'export default function(){return <span id="fallback">Existing renderer</span>}';
    if(id.endsWith('/lib/scryfall.js'))return 'export const resolveScryfallPrintingMetadata=url=>window.metadata(url);export const resolveScryfallSetIcon=async()=>null;export const peekScryfallPrintingMetadata=()=>undefined;export const peekScryfallSetIcon=()=>undefined;';
  },configureServer(server){server.middlewares.use((req,res,next)=>{
    if(req.url!=='/printing-test')return next();
    res.setHeader('Content-Type','text/html');res.end('<div id="root"></div><script type="module" src="/printing-test.jsx"></script>');
  });}}],server:{host:'127.0.0.1',port:0}});
let browser;
try {
  await server.listen();browser=await chromium.launch({headless:true});const page=await browser.newPage();
  const errors=[];page.on('pageerror',error=>{errors.push(error.message);console.error(error.message);});
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/printing-test`);
  await page.waitForFunction(()=>Boolean(window.test));
  const card={id:42,name:'Live name',type_line:'Creature — Wizard',oracle_text:'Live rules',layout:'normal',set:'dmu',frame:'2015'};
  const render=async(value,imageUrl)=>page.evaluate(({value,imageUrl})=>window.test.render(value,imageUrl),{value,imageUrl});
  const count=async n=>page.waitForFunction(n=>window.test.requests.length===n,n);
  const request=async i=>page.evaluate(i=>window.test.requests[i].request,i);
  const flush=async()=>page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));

  await render(card);await count(1);
  await page.evaluate(()=>window.test.requests[0].resolve());await page.locator('[data-mse-frame="ready"]').waitFor();
  await render({...card,frame_effects:['showcase']});await count(2);
  assert.equal((await request(1)).style,'magic-m15-showcase-dominaria-stained-glass.mse-style');
  assert.deepEqual((await request(1)).snapshot.frame_effects,['showcase']);
  assert.equal(await page.locator('canvas').isVisible(),false);
  await page.evaluate(()=>window.test.requests[1].reject(new Error('Missing font')));
  await page.waitForFunction(()=>document.querySelector('canvas').dataset.mseFrame==='error');
  assert.equal(await page.locator('[data-frame-mode="simple"]').isVisible(),true);

  await render({...card,frame:'1997',border_color:'white'});await count(3);
  assert.equal((await request(2)).style,'magic-old.mse-style');
  assert.equal((await request(2)).adapter.nativeCard.border_color,'white');

  await render(card,'https://images.example/a.jpg');await count(4);
  await page.waitForFunction(()=>window.test.metadata.length===1);
  await render(card,'https://images.example/b.jpg');await count(5);
  await page.waitForFunction(()=>window.test.metadata.length===2);
  await page.evaluate(()=>window.test.metadata[0].resolve({frame:'1997',set:'3ed'}));await flush();
  assert.equal(await page.evaluate(()=>window.test.requests.length),5);
  await page.evaluate(()=>window.test.metadata[1].resolve({frame:'2015',set:'dmu',frame_effects:['showcase'],oracle_text:'Printed old rules'}));
  await count(6);
  assert.equal((await request(5)).style,'magic-m15-showcase-dominaria-stained-glass.mse-style');
  assert.equal((await request(5)).snapshot.oracle_text,'Live rules');
  assert.equal(await page.evaluate(()=>window.test.requests[4].signal.aborted),true);
  await page.evaluate(()=>window.test.requests[5].resolve());await page.locator('[data-mse-frame="ready"]').waitFor();

  await render(card,'https://images.example/c.jpg');await count(7);
  await page.waitForFunction(()=>window.test.metadata.length===3);
  assert.equal((await request(6)).style,'magic-m15-altered.mse-style');
  await page.evaluate(()=>window.test.metadata[2].reject(new Error('Unavailable metadata')));await flush();
  assert.equal(await page.evaluate(()=>window.test.requests.length),7);
  await page.evaluate(()=>window.test.requests[6].resolve());await page.locator('[data-mse-frame="ready"]').waitFor();

  await render({...card,set:'khm',frame_effects:['showcase']});await count(8);
  assert.equal((await request(7)).style,'magic-m15-altered.mse-style');
  assert.deepEqual((await request(7)).adapter.styling,{});
  await render({...card,frame_effects:['extendedart']});await count(9);
  assert.deepEqual((await request(8)).adapter.styling,{frames:'frameless art'});
  await render({...card,mse_style:'custom.mse-style',frame_effects:['showcase']},'https://images.example/d.jpg');await count(10);
  assert.equal((await request(9)).style,'custom.mse-style');
  assert.deepEqual((await request(9)).adapter.styling,{});
  // A forced style still looks up the printing, whose artist and number fill the footer.
  await flush();assert.equal(await page.evaluate(()=>window.test.metadata.length),4);
  assert.equal(await page.evaluate(()=>window.test.metadata[3].url),'https://images.example/d.jpg');
  await render({...card,type_line:'Enchantment — Saga'});
  await page.waitForFunction(()=>document.querySelector('canvas').dataset.mseFrame==='unavailable');
  assert.equal(await page.locator('[data-frame-mode="simple"]').isVisible(),true);
  assert.equal(await page.evaluate(()=>window.test.requests.length),10);
  assert.deepEqual(errors,[]);
  console.log(JSON.stringify({status:'passed',checks:['printing metadata propagation','printing change invalidation','render failure fallback',
    'displayed-image metadata','stale lookup cancellation','live text preservation','lookup failure fallback','installed template filtering',
    'extended-art options','explicit override','missing general template fallback']}));
}finally{await browser?.close();await server.close();}
