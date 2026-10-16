import test from 'node:test';
import assert from 'node:assert/strict';
import {cardGlintFace} from '../src/lib/card-glint/card.mjs';
import {measuredRegions,cardGlintActionsCovered} from '../src/lib/card-glint/interactions.mjs';

test('live text, colors and stats override physical printing metadata', () => {
  const face = cardGlintFace({name:'Changed',oracle_text:'Draw two cards.',mana_cost:'{2}{U}',type_line:'Creature — Wizard',
    colors:['R'],power:'5',toughness:'6'}, {printing:{oracle_text:'Old text',colors:['U'],set:'lea',frame:'1993',artist:'Artist'},
    imageUrl:'https://cards.scryfall.io/normal/front/a/b/id.jpg'});
  assert.equal(face.rulesText,'Draw two cards.'); assert.deepEqual(face.colors,['R']);
  assert.deepEqual(face.manaSymbols,['2','U']); assert.equal(face.power,'5');
  assert.equal(face.frameStyle,'retro'); assert.equal(face.artist,'Artist');
  assert.equal(face.illustration,'https://cards.scryfall.io/art_crop/front/a/b/id.jpg');
});
test('faces, split cards, land identity and colorless changes survive adaptation', () => {
  assert.equal(cardGlintFace({card_faces:[{name:'Front'},{name:'Back',oracle_text:'Back rules'}]},{faceIndex:1}).name,'Back');
  const split=cardGlintFace({layout:'split',card_faces:[{name:'Left'},{name:'Right'}]});
  assert.deepEqual(split.faces.map(face=>face.name),['Left','Right']);
  assert.deepEqual(cardGlintFace({type_line:'Land',color_identity:['U'],intrinsic_mana:['G']}).frameColors,['U','G']);
  assert.deepEqual(cardGlintFace({colors:[],mana_cost:'{U}'}).colors,[]);
});
test('measured wrapped text creates targets and never claims missing actions are covered', () => {
  const context={measureText:text=>({width:text.length*5})};
  const run=text=>({text,width:text.length*5,size:10,ascent:8,descent:2});
  const lines=[{role:'rules',runs:[run('Draw ')],baseline:20},{role:'rules',runs:[run('two cards.')],baseline:30}];
  const view={lines:['Draw two cards.','Missing ability'],actions:new Map([[0,[{id:1}]],[1,[{id:2}]]])};
  const regions=measuredRegions([{x:10,lines}],context,()=>'',view);
  assert.equal(regions.length,2); assert.equal(regions[0].left,10); assert.equal(regions[0].top,12);
  assert.equal(cardGlintActionsCovered(regions,view),false);
  view.actions.delete(1); assert.equal(cardGlintActionsCovered(regions,view),true);
});


test('unmatched cards use explicit modern defaults and their own characteristics', () => {
  const face = cardGlintFace({name:'Custom Sentinel',set:'lea',card_types:['artifact','creature'],
    supertypes:['legendary'],subtypes:['Golem'],colors:[],power_toughness:'3/7',oracle_text:'Vigilance'}, {printing:null});
  assert.equal(face.frameStyle,'modern');
  assert.equal(face.borderColor,'black');
  assert.equal(face.securityStamp,'none');
  assert.equal(face.layout,'normal');
  assert.equal(face.typeLine,'Legendary Artifact Creature — Golem');
  assert.equal(face.artifact,true); assert.equal(face.creature,true); assert.equal(face.legendary,true);
  assert.deepEqual(face.colors,[]); assert.equal(face.power,'3'); assert.equal(face.toughness,'7');
  assert.equal(face.illustration,null); assert.equal(face.artist,undefined);
  assert.equal(cardGlintFace({type_line:'Battle — Siege',defense:4}).frameTemplate,'battle');
  const split=cardGlintFace({layout:'split',card_faces:[{name:'Left'},{name:'Right'}]});
  assert.ok(split.faces.every(face=>face.layout==='normal' && face.frameStyle==='modern'));
  const printed=cardGlintFace({name:'Printed'},{printing:{frame:'1997',border_color:'white'}});
  assert.equal(printed.frameStyle,'retro'); assert.equal(printed.borderColor,'white');
});

test('custom source URLs are not printed as flavor while actual flavor is preserved',()=>{
  const card={name:'Custom Island',type_line:'Basic Land — Island',oracle_text:'',flavor_text:'https://www.reddit.com/r/custommagic/comments/ch910l/bob_ross_basic_lands/'};
  assert.deepEqual(cardGlintFace(card,{printing:null}).rulesParagraphs,[]);
  const flavor='A quiet shore beneath an endless sky.';
  assert.deepEqual(cardGlintFace({...card,flavor_text:flavor}).rulesParagraphs,[{role:'flavor',runs:[{text:flavor,italic:true}]}]);
});

test('background frames preserve type casing and subtypes from lightweight snapshots',()=>{
  const face=cardGlintFace({name:'Myr Moonvessel',card_types:['artifact','creature'],characteristic_signature:'types:artifact,creature\nsupertypes:\nsubtypes:Myr'});
  assert.equal(face.typeLine,'Artifact Creature — Myr');
  assert.equal(face.artifact,true);
  assert.equal(face.creature,true);
});
