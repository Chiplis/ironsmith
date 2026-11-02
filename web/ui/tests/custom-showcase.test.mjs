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
 const calls=[];return {calls,previewCustomCard:async()=>({canCreate:true}),createRuntimeSavepoint:async()=>7,
  resetEmpty:async(...v)=>calls.push(['reset',...v]),addCardToZone:async(...v)=>calls.push(['official',...v]),
  createCustomCard:async v=>{if(fail)throw Error('insertion failed');calls.push(['custom',v]);},finishPuzzleSetup:async()=>calls.push(['finish']),
  restoreRuntimeSavepoint:async id=>calls.push(['restore',id]),releaseRuntimeSavepoint:async id=>calls.push(['release',id])};
}
const sets=['msem','rev','msem','rev'].map((scope,i)=>({id:String(i),source:{scope},cards:Array.from({length:8},(_,n)=>({id:`${i}-${n}`,draft:{faces:[{name:`Card ${i}-${n}`}],customMechanics:[]}}))}));
const options={librarySize:1,catalogLoader:async()=>({sets}),setLoader:async s=>s,prepare:async draft=>({input:draft}),random:()=>0.5};
test('custom URL deals imported cards and both setup permanents to every seat',async()=>{
 assert.ok(isCustomShowcase('?showcase=custom'));assert.ok(!isCustomShowcase('?showcase=other'));
 const game=mockGame();await loadCustomShowcase(game,options);
 assert.equal(game.calls.filter(c=>c[0]==='official'&&c[2]==='Omniscience').length,2);
 const created=game.calls.filter(c=>c[0]==='custom').map(c=>c[1]);
 assert.equal(created.filter(c=>c.zoneName==='hand').length,14);assert.equal(created.filter(c=>c.zoneName==='library').length,2);
 assert.equal(created.filter(c=>c.draft.faces[0].name==='IronSmith Mana Fixer').length,2);
});
test('insertion failure rolls the table back and releases its savepoint',async()=>{
 const game=mockGame(true);await assert.rejects(loadCustomShowcase(game,options),/insertion failed/);
 assert.deepEqual(game.calls.slice(-2),[['restore',7],['release',7]]);
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
