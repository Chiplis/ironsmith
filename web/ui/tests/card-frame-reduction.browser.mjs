import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {mkdir} from 'node:fs/promises';
import {createServer} from 'vite';
import {chromium} from 'playwright';

const root=fileURLToPath(new URL('../',import.meta.url));
const entry=`import React from 'react';
import {createRoot} from 'react-dom/client';
import {GameContext} from '/src/context/GameContext.shared';
import {I18nProvider} from '/src/i18n/I18nContext';
import {MseRendererProvider} from '/src/context/MseRendererContext';
import MiniatureCardFrame from '/src/components/cards/MiniatureCardFrame';
import HoverArtOverlay from '/src/components/right-rail/HoverArtOverlay';
import CardFramePresentation from '/src/components/cards/CardFramePresentation';
const root=createRoot(document.getElementById('root')),pending=[],activated=[],ready=[];
const action={kind:'activate_ability',object_id:42,ability_index:0,description:'{1}: Draw a card.',mana_payment_available:true};
const base={id:42,name:'Living Archive',type_line:'Artifact Creature — Construct',mana_cost:'{3}',oracle_text:'Vigilance\\n{1}: Draw a card.',colors:[],power:'2',toughness:'4',controller:0};
const art='data:image/svg+xml,'+encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300"><rect width="400" height="300" fill="#284955"/><circle cx="200" cy="150" r="90" fill="#aeb58b"/></svg>');
const session={styles:['magic-m15-altered.mse-style'],dispose(){},renderer:{async renderTo(canvas,request,{signal}){
 await new Promise((resolve,reject)=>pending.push({request,signal,resolve,reject}));
 if(signal.aborted)return;canvas.width=375;canvas.height=523;canvas.getContext('2d').fillRect(0,0,375,523);
 const text=request.snapshot.oracle_text.replace(/[{}]/g,'');
 return {width:375,height:523,context:{fieldVisible:()=>true,property:(_name,key)=>({left:20,top:290,height:160,angle:0}[key]),
 textMeasurements:new Map([['card.rule_text',{result:{measurement:{runs:[{text,characterStart:0}]},lines:[{start:0,end:text.length,top:0,height:18,positions:Array.from({length:text.length+1},(_,i)=>i*3)}]}}]])}};

}}};
const createSession=async()=>session;
function render({enabled=false,imageUrl=art,power='2',rules=base.oracle_text,actions=[action]}={}){
 const card={...base,power,oracle_text:rules};
 const state={players:[{id:0,battlefield:[card]}],perspective:0};
 root.render(<I18nProvider><GameContext.Provider value={{state,game:null}}>
  <MseRendererProvider corpusUrl={enabled?'https://assets.example/mse/':null} createSession={createSession}>
   <div style={{display:'flex',gap:20,padding:20}}>
    <div id="miniature" style={{position:'relative',width:190,height:266}}><MiniatureCardFrame card={card} imageUrl={imageUrl}/></div>
    <div id="inspector" style={{position:'relative',width:380,height:531}}><HoverArtOverlay objectId={42} displayMode="card-frame" sourceImageUrl={imageUrl}
      interactiveActions={actions} onInteractiveAction={value=>activated.push(value)} onCardFrameReadyChange={value=>ready.push(value)}/></div>
    <div id="multipart" style={{width:300,height:420}}><CardFramePresentation card={{name:'Fire // Ice',layout:'split',card_faces:[
      {name:'Fire',mana_cost:'{1}{R}',type_line:'Instant',oracle_text:'Deal 2 damage divided as you choose among one or two targets.'},
      {name:'Ice',mana_cost:'{1}{U}',type_line:'Instant',oracle_text:'Tap target permanent. Draw a card.'}]}}/></div>
   </div>
  </MseRendererProvider>
 </GameContext.Provider></I18nProvider>);
}
window.test={render,pending,activated,ready};render();`;
const server=await createServer({root,configFile:false,logLevel:'error',cacheDir:`node_modules/.vite-frame-reduction-${process.pid}`,
  resolve:{alias:{'@':fileURLToPath(new URL('../src',import.meta.url))}},esbuild:{jsx:'automatic'},
  optimizeDeps:{entries:[],include:['react','react-dom/client','react/jsx-runtime','react/jsx-dev-runtime','lucide-react','radix-ui','clsx','tailwind-merge','class-variance-authority']},
  plugins:[{name:'frame-reduction',resolveId(id){if(id==='/reduction.jsx')return id;},load(id){
    if(id==='/reduction.jsx')return entry;
    if(id.endsWith('/context/GameContext.jsx'))return `import {useContext} from 'react';import {GameContext} from './GameContext.shared';export const useGame=()=>useContext(GameContext);`;
    if(id.endsWith('/hooks/useScryfallImageUrl.js'))return `export const useScryfallImage=()=>({url:null,ready:true});`;
    if(id.endsWith('/hooks/useScryfallFlavorText.js'))return `export default function(){return '';}`;
  },configureServer(server){server.middlewares.use((req,res,next)=>{
    if(req.url!=='/reduction')return next();res.setHeader('Content-Type','text/html');
    res.end('<style>body{margin:0;background:#243040}.absolute{position:absolute}.inset-0{inset:0}</style><div id="root"></div><script type="module" src="/reduction.jsx"></script>');
  });}}],server:{host:'127.0.0.1',port:0,watch:null}});
let browser;
try{
  await server.listen();browser=await chromium.launch({headless:true});const page=await browser.newPage({viewport:{width:980,height:620}});
  const errors=[],requests=[];
  page.on('pageerror',error=>{errors.push(error.message);console.error(error.message);});
  page.on('request',request=>requests.push(request.url()));
  await page.route('**/missing-art.png',route=>route.abort());
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/reduction`);
  await page.locator('#inspector [data-frame-mode="simple"]').waitFor();
  await page.waitForFunction(()=>window.test.ready.includes(true));
  assert.equal(await page.locator('[data-frame-mode="simple"]').count(),3);
  assert.ok((await page.locator('#inspector .simple-card-frame').innerText()).includes('2/4'));
  assert.ok((await page.locator('#multipart').innerText()).includes('Ice'));
  assert.ok((await page.locator('#multipart').innerText()).includes('Tap target permanent'));
  const titleSize=await page.locator('#miniature .simple-card-frame__heading').first().evaluate(node=>getComputedStyle(node).fontSize);
  const firstWidth=await page.locator('#miniature .simple-card-frame').evaluate(node=>node.getBoundingClientRect().width);
  await page.locator('#miniature').evaluate(node=>{node.style.width='96px';node.style.height='134px';});
  await page.waitForFunction(()=>document.querySelector('#miniature .simple-card-frame').getBoundingClientRect().width<100);
  assert.equal(await page.locator('#miniature .simple-card-frame__heading').first().evaluate(node=>getComputedStyle(node).fontSize),titleSize);
  assert.ok(await page.locator('#miniature .simple-card-frame').evaluate(node=>node.getBoundingClientRect().width)<firstWidth);
  const activate=page.getByRole('button',{name:'Living Archive: {1}: Draw a card.',exact:true});
  await activate.click();assert.equal(await page.evaluate(()=>window.test.activated[0].ability_index),0);
  // The independent controls remain mounted when the canvas replaces fallback.
  await page.evaluate(()=>window.test.render({enabled:true}));
  await page.waitForFunction(()=>window.test.pending.length>=2);
  await page.evaluate(()=>window.test.pending.forEach(value=>value.resolve()));
  await page.locator('#inspector [data-mse-frame="ready"]').waitFor();
  assert.equal(await page.locator('#inspector [data-frame-mode="simple"]').count(),0);
  await activate.click();assert.equal(await page.evaluate(()=>window.test.activated.length),2);
  // A live update keeps the card's previous canvas while it re-renders.
  await page.evaluate(()=>window.test.render({enabled:true,power:'5'}));
  await page.locator('#inspector [data-mse-frame="pending"]').waitFor();
  await page.waitForTimeout(400);
  assert.equal(await page.locator('#inspector [data-frame-mode="simple"]').count(),0);
  assert.equal(await page.locator('#inspector canvas').isVisible(),true);
  // A failed render falls back to the simple frame with the live values.
  await page.evaluate(()=>window.test.pending.forEach(value=>value.reject(new Error('Template failed'))));
  await page.locator('#inspector [data-mse-frame="error"]').waitFor({state:'attached'});
  await page.locator('#inspector [data-frame-mode="simple"]').waitFor();
  assert.ok((await page.locator('#inspector .simple-card-frame').innerText()).includes('5/4'));
  await activate.click();assert.equal(await page.evaluate(()=>window.test.activated.length),3);
  await page.evaluate(()=>window.test.render({imageUrl:'/missing-art.png',power:'7'}));
  await page.waitForFunction(()=>document.querySelectorAll('.simple-card-frame__art img').length===0);
  assert.ok((await page.locator('#inspector .simple-card-frame').innerText()).includes('7/4'));
  await page.evaluate(()=>window.test.render());
  await page.locator('#inspector .simple-card-frame__art img').waitFor();
  assert.equal(await page.locator('#inspector details').count(),0);
  await mkdir('/tmp/ironsmith-frame-reduction',{recursive:true});
  await page.screenshot({path:'/tmp/ironsmith-frame-reduction/fallbacks.png'});
  const manaActions=['W','U'].map((color,index)=>({kind:'activate_mana_ability',object_id:42,ability_index:index,
    ability_text:`{T}: Add {${color}}.`,mana_payment_available:true}));
  await page.evaluate(actions=>window.test.render({rules:'{T}: Add {W}.\n{T}: Add {U}.',actions}),manaActions);
  await page.waitForFunction(()=>document.querySelectorAll('#inspector .inspector-mana-choice').length===2);
  assert.equal(await page.locator('#inspector details').count(),0);
  await page.locator('#inspector .inspector-mana-choice').nth(1).waitFor({state:'visible'});
  await page.locator('#inspector .inspector-mana-choice').nth(1).focus();
  await page.keyboard.press('Enter');
  assert.equal(await page.evaluate(()=>window.test.activated.at(-1).ability_index),1);
  assert.deepEqual(errors,[]);
  assert.deepEqual(requests.filter(url=>/card-frame-preparation|card-region-catalog|default-card-frames|RegisteredCardFrame|OriginalCardFallback|sampleCardFrame|card-frames\/default/.test(url)),[]);
  console.log(JSON.stringify({status:'passed',checks:['shared fallback','inspector readiness','live stats','multipart text','miniature scaling',
    'ability activation across renderer changes','MSE failure','missing artwork','keyboard mana choices','no masking or stock assets']}));
}finally{await browser?.close();await server.close();}
