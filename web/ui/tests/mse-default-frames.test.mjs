import test from 'node:test';
import assert from 'node:assert/strict';
import {MseDefaultFrames,mseDefaultFrameKey,MSE_DEFAULT_FRAME_WARMUP} from '../src/lib/mse/default-frames.mjs';

const STYLE='magic-m15-altered.mse-style';
function frames({fail=new Set()}={}){
  const rendered=[],idle=[];
  const renderer={async render({style,snapshot}){
    const id=`${style}|${snapshot.colors.join('')}|${snapshot.type_line}`;rendered.push(id);
    if(fail.has(id))throw new Error('Render failed');
    return {width:375,height:523,canvas:{id}};
  }};
  const value=new MseDefaultFrames({renderer,schedule:callback=>idle.push(callback),encode:async canvas=>new Blob([canvas.id])});
  return {value,rendered,idle,flush:async()=>{await new Promise(resolve=>setTimeout(resolve));}};
}

test('frame keys follow printed colors, and color identity for lands',()=>{
  assert.deepEqual(mseDefaultFrameKey({type_line:'Creature — Elf',colors:['G'],color_identity:['G','W']}),{colors:'G',kind:''});
  assert.deepEqual(mseDefaultFrameKey({type_line:'Land — Forest Plains',colors:[],color_identity:['W','G']}),{colors:'WG',kind:'land'});
  assert.deepEqual(mseDefaultFrameKey({type_line:'Artifact',colors:[]}),{colors:'',kind:'artifact'});
  assert.deepEqual(mseDefaultFrameKey({layout:'transform',card_faces:[{type_line:'Creature',colors:['R']},{type_line:'Creature',colors:['black']}]},1),{colors:'B',kind:''});
  assert.equal(new Set(MSE_DEFAULT_FRAME_WARMUP.map(k=>`${k.colors}|${k.kind}`)).size,39);
});

test('hand cards and inspector details, which carry no colors, key on their mana cost',()=>{
  // HandCardSnapshot/ObjectDetailsSnapshot: no colors, color_identity or signature.
  assert.deepEqual(mseDefaultFrameKey({name:'Malevolent Rumble',mana_cost:'{1}{G}',card_types:['Sorcery']}),{colors:'G',kind:''});
  assert.deepEqual(mseDefaultFrameKey({mana_cost:'{1}{G/W}',type_line:'Creature'}),{colors:'WG',kind:''});
  assert.deepEqual(mseDefaultFrameKey({card_types:['Land'],oracle_text:'({T}: Add {G}.)'}),{colors:'G',kind:'land'});
  assert.deepEqual(mseDefaultFrameKey({card_types:['Artifact'],mana_cost:'{2}'}),{colors:'',kind:'artifact'});
  // Live colors (e.g. devoid) win over the mana cost.
  assert.deepEqual(mseDefaultFrameKey({type_line:'Creature',mana_cost:'{1}{B}',characteristic_signature:'colors:\ntypes:Creature'}),{colors:'',kind:''});
});

test('warm-up waits for idle time, while a requested frame renders at once',async()=>{
  const {value,rendered,idle,flush}=frames();
  value.warm([{colors:'W',kind:''},{colors:'U',kind:''}]);
  await flush();
  assert.deepEqual(rendered,[]);
  value.request('magic-old.mse-style',{colors:'G',kind:'land'});
  await flush();
  assert.deepEqual(rendered,['magic-old.mse-style|G|Land']);
  idle.shift()();await flush();
  assert.deepEqual(rendered.slice(1),[`${STYLE}|W|Sorcery`]);
});

test('lookup prefers the exact style, then the default style, then its nonland frame',async()=>{
  const {value,idle,flush}=frames();
  let changes=0;value.subscribe(()=>changes++);
  value.warm([{colors:'G',kind:''}]);
  idle.shift()();await flush();
  assert.equal(changes,1);
  const nonland=value.lookup('magic-old.mse-style',{colors:'G',kind:'land'});
  assert.equal(nonland.width,375);
  value.request(STYLE,{colors:'G',kind:'land'});await flush();
  const land=value.lookup('magic-old.mse-style',{colors:'G',kind:'land'});
  assert.notEqual(land.url,nonland.url);
  value.request('magic-old.mse-style',{colors:'G',kind:'land'});await flush();
  assert.notEqual(value.lookup('magic-old.mse-style',{colors:'G',kind:'land'}).url,land.url);
  assert.equal(value.lookup(STYLE,{colors:'U',kind:''}),null);
});

test('a failed frame stays missing without stopping the queue, and dispose stops work',async()=>{
  const {value,rendered,idle,flush}=frames({fail:new Set([`${STYLE}|W|Sorcery`])});
  const warn=console.warn;console.warn=()=>{};
  try{
    value.warm([{colors:'W',kind:''},{colors:'U',kind:''},{colors:'B',kind:''}]);
    idle.shift()();await flush();
    idle.shift()();await flush();
  }finally{console.warn=warn;}
  assert.equal(value.lookup(STYLE,{colors:'W',kind:''}),null);
  assert.ok(value.lookup(STYLE,{colors:'U',kind:''}));
  value.dispose();
  assert.equal(value.lookup(STYLE,{colors:'U',kind:''}),null);
  for(const callback of idle)callback();await flush();
  assert.equal(rendered.length,2);
});
