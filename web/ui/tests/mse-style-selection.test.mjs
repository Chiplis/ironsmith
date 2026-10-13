import test from 'node:test';
import assert from 'node:assert/strict';
import {selectMseStyle} from '../src/lib/mse/style-selection.mjs';

test('automatic frames follow canonical card types and multi-part layouts',()=>{
  for(const [snapshot,style] of [
    [{type_line:'Creature — Wizard'},'magic-m15-altered'],
    [{type_line:'Enchantment — Saga'},'magic-m15-saga'],
    [{type_line:'Legendary Planeswalker — Test'},'magic-m15-planeswalker'],
    [{type_line:'Battle — Siege'},'magic-m15-mainframe-battles'],
    [{layout:'split',card_faces:[{type_line:'Instant'},{type_line:'Sorcery'}]},'magic-m15-split-fusable'],
    [{layout:'split',card_faces:[{}, {oracle_text:'Aftermath (Cast this spell only from your graveyard.)'}]},'magic-m15-aftermath'],
    [{layout:'adventure',card_faces:[{type_line:'Creature'},{}]},'magic-m15-adventure'],
    [{type_line:'Battle — Siege // Creature',card_faces:[{type_line:'Battle — Siege'},{}]},'magic-m15-mainframe-battles'],
    [{type_line:'Enchantment — Saga',type_line_display:'Enchantement : saga'},'magic-m15-saga'],
  ])assert.equal(selectMseStyle(snapshot),style+'.mse-style');
});
test('explicit styles are unrestricted and take precedence over automatic defaults',()=>{
  const card={type_line:'Battle — Siege'};
  assert.equal(selectMseStyle(card,{defaultStyle:'custom.mse-style'}),'custom.mse-style');
  assert.equal(selectMseStyle(card,{style:'another-game.mse-style',defaultStyle:'custom.mse-style'}),'another-game.mse-style');
  assert.equal(selectMseStyle(card,{style:'auto',defaultStyle:'auto'}),'magic-m15-mainframe-battles.mse-style');
});
