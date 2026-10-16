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
