import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { loadCustomShowcase, isCustomShowcase } from '../src/lib/custom-showcase.js';
import { mergeBundledLibrary } from '../src/lib/bundled-custom-sets.js';
import { normalizeDraftForApi } from '../src/lib/custom-card-draft.js';
import { setMechanics, updateSetMechanic } from '../src/lib/set-workspace.js';
import { splitTextWithMtgKeywordRules } from '../src/lib/mtg-keywords.js';
const root=new URL('../public/',import.meta.url);
test('bundled source count, printing attribution, images and valid mechanic variants',()=>{
 const catalog=JSON.parse(fs.readFileSync(new URL('custom-sets/index.json',root)));let count=0;
 for(const entry of catalog.sets) {
  const set=JSON.parse(fs.readFileSync(new URL(entry.bundlePath,root)));assert.equal(set.cards.length,entry.cardCount);
  for(const card of set.cards)for(const face of card.draft.faces) {
   assert.ok(face.imageUrl.startsWith('custom-sets/images/'),face.imageUrl);
   if(face.imageUrl.startsWith('custom-sets/'))assert.ok(fs.existsSync(new URL(face.imageUrl,root)));assert.ok(face.attribution.setCode);assert.ok(face.attribution.collectorNumber);
   assert.equal(typeof face.attribution.artist,'string');assert.notEqual(face.originalPrintedRulesText,undefined);
  }
  const definitions=setMechanics({customCards:set.cards,customMechanics:set.customMechanics});
  for(const def of definitions){assert.equal(def.attribution.setCode,set.code);assert.ok(def.attribution.collectorNumber);}
  count+=set.cards.length;
 }
 assert.equal(count,23704);assert.equal(catalog.missingImages.length,0);
});
test('saved edits shadow bundled defaults without mutating them',()=>{
 const shipped={name:'MSEM · Test',cards:[]},saved={...shipped,cards:[{id:'edit'}]};
 assert.deepEqual(mergeBundledLibrary({formats:[],sets:[saved]},[shipped]).sets,[saved]);assert.deepEqual(shipped.cards,[]);
});
test('printing metadata survives normalization',()=>{
 const attribution={artist:'Example Artist',setCode:'EX',collectorNumber:'17'};
 const result=normalizeDraftForApi({attribution,faces:[{name:'Example',oracleText:'Draw a card.',attribution,imageUrl:'https://example.com/17.jpg'}]});
 assert.deepEqual(result.attribution,attribution);assert.deepEqual(result.faces[0].attribution,attribution);assert.ok(result.faces[0].imageUrl);
});
test('tooltips use the supplied rewrite without cross-card leakage',()=>{
 for(const reminder of ['Draw a card.','Draw two cards.','Draw a card.']) {
  const parts=splitTextWithMtgKeywordRules('Kindle','en',[{keyword:'kindle',reminder}]);assert.equal(parts[0].rule.summary,reminder);
 }
});
test('editing one mechanic variant preserves another',async()=>{
 const a={keyword:'kindle',reminder:'Draw a card.'},b={keyword:'kindle',reminder:'Draw two cards.'};
 const card=(name,def)=>({draft:{faces:[{name,cardTypes:['Sorcery'],oracleText:'Kindle'}],customMechanics:[def]}});
 const next=await updateSetMechanic({customCards:[card('A',a),card('B',b)]},{...a,reminder:'Draw three cards.'},async()=>({canCreate:true,faces:[]}),a);
 assert.equal(next.customCards[0].draft.customMechanics[0].reminder,'Draw three cards.');assert.equal(next.customCards[1].draft.customMechanics[0].reminder,b.reminder);
});
function mockGame(fail=false) {
 const calls=[];return {calls,createRuntimeSavepoint:async()=>7,
  resetEmpty:async(...v)=>calls.push(['reset',...v]),
  registerCompiledCardSourceArtifacts:async source=>{if(fail)throw Error('invalid artifact');calls.push(['register',source]);},
  stagePuzzleCardsToZones:async batch=>calls.push(['stage',batch]),
  createCustomCard:()=>{throw Error('Runtime compilation is forbidden');},
  finishPuzzleSetup:async()=>calls.push(['finish']),
  restoreRuntimeSavepoint:async id=>calls.push(['restore',id]),releaseRuntimeSavepoint:async id=>calls.push(['release',id])};
}
const entry=input=>({input,source:{group:{name:input.faces[0].name}},artifacts:[{test:true}]});
const sets=['msem','rev','msem','rev'].map((scope,i)=>({id:String(i),source:{scope},cards:Array.from({length:8},(_,n)=>({id:`${i}-${n}`,draft:{faces:[{name:`Card ${i}-${n}`}],customMechanics:[]}}))}));
const options={librarySize:1,poolLoader:async()=>({fixer:entry({faces:[{name:'IronSmith Mana Fixer'}]}),cards:sets.flatMap(set=>set.cards.map(card=>({card:{...card,draft:{...card.draft,attribution:{sourceFormat:set.source.scope}}},...entry(card.draft)})))}),random:()=>0.5};
test('custom URL stages imported cards and both setup permanents without compilation',async()=>{
 assert.ok(isCustomShowcase('?showcase=custom'));assert.ok(!isCustomShowcase('?showcase=other'));
 const game=mockGame();await loadCustomShowcase(game,options);
 const batch=game.calls.find(c=>c[0]==='stage')[1];
 assert.equal(batch.filter(c=>c.cardName==='Omniscience').length,2);
 assert.equal(batch.filter(c=>c.zoneName==='hand').length,14);
 assert.equal(batch.filter(c=>c.zoneName==='library').length,2);
 assert.equal(batch.filter(c=>c.cardName==='IronSmith Mana Fixer').length,2);
});
test('incompatible artifacts roll back instead of compiling at runtime',async()=>{
 const game=mockGame(true);await assert.rejects(loadCustomShowcase(game,options),/artifact is incompatible/);
 assert.deepEqual(game.calls.slice(-2),[['restore',7],['release',7]]);
 assert.ok(!game.calls.some(c=>c[0]==='stage'));
});
test('missing artifacts fail before any card placement',async()=>{
 const pool=await options.poolLoader();delete pool.cards[0].artifacts;
 const game=mockGame();await assert.rejects(loadCustomShowcase(game,{...options,poolLoader:async()=>pool,librarySize:9}),/missing its compiled artifact/);
 assert.ok(!game.calls.some(c=>c[0]==='stage'));
});
test('imported mechanic variants compile independently of the personal registry',async()=>{
 const {prepareCustomDraftForCreation}=await import('../src/lib/custom-card-library.js');
 for(const reminder of ['Draw a card.','Draw two cards.']) {
  const draft={customMechanicsScope:'card',faces:[{name:'Scoped example',cardTypes:['Sorcery'],oracleText:'Kindle',loyalty:'',defense:''}],customMechanics:[{keyword:'kindle',reminder}]};
  const prepared=await prepareCustomDraftForCreation(draft,async input=>{assert.equal(input.faces[0].loyalty,null);return {canCreate:true,faces:[]};});
  assert.equal(prepared.draft.customMechanics[0].reminder,reminder);
  assert.equal(prepared.draft.customMechanicsScope,'card');
 }
});
test('bundled startup makes one worker deal without runtime previews or set downloads',async()=>{
 const pool={version:1,fixer:{input:{faces:[{name:'IronSmith Mana Fixer'}]}},cards:sets.flatMap(set=>set.cards.map(card=>({card:{...card,draft:{...card.draft,attribution:{sourceFormat:set.source.scope}}},input:card.draft})))};
 let plan;
 const game={populateCustomShowcase:async value=>{plan=value;},previewCustomCard:()=>{throw Error('Startup must not preview drafts');}};
 await loadCustomShowcase(game,{librarySize:1,poolLoader:async()=>pool,random:()=>0.5,setLoader:()=>{throw Error('Startup must not download sets');}});
 assert.equal(plan.prepared.length,16);assert.equal(plan.players.length,2);
 assert.equal(new Set(plan.prepared.map(value=>value.input.faces[0].name)).size,16);
});
test('shipped showcase pool retains source cards and enough unique choices from both formats',()=>{
 const pool=JSON.parse(fs.readFileSync(new URL('custom-sets/showcase.json',root)));
 assert.equal(pool.version,1);assert.ok(pool.cards.length>=100);
 assert.ok(pool.fixer.artifacts?.length);assert.ok(pool.support.some(value=>value.source.group.name==='Omniscience'));
 assert.ok(pool.cards.every(value=>value.source&&value.artifacts?.length),'Every showcase card must be precompiled');
 const names=new Set(),counts={msem:0,rev:0},sets=new Map();
 for(const value of pool.cards){
  const card=value.card,meta=card.draft.attribution;counts[meta.sourceFormat]++;
  assert.ok(!names.has(card.draft.faces[0].name));names.add(card.draft.faces[0].name);
  const key=`${meta.sourceFormat}-${encodeURIComponent(meta.setCode)}`;
  if(!sets.has(key))sets.set(key,JSON.parse(fs.readFileSync(new URL(`custom-sets/${key}.json`,root))));
  assert.deepEqual(card,sets.get(key).cards.find(source=>source.id===card.id));
  assert.equal(value.input.faces[0].name,card.draft.faces[0].name);
 }
 assert.ok(counts.msem>=37 && counts.rev>=37);
});
test('compiled showcase cards use one native bulk placement and no draft compilation',async()=>{
 const {populateCustomShowcase}=await import('../src/lib/custom-showcase-setup.js');
 const game=mockGame();let registrations=0,batch;
 game.registerCompiledCardSourceArtifacts=async()=>{registrations++;};
 game.addCardsToZones=async value=>{batch=value;};
 game.stagePuzzleCardsToZones=async value=>{batch=value;};
 const entry=name=>({input:{faces:[{name}]},source:{group:{name}},artifacts:[{test:true}]});
 const result=await populateCustomShowcase(game,{players:['A'],librarySize:1,prepared:Array.from({length:8},(_,i)=>entry(`Card ${i}`)),fixer:entry('IronSmith Mana Fixer')});
 assert.equal(registrations,9);assert.equal(batch.length,10);assert.equal(result.native,0);
 assert.deepEqual(result.rejected,[]);assert.ok(!game.calls.some(call=>call[0]==='custom'));
});
