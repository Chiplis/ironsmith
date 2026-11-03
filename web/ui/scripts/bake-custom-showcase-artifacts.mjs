import fs from 'node:fs';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium} from 'playwright';
const file='public/custom-sets/showcase.json',pool=JSON.parse(fs.readFileSync(file));
const vite=await createServer({server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'error'});await vite.listen();const browser=await chromium.launch();
try {
 const page=await browser.newPage();await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/forge-board.html`);
 const result=await page.evaluate(async({pool,path,supportOnly})=>{
  const wasm=await import('/@fs'+path);await wasm.default();
  const {showcaseArtifactInput,showcaseArtifactSource}=await import('/scripts/custom-showcase-artifacts.mjs');
  const game=new wasm.WasmGame(),failures=[];let baked=0;
  for(const value of [...pool.cards,pool.fixer]){
   if(supportOnly){game.registerCompiledCardSourceArtifacts(value.source,value.artifacts);baked++;continue;}
   delete value.artifacts;delete value.source;
   const input=showcaseArtifactInput(value.input);if(!input){failures.push({name:value.input.faces[0].name,error:'Standalone artifact transport does not support attached token definitions yet'});continue;}
   try{const artifact=JSON.parse(JSON.stringify(wasm.compileCardArtifact(input),(_key,value)=>value===undefined?null:value instanceof Map?Object.fromEntries(value):value)),source=showcaseArtifactSource(value.input,input);
    game.registerCompiledCardSourceArtifacts(source,[artifact]);value.artifacts=[artifact];value.source=source;baked++;
   }catch(error){failures.push({name:input.name,error:String(error)});}
  }
  const catalog=JSON.parse(game.getEmbeddedCardCatalogIndexJson());
  const {cardRouteKey}=await import('/src/lib/scryfall.js');
  pool.support=[];
  for(const name of ['Omniscience',...(catalog.dungeons||[]).map(d=>d.name)]){
   const source=JSON.parse(game.getEmbeddedCardSourceJson(cardRouteKey(name)));
   let artifacts=source.artifacts;
   try {if(!artifacts?.length)throw Error('No artifacts');game.registerCompiledCardSourceArtifacts(source,artifacts);}
   catch {artifacts=[JSON.parse(JSON.stringify(wasm.compileCardArtifact({name:source.group.name,text:source.group.block,localId:1}),(_key,value)=>value===undefined?null:value instanceof Map?Object.fromEntries(value):value))];game.registerCompiledCardSourceArtifacts(source,artifacts);}
   delete source.artifacts;pool.support.push({source,artifacts});
  }
  return {pool,baked,failures};
 },{pool,supportOnly:process.argv.includes('--support-only'),path:fileURLToPath(new URL('../../wasm_demo/pkg/ironsmith.js',import.meta.url))});
 if(result.baked<100)throw Error(JSON.stringify(result.failures.slice(0,3)));
 if(!result.pool.fixer.artifacts?.length)throw Error('Mana Fixer must have a validated artifact');
 // Unsupported artifact payloads stay in the editable source sets, never in a
 // startup pool that promises zero compilation. Keep a reviewable build report.
 result.pool.cards=result.pool.cards.filter(value=>value.artifacts?.length);
 const prior=fs.existsSync('public/custom-sets/showcase-build-report.json')?JSON.parse(fs.readFileSync('public/custom-sets/showcase-build-report.json','utf8')):{};
 const included=new Set(result.pool.cards.map(value=>value.input.faces[0].name));
 const excluded=[...new Map([...(prior.excluded||[]),...result.failures].filter(value=>!included.has(value.name)).map(value=>[value.name,value])).values()];
 fs.writeFileSync('public/custom-sets/showcase-build-report.json',JSON.stringify({baked:result.baked,excluded},null,2));
 fs.writeFileSync(file,JSON.stringify(result.pool));console.log(JSON.stringify({baked:result.baked,excluded:excluded.length}));
}finally{await browser.close();await vite.close();}
