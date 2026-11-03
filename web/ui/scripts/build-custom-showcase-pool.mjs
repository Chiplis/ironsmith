// Run from web/ui: node scripts/build-custom-showcase-pool.mjs
import fs from 'node:fs';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {createServer} from 'vite';
import {chromium} from 'playwright';
const hash=value=>createHash('sha256').update(value).digest('hex');
const catalog=JSON.parse(fs.readFileSync('public/custom-sets/index.json'));
const vite=await createServer({server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'error'});await vite.listen();
const browser=await chromium.launch();
const cards=[],skipped=[],names=new Set();
try {
 const page=await browser.newPage();
 await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/forge-board.html`);
 const runtimePath=fileURLToPath(new URL('../../wasm_demo/pkg/ironsmith.js',import.meta.url));
 await page.evaluate(async path=>{const wasm=await import(`/@fs${path}`);await wasm.default();window.poolGame=new wasm.WasmGame();},runtimePath);
 const prepare=async draft=>page.evaluate(async draft=>{
   const {prepareCustomCard}=await import('/src/lib/custom-mechanics.js');
   const {normalizeDraftForApi}=await import('/src/lib/custom-card-draft.js');
   const result=await prepareCustomCard(normalizeDraftForApi(draft),input=>window.poolGame.previewCustomCard(input));
   return result.input;
 },draft);
 for(const entry of catalog.sets){
  const set=JSON.parse(fs.readFileSync(`public/${entry.bundlePath}`));
  const eligible=set.cards.filter(c=>!c.importIssue&&!c.showcaseExcluded&&!c.draft.token&&c.draft.layout==='single');
  eligible.sort((a,b)=>Number(!!b.draft.customMechanics.length)-Number(!!a.draft.customMechanics.length)||hash(a.id).localeCompare(hash(b.id)));
  let accepted=0,attempts=0;
  for(const card of eligible){
   if(accepted===3||attempts++===15)break;
   const name=card.draft.faces[0].name;if(names.has(name))continue;
   try {const input=await prepare(card.draft);cards.push({card,input});names.add(name);accepted++;}
   catch(error){skipped.push({id:card.id,error:error.message.slice(0,300)});}
  }
  console.log(entry.id,accepted,'total',cards.length);
 }
 const fixerDraft=await page.evaluate(async()=> (await import('/src/lib/custom-showcase.js')).MANA_FIXER_DRAFT);
 const fixer={input:await prepare(fixerDraft)};
 if(cards.length<100)throw Error(`Only ${cards.length} cards compiled`);
 fs.writeFileSync('public/custom-sets/showcase.json',JSON.stringify({version:1,cards,fixer}));
 console.log(JSON.stringify({cards:cards.length,mechanicCards:cards.filter(c=>c.card.draft.customMechanics.length).length,skipped:skipped.length,bytes:fs.statSync('public/custom-sets/showcase.json').size}));
}finally{await browser.close();await vite.close();}

// Artifacts must be baked against the exact deployed WASM schema.
await import('./bake-custom-showcase-artifacts.mjs');
