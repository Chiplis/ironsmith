import test from 'node:test';
import assert from 'node:assert/strict';
import {CardGlintRenderQueue} from '../src/lib/card-glint/render-queue.mjs';
import {CardGlintImageCache} from '../src/lib/card-glint/image-cache.mjs';
import {CardGlintPreparer,cardGlintZoneCards,prepareCardGlintFace} from '../src/lib/card-glint/preparation.mjs';
import {cardGlintFace} from '../src/lib/card-glint/card.mjs';
import {cardFrameSnapshot} from '../src/lib/card-frame-snapshot.mjs';
const flush=()=>new Promise(resolve=>setImmediate(resolve));
const card=name=>({name,type_line:'Creature',oracle_text:'Draw a card.'});
const raster=()=>({canvas:{width:10,height:10}});

test('initial and updated board preparations include our hand first, every battlefield, and no hidden identities',()=>{
  const state={perspective:1,players:[{id:0,hand_cards:[card('Opponent secret')],battlefield:[card('Enemy field'),{...card('Face down'),face_down:true}]},
    {id:1,hand_cards:[card('Our hand'),card('Hidden card')],battlefield:[card('Our field')]}]};
  assert.deepEqual(cardGlintZoneCards(state).map(c=>c.name),['Our hand','Enemy field','Our field']);
  assert.deepEqual(cardGlintZoneCards({...state,perspective:null}).map(c=>c.name),['Enemy field','Our field']);
  assert.deepEqual(cardGlintZoneCards(null),[]);
});
test('public zone frames warm for every player and follow zone changes without hidden cards',()=>{
  const state={perspective:1,players:[
    {id:0,command_cards:[card('Enemy commander')],exile_cards:[card('Enemy exile'),{...card('Secret exile'),face_down:true}],graveyard_cards:[card('Enemy graveyard')]},
    {id:1,hand_cards:[card('Our hand')],battlefield:[card('Our field')],command_cards:[card('Our commander')],exile_cards:[card('Our exile'),card('Hidden card')],graveyard_cards:[card('Our graveyard')]},
  ]};
  assert.deepEqual(cardGlintZoneCards(state).map(c=>c.name),[
    'Our hand','Our field','Enemy commander','Our commander','Enemy exile','Our exile','Enemy graveyard','Our graveyard',
  ]);
  state.players[1].graveyard_cards.push(state.players[1].battlefield.pop());
  assert.deepEqual(cardGlintZoneCards(state).map(c=>c.name),[
    'Our hand','Enemy commander','Our commander','Enemy exile','Our exile','Enemy graveyard','Our graveyard','Our field',
  ]);
});
test('warming builds the exact foreground face, including the displayed reverse face and live snapshot',async()=>{
  const source={name:'Back',layout:'transform',card_faces:[{name:'Front'}, {...card('Back'),power:'3',toughness:'4'}]};
  const imageUrl='https://cards.scryfall.io/normal/back/a/b/test.jpg', printing={set:'isd',frame:'2003'};
  const actual=await prepareCardGlintFace(source,{resolveImage:async()=>imageUrl,resolvePrinting:async()=>printing});
  assert.deepEqual(actual,cardGlintFace(JSON.parse(cardFrameSnapshot(source)),{imageUrl,printing,faceIndex:1}));
});
test('foreground bypasses pending background work and joins an existing warm render',async()=>{
  const started=[],finish=new Map();
  const queue=new CardGlintRenderQueue(face=>new Promise(resolve=>{started.push(face.name);finish.set(face.name,()=>resolve(raster()));}));
  const a=queue.render(card('a'),{background:true}), b=queue.render(card('b'),{background:true});
  await flush();assert.deepEqual(started,['a']);
  const foreground=queue.render(card('visible'));await flush();assert.deepEqual(started,['a','visible']);
  assert.equal(queue.render(card('a')),a,'foreground joins in-flight background promise');
  finish.get('visible')();await foreground;await flush();assert.deepEqual(started,['a','visible']);
  finish.get('a')();await a;await flush();assert.deepEqual(started,['a','visible','b']);
  finish.get('b')();await b;
  assert.equal(queue.render(card('b')),b,'completed warming is reused');
});
test('failed renders retry and in-flight jobs survive completed-frame eviction',async()=>{
  let attempts=0;
  const queue=new CardGlintRenderQueue(async()=>{if(!attempts++)throw Error('temporary');return raster();},{maxBytes:400});
  await assert.rejects(queue.render(card('a')));await queue.render(card('a'));
  await queue.render(card('b'));assert.equal(queue.has(card('a')),false);assert.equal(queue.has(card('b')),true);
});
test('preparer replaces stale work, skips cached cards, warms live changes, and stops on disposal',async()=>{
  const scheduled=[],rendered=[],cache=new Set();let release;
  const value=new CardGlintPreparer({concurrency:1,schedule:callback=>scheduled.push(callback),
    prepare:async card=>card.name==='slow'?await new Promise(resolve=>{release=()=>resolve(card);}):card,
    has:face=>cache.has(JSON.stringify(face)),render:async(face,options)=>{assert.equal(options.background,true);rendered.push(face);cache.add(JSON.stringify(face));}});
  value.update([card('a'),card('b')]);await scheduled.shift()();
  value.update([card('a'),card('c')]);await scheduled.shift()(); // stale b is discarded
  await scheduled.shift()();assert.deepEqual(rendered.map(c=>c.name),['a','c']);
  value.update([card('a'),card('c')]);assert.equal(scheduled.length,0);
  value.update([{...card('a'),power:'5'}]);await scheduled.shift()();assert.equal(rendered.at(-1).power,'5');
  value.update([card('slow')]);const running=scheduled.shift()();await flush();
  value.update([card('new board')]);release();await running;await scheduled.shift()();
  assert.equal(rendered.some(c=>c.name==='slow'),false);
  value.update([card('after dispose')]);value.dispose();await scheduled.shift()();
  assert.equal(rendered.some(c=>c.name==='after dispose'),false);
});
test('shared decoded assets retain upstream aliases and stay alive until all painters release them',async()=>{
  const loaded=[],closed=[];
  const cache=new CardGlintImageCache(async urls=>{
    const url=urls[0];loaded.push(url);const bitmap={width:4,height:4};
    return {images:new Map([[url,bitmap],[url+'-historical-detail',bitmap]]),dispose:()=>closed.push(url)};
  },{maxBytes:64});
  const signal=new AbortController().signal;
  const [a,b]=await Promise.all([cache.images(['frame'],signal),cache.images(['frame'],signal)]);
  assert.deepEqual(loaded,['frame']);assert.equal(a.images.get('frame'),b.images.get('frame'));
  assert.ok(a.images.has('frame-historical-detail'));
  const c=await cache.images(['art'],signal);c.dispose();
  assert.deepEqual(closed,['art'],'leased frame cannot be evicted');a.dispose();b.dispose();
  const d=await cache.images(['new'],signal);d.dispose();assert.ok(closed.includes('frame'));
});


test('failed printing lookups still prepare custom cards with default frames', async () => {
  const card={name:'Custom Golem',card_types:['artifact','creature'],power_toughness:'4/4'};
  const missing=await prepareCardGlintFace(card,{resolveImage:async()=>{throw new Error('not found');}});
  assert.equal(missing.frameStyle,'modern'); assert.equal(missing.artifact,true);
  const art='data:image/png;base64,custom-art';
  const noPrinting=await prepareCardGlintFace(card,{resolveImage:async()=>art,resolvePrinting:async()=>{throw new Error('offline');}});
  assert.equal(noPrinting.frameStyle,'modern'); assert.equal(noPrinting.illustration,art);
});
