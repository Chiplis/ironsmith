import test from 'node:test';
import assert from 'node:assert/strict';
import {sourceMaskLayoutGap} from '../src/lib/card-frame-layout.js';
test('segmented layouts cannot masquerade as supported conventional masks',()=>{
  for(const layout of ['leveler','prototype','adventure','split','saga','class','mutate','prepare'])assert.equal(sourceMaskLayoutGap({layout}),`layout-${layout}`);
  assert.equal(sourceMaskLayoutGap({layout:'normal',type_line:'Legendary Planeswalker — Jace'}),'loyalty-panels');
  assert.equal(sourceMaskLayoutGap({layout:'normal',keywords:['Station']}),'station-ranks');
});
test('face type distinguishes a horizontal battle from its ordinary reverse',()=>{
  assert.equal(sourceMaskLayoutGap({layout:'transform',type_line:'Battle — Siege'}),'horizontal-battle');
  assert.equal(sourceMaskLayoutGap({layout:'transform',type_line:'Creature — Elemental'}),null);
  assert.equal(sourceMaskLayoutGap({layout:'normal',frame:'1997',type_line:'Land — Forest Island'}),null);
});

test('localized placeholder images request the original/English fallback path',()=>{
  assert.equal(sourceMaskLayoutGap({layout:'normal',image_status:'placeholder'}),'placeholder-image');
});
