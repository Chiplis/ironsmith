import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium} from 'playwright';

const root=fileURLToPath(new URL('../',import.meta.url));
const entry=`import React from 'react';
import {createRoot} from 'react-dom/client';
import {MseRendererProvider} from '/src/context/MseRendererContext.jsx';
import MseCardFrame from '/src/components/cards/MseCardFrame.jsx';
import MiniatureCardFrame from '/src/components/cards/MiniatureCardFrame.jsx';
import {createBrowserArtworkLoader} from '/src/lib/mse/browser-artwork.mjs';
const pending=[];let disposed=0;
const session={dispose(){disposed++;},renderer:{renderTo:async(canvas,request,{signal})=>{
 await new Promise((resolve,reject)=>pending.push({resolve,reject,signal,request}));
 if(signal.aborted)throw new Error('Cancelled');
 canvas.width=10;canvas.height=12;canvas.getContext('2d').fillRect(0,0,10,12);
}}};
const createSession=async()=>session,root=createRoot(document.getElementById('root'));
function render(name,{type_line,style,defaultStyle}={}){root.render(<MseRendererProvider corpusUrl="https://assets.example/mse/" createSession={createSession} defaultStyle={defaultStyle}>
 <MseCardFrame snapshot={JSON.stringify({name,type_line})} style={style} fallback={<span id="fallback">Fallback {name}</span>}/>
</MseRendererProvider>);}
window.test={async artwork(){
 const source=document.createElement('canvas');source.width=2;source.height=2;
 const ctx=source.getContext('2d');ctx.fillStyle='rgb(255,0,0)';ctx.fillRect(0,0,2,2);
 const loader=createBrowserArtworkLoader(),url=source.toDataURL();
 try{const [a,b]=await Promise.all([loader.load(url),loader.load(url)]);return {same:a===b,width:a.width,pixel:Array.from(a.data.slice(0,4))};}finally{loader.dispose();}
},render,renderMiniature(card,faceIndex=0){root.render(<MseRendererProvider corpusUrl="https://assets.example/mse/" createSession={createSession}><MiniatureCardFrame card={card} faceIndex={faceIndex}/></MseRendererProvider>);},resolve:i=>pending[i].resolve(),reject:i=>pending[i].reject(new Error('Failed')),
 count:()=>pending.length,request:i=>pending[i].request,aborted:i=>pending[i].signal.aborted,disposeCount:()=>disposed,unmount:()=>root.unmount()};
render('A');`;
const server=await createServer({root,configFile:false,logLevel:'error',
  resolve:{alias:{'@':fileURLToPath(new URL('../src',import.meta.url))}},
  esbuild:{jsx:'automatic'},optimizeDeps:{include:['react','react-dom/client']},
  plugins:[{name:'mse-react-test',resolveId(id){if(id==='/mse-test-entry.jsx')return id;},load(id){if(id==='/mse-test-entry.jsx')return entry;if(id.endsWith('/components/right-rail/HoverArtOverlay.jsx'))return 'export default function(){return <span id="miniature-fallback">Miniature fallback</span>}';},
    configureServer(server){server.middlewares.use((req,res,next)=>{if(req.url!=='/mse-test')return next();res.setHeader('Content-Type','text/html');res.end('<div id="root"></div><script type="module" src="/mse-test-entry.jsx"></script>');});}}],
  server:{host:'127.0.0.1',port:0},});
let browser;
try{
  await server.listen();browser=await chromium.launch({headless:true});const page=await browser.newPage();page.on('pageerror',error=>console.error(error));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/mse-test`);
  await page.waitForFunction(()=>window.test?.count()===1);
  assert.deepEqual(await page.evaluate(()=>window.test.artwork()),{same:true,width:2,pixel:[255,0,0,255]});
  assert.equal(await page.locator('#fallback').textContent(),'Fallback A');
  await page.evaluate(()=>window.test.resolve(0));await page.locator('[data-mse-frame="ready"]').waitFor();
  assert.equal(await page.locator('#fallback').count(),0);
  await page.evaluate(()=>window.test.render('B'));await page.waitForFunction(()=>window.test.count()===2);
  assert.equal(await page.locator('#fallback').textContent(),'Fallback B');
  assert.equal(await page.locator('canvas').isVisible(),false);
  await page.evaluate(()=>window.test.render('C'));await page.waitForFunction(()=>window.test.count()===3);
  assert.equal(await page.evaluate(()=>window.test.aborted(1)),true);
  await page.evaluate(()=>window.test.resolve(1));assert.equal(await page.locator('#fallback').textContent(),'Fallback C');
  await page.evaluate(()=>window.test.reject(2));await page.waitForFunction(()=>document.querySelector('canvas')?.dataset.mseFrame==='error');
  assert.equal(await page.locator('#fallback').textContent(),'Fallback C');
  await page.evaluate(()=>window.test.render('Saga',{type_line:'Enchantment — Saga'}));
  await page.waitForFunction(()=>window.test.count()===4);
  assert.equal(await page.evaluate(()=>window.test.request(3).style),'magic-m15-saga.mse-style');
  await page.evaluate(()=>window.test.resolve(3));await page.locator('[data-mse-frame="ready"]').waitFor();
  await page.evaluate(()=>window.test.render('Saga',{type_line:'Enchantment — Saga',defaultStyle:'magic-old-saga.mse-style'}));
  await page.waitForFunction(()=>window.test.count()===5);
  assert.equal(await page.evaluate(()=>window.test.request(4).style),'magic-old-saga.mse-style');
  await page.evaluate(()=>window.test.render('Saga',{type_line:'Enchantment — Saga',style:'custom.mse-style'}));
  await page.waitForFunction(()=>window.test.count()===6);
  assert.equal(await page.evaluate(()=>window.test.request(5).style),'custom.mse-style');
  assert.equal(await page.evaluate(()=>window.test.aborted(4)),true);
  await page.evaluate(()=>window.test.renderMiniature({id:42,name:'Printed Trial',type_line:'Creature — Wizard',rarity:'mythic',flavor_text:'Original flavor.'}));
  await page.waitForFunction(()=>window.test.count()===7);
  assert.equal(await page.evaluate(()=>window.test.request(6).snapshot.rarity),'mythic');
  assert.equal(await page.evaluate(()=>window.test.request(6).snapshot.flavor_text),'Original flavor.');
  await page.evaluate(()=>window.test.resolve(6));await page.locator('[data-mse-frame="ready"]').waitFor();
  await page.evaluate(()=>window.test.renderMiniature({id:42,name:'Printed Trial',type_line:'Creature — Wizard',rarity:'rare',flavor_text:'Updated flavor.'}));
  await page.waitForFunction(()=>window.test.count()===8);
  assert.equal(await page.evaluate(()=>window.test.request(7).snapshot.rarity),'rare');
  assert.equal(await page.evaluate(()=>window.test.request(7).snapshot.flavor_text),'Updated flavor.');
  assert.equal(await page.locator('canvas').isVisible(),false);
  const dfc={id:44,layout:'transform',card_faces:[{name:'Front',type_line:'Creature'},{name:'Back',type_line:'Planeswalker'}]};
  await page.evaluate(card=>window.test.renderMiniature(card,0),dfc);
  await page.waitForFunction(()=>window.test.count()===9);
  assert.equal(await page.evaluate(()=>window.test.request(8).adapter.faceIndex),0);
  await page.evaluate(()=>window.test.resolve(8));await page.locator('[data-mse-frame="ready"]').waitFor();
  await page.evaluate(card=>window.test.renderMiniature(card,1),dfc);
  await page.waitForFunction(()=>window.test.count()===10);
  assert.equal(await page.evaluate(()=>window.test.request(9).adapter.faceIndex),1);
  assert.equal(await page.evaluate(()=>window.test.request(9).style),'magic-m15-mainframe-planeswalker.mse-style');
  assert.equal(await page.locator('canvas').isVisible(),false);
  await page.evaluate(()=>window.test.unmount());
  assert.equal(await page.evaluate(()=>window.test.disposeCount()),1);
  console.log(JSON.stringify({status:'passed',checks:['browser artwork decoding and caching','loading fallback','render commit','stale render cancellation','error fallback','automatic style selection and overrides','miniature printing metadata and invalidation','DFC face selection and stale image invalidation','resource disposal']}));
}finally{await browser?.close();await server.close();}
