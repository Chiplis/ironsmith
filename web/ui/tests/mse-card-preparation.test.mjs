import test from 'node:test';
import assert from 'node:assert/strict';
import {MseCardPreparer,publicZoneCards} from '../src/lib/mse-card-preparation.js';
import {mseFrameRequest} from '../src/lib/mse/frame-request.mjs';
import {cardFrameSnapshot} from '../src/lib/card-frame-snapshot.mjs';

const elf={id:1,name:'Llanowar Elves',type_line:'Creature — Elf Druid',mana_cost:'{G}',oracle_text:'{T}: Add {G}.',power:'1',toughness:'1'};
const state={players:[
  {battlefield:[elf],graveyard_cards:[{id:2,name:'Lightning Helix',mana_cost:'{R}{W}'}],exile_cards:[{id:3,name:'Hidden',face_down:true}],command_cards:[]},
  {battlefield:[{id:4,name:'Forest',type_line:'Basic Land — Forest'}],graveyard_cards:[],exile_cards:[{id:5,name:'Counterspell',mana_cost:'{U}{U}'}]},
]};

test('public zones list every battlefield first, then graveyards, exiles and command zones, never face-down cards',()=>{
  assert.deepEqual(publicZoneCards(state).map(card=>card.name),['Llanowar Elves','Forest','Lightning Helix','Counterspell']);
  assert.deepEqual(publicZoneCards(null),[]);
});

function preparer(){
  const idle=[],rendered=[],requested=[];
  const printing={set:'fdn',artist:'Kev Walker',rarity:'common',collector_number:'227',frame:'2015',released_at:'2024-11-15',lang:'en'};
  const value=new MseCardPreparer({
    session:{styles:undefined,renderer:{render:async request=>{rendered.push(request);}}},
    defaultFrames:{request:(style,key)=>requested.push([style,key])},
    schedule:callback=>idle.push(callback),
    resolveImage:async name=>`https://cards.scryfall.io/normal/front/0/0/${name.length.toString(16).padStart(8,'0')}-0000-0000-0000-000000000000.jpg`,
    resolvePrinting:async()=>printing,resolveSetIcon:async code=>`https://svgs.scryfall.io/sets/${code}.svg`,
  });
  const run=async()=>{idle.shift()();await new Promise(resolve=>setTimeout(resolve,0));};
  return {value,idle,rendered,requested,printing,run};
}

test('cards are prepared one at a time at idle, with the same request their frame builds',async()=>{
  const {value,idle,rendered,requested,printing,run}=preparer();
  value.update(publicZoneCards(state));
  assert.equal(idle.length,1,'one job is scheduled at a time');
  await run();
  const imageUrl=`https://cards.scryfall.io/normal/front/0/0/${'Llanowar Elves'.length.toString(16).padStart(8,'0')}-0000-0000-0000-000000000000.jpg`;
  const expected=mseFrameRequest(JSON.parse(cardFrameSnapshot(elf)),{imageUrl,printing,setSymbol:'https://svgs.scryfall.io/sets/fdn.svg'}).request;
  assert.deepEqual(rendered[0],expected);
  assert.deepEqual(requested[0],[expected.style,{colors:'G',kind:''}]);
  while(idle.length)await run();
  assert.equal(rendered.length,4);
});

test('updates skip prepared cards, re-prepare changed ones, and dispose stops the queue',async()=>{
  const {value,idle,rendered,run}=preparer();
  value.update([elf]);await run();
  value.update([elf]);
  assert.equal(idle.length,0,'a prepared card is not queued again');
  value.update([{...elf,power:'3'}]);await run();
  assert.equal(rendered.length,2,'a live change prepares the new snapshot');
  value.update([{id:9,name:'Grizzly Bears'}]);value.dispose();
  for(const callback of idle.splice(0))callback();await new Promise(resolve=>setTimeout(resolve,0));
  assert.equal(rendered.length,2);
});
