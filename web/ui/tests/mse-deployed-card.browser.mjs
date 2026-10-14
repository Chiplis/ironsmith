// Real renderer integration against an installed deployment corpus.
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {resolve,sep} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium} from 'playwright';
const root=fileURLToPath(new URL('../',import.meta.url));
const corpus=process.env.MSE_TEST_CORPUS;
if(!corpus)throw new Error('Set MSE_TEST_CORPUS to the installed deployment corpus');
const entry=`import React from 'react';import {createRoot} from 'react-dom/client';
import {I18nProvider} from '/src/i18n/I18nContext';
import {MseRendererProvider} from '/src/context/MseRendererContext';
import CardFramePresentation from '/src/components/cards/CardFramePresentation';
const lines=['{T}: Add {C}.',"{T}: Add one mana of any color. Spend this mana only to cast a legendary spell, and that spell can't be countered."];
const card={name:'Delighted Halfling',mana_cost:'{G}',type_line:'Creature — Halfling Citizen',oracle_text:lines.join('\\n'),colors:['G'],power:'1',toughness:'2'};
const actions=new Map(lines.map((line,index)=>[index,[{kind:'activate_mana_ability',ability_index:index,mana_payment_available:true}]]));
window.activated=[];
const root=createRoot(document.getElementById('root'));
function render(currentCard=card,currentLines=lines,currentActions=actions){root.render(<I18nProvider><MseRendererProvider corpusUrl={location.origin+'/corpus/'}>
<div style={{width:450,height:630}}><CardFramePresentation card={currentCard} rulesView={{lines:currentLines,actions:currentActions}} onActivate={action=>window.activated.push(action)}/></div>
</MseRendererProvider></I18nProvider>);}
window.showHelpers=()=>render({...card,name:'Helper fixture',oracle_text:'Flying\\nAdd {G}.'},['Flying','Add {G}.'],new Map());render();`;
const server=await createServer({root,configFile:false,logLevel:'error',cacheDir:`node_modules/.vite-mse-deployed-${process.pid}`,
 resolve:{alias:{'@':resolve(root,'src')}},esbuild:{jsx:'automatic'},optimizeDeps:{entries:[],include:['react','react-dom/client','react/jsx-runtime','react/jsx-dev-runtime','lucide-react','radix-ui','clsx','tailwind-merge','class-variance-authority']},
 plugins:[{name:'deployed-card',resolveId(id){if(id==='/deployed-card.jsx')return id;},load(id){if(id==='/deployed-card.jsx')return entry;},
 configureServer(server){server.middlewares.use(async(req,res,next)=>{
  if(req.url==='/probe'){res.setHeader('Content-Type','text/html');res.end('<div id="root"></div><script type="module" src="/deployed-card.jsx"></script>');return;}
  if(!req.url.startsWith('/corpus/'))return next();
  const suffix=decodeURIComponent(req.url.slice('/corpus/'.length)),path=resolve(corpus,suffix);
  if(!path.startsWith(resolve(corpus)+sep)){res.statusCode=400;res.end();return;}
  try{const bytes=await readFile(path);res.setHeader('Content-Type',path.endsWith('.json')?'application/json':'application/octet-stream');res.end(bytes);}
  catch{if(process.env.MSE_ASSET_ORIGIN){try{const response=await fetch(new URL(req.url.slice('/corpus/'.length),process.env.MSE_ASSET_ORIGIN));res.statusCode=response.status;res.setHeader('Content-Type',response.headers.get('content-type')||'application/octet-stream');res.end(Buffer.from(await response.arrayBuffer()));}catch(error){res.statusCode=502;res.end(error.message);}}else{res.statusCode=404;res.end();}}
 });}}],server:{host:'127.0.0.1',port:0}});
let browser;
try{
 await server.listen();browser=await chromium.launch({headless:true});const page=await browser.newPage({viewport:{width:480,height:670}});
 const errors=[],assets=[];page.on('requestfailed',r=>console.log('failed',r.url(),r.failure()));page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')console.error(m.text());});
 page.on('request',r=>{if(r.url().includes('/corpus/'))assets.push(r.url());});
 const start=Date.now();await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/probe`);
 try{await page.waitForFunction(()=>[...document.querySelectorAll('[data-mse-frame]')].some(node=>['ready','error','unavailable'].includes(node.dataset.mseFrame)),{},{timeout:120000});}
 catch(error){console.error('Pending assets',assets);console.error(await page.content());console.error(errors);throw error;}
 console.log('Frame state',await page.locator('[data-mse-frame]').evaluateAll(nodes=>nodes.map(node=>node.dataset)));
 if(await page.locator('[data-mse-frame]').getAttribute('data-mse-frame')!=='ready')console.log(await page.locator('[data-mse-frame]').evaluate(node=>{let f=node[Object.keys(node).find(k=>k.startsWith('__reactFiber'))];while(f){let h=f.memoizedState;while(h){if(h.memoizedState?.result?.context)return [...h.memoizedState.result.context.textMeasurements].map(([key,v])=>[key,v.result?.measurement?.runs?.map(r=>({text:r.text,soft:r.soft,characterStart:r.characterStart}))]);h=h.next;}f=f.return;}return null;}));
 assert.equal(await page.locator('[data-mse-frame]').getAttribute('data-mse-frame'),'ready');
 const elapsedMs=Date.now()-start;
 const idle=await page.locator('.mse-card-hit-target').first().evaluate(node=>({shadow:getComputedStyle(node).boxShadow,background:getComputedStyle(node).backgroundColor}));
 assert.equal(idle.shadow,'none');assert.equal(idle.background,'rgba(0, 0, 0, 0)');
 assert.equal(await page.locator('[data-frame-mode="simple"]').count(),0);
 assert.equal(await page.locator('details').count(),0);
 const mana=page.getByRole('button',{name:'Delighted Halfling: {T}: Add {C}.',exact:true});await mana.click();
 assert.equal(await page.evaluate(()=>window.activated[0].ability_index),0);
 const other=page.getByRole('button',{name:/Spend this mana/}).first();await other.hover();
 assert.ok(await page.locator('.mse-card-hit-target[data-hovered="true"]').count()>1);
 await other.focus();await page.keyboard.press('Enter');
 assert.equal(await page.evaluate(()=>window.activated[1].ability_index),1);
 await page.evaluate(()=>window.showHelpers());
 await page.locator('[data-mse-frame="ready"]').waitFor({timeout:30000});
 const flying=page.getByRole('button',{name:'Flying rules helper',exact:true});await flying.click();
 await page.locator('[data-ui-layer="tooltip"]').waitFor();
 assert.match(await page.locator('[data-ui-layer="tooltip"]').innerText(),/flying/i);
 assert.equal(await page.evaluate(()=>window.activated.length),2);
 await page.keyboard.press('Escape');
 const green=page.getByRole('button',{name:'{G}',exact:true});await green.click();
 await page.locator('[data-ui-layer="tooltip"]').waitFor();
 assert.match(await page.locator('[data-ui-layer="tooltip"]').innerText(),/green/i);
 await page.keyboard.press('Escape');
 const stylesheet=await readFile(resolve(root,'src/styles/design-system.css'),'utf8');
 await page.addStyleTag({content:stylesheet.slice(stylesheet.indexOf('/* Keep automatic previews'),stylesheet.indexOf('@media',stylesheet.indexOf('/* Keep automatic previews')))});
 await page.locator('#root').evaluate(node=>{node.classList.add('floating-card-preview');node.dataset.automaticStackPreview='true';});
 await flying.focus();await page.keyboard.press('Enter');
 await page.locator('[data-ui-layer="tooltip"]').waitFor();
 await page.keyboard.press('Escape');
 assert.equal(await flying.evaluate(node=>getComputedStyle(node).pointerEvents),'auto');
 assert.deepEqual(errors,[]);
 const fonts=assets.filter(p=>/\.(ttf|otf)$/i.test(p));assert.ok(fonts.length>0&&fonts.length<20);
 assert.ok(assets.some(p=>p.includes('/packages/')));
 assert.ok(!assets.some(p=>p.endsWith('/style')));
 await page.screenshot({path:'/tmp/ironsmith-mse-interactive.png'});
 console.log(JSON.stringify({status:'passed',elapsedMs,fontRequests:fonts.length,corpusRequests:assets.length,checks:['real MSE canvas','inline ability click','keyboard activation','no details dropdown','lazy fonts','package bundles','keyword helpers','mana symbol helpers','automatic preview helpers','borderless grouped hover']}));
}finally{await browser?.close();await server.close();}
