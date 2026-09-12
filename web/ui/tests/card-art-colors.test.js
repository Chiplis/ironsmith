import test from 'node:test';
import assert from 'node:assert/strict';
import { cardArtColors, cardArtSymbolLayout } from '../src/lib/card-art-colors.js';
test('loader uses live colors before mana costs and keeps colorless cards colorless',()=>{
 assert.deepEqual(cardArtColors({characteristic_signature:'name:Test\ncolors:-\ntypes:Creature',mana_cost:'{R}'}),[]);
 assert.deepEqual(cardArtColors({characteristic_signature:'colors:Blue,Red',mana_cost:'{G}'}),['U','R']);
 assert.deepEqual(cardArtColors({colors:[],mana_cost:'{B}'}),[]);
 assert.deepEqual(cardArtColors({mana_cost:'{2}{W/U}{B/P}{W}'}),['W','U','B']);
 assert.deepEqual(cardArtColors({color_identity:['G'],produced_mana:['G']}),[]);
});
test('every color combination fits in the viewbox without overlapping symbols',()=>{
 for(let mask=0;mask<32;mask++){
  const colors=['W','U','B','R','G'].filter((_,i)=>mask&(1<<i));
  const layout=cardArtSymbolLayout(colors);
  assert.deepEqual(layout.map(s=>s.color),colors.length?colors:['C']);
  for(const s of layout){assert.ok(s.x>=0&&s.y>=0&&s.x+s.size<=100&&s.y+s.size<=100);}
  for(let i=0;i<layout.length;i++)for(let j=i+1;j<layout.length;j++){
   const a=layout[i],b=layout[j];assert.ok(Math.hypot(a.x-b.x,a.y-b.y)>=(a.size+b.size)/2);
  }
 }
});
