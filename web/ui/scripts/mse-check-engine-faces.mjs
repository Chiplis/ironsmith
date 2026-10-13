import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {corpusStore} from './mse-corpus-store.mjs';
import {MseCardRenderer} from '../src/lib/mse/card-renderer.mjs';
import {selectMseStyle} from '../src/lib/mse/style-selection.mjs';
import {text} from '../src/lib/mse/script-values.mjs';

const [directory,output]=process.argv.slice(2);
if(!directory||!output)throw new Error('Usage: mse-check-engine-faces CORPUS OUTPUT.json');
const renderer=new MseCardRenderer({store:await corpusStore(directory),fonts:{},imageRenderer:{},fieldUpdateMode:'ordered'});
// Exercise real adapter/template initialization without substituting template
// scripts. Drawing is covered separately; this check needs no browser fonts.
renderer.renderContext=async context=>context;
const results=[];
for(const layout of ['transform','modal_dfc']){
 const snapshot={id:'face-trial',layout,card_faces:[
  {name:'Day Trial',mana_cost:'{1}{U}',type_line:'Creature — Wizard',oracle_text:'Draw a card.',power:'2',toughness:'3'},
  {name:'Night Trial',type_line:'Legendary Planeswalker — Trial',oracle_text:'+1: Draw a card.\n-2: Scry 2.',loyalty:'4'},
 ]};
 for(const faceIndex of [0,1]){
  const style=selectMseStyle(snapshot,{faceIndex});
  const context=await renderer.render({style,snapshot,adapter:{faceIndex}});
  const name=text(context.field('card','name'));
  const counterpart=text(context.runtime.run(faceIndex?'get_front_face(card).name':'get_back_face(card).name'));
  assert.equal(name,snapshot.card_faces[faceIndex].name);
  assert.equal(counterpart,snapshot.card_faces[1-faceIndex].name);
  assert.deepEqual(context.diagnostics,[]);
  const transformation=text(context.field('card','transformation'));
  if(layout==='modal_dfc')assert.ok(transformation.includes(faceIndex?'modal back':'modal front'));
  results.push({layout,faceIndex,style,name,counterpart,transformation,diagnostics:context.diagnostics});
 }
}
renderer.dispose();
await writeFile(output,JSON.stringify({success:true,scope:'Real pinned templates, engine DFC adaptation and linked-face script initialization. No drawing or native visual parity assertion.',results},null,2)+'\n');
console.log(JSON.stringify({success:true,cases:results.length}));
