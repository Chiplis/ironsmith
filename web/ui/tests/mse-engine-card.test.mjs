import test from 'node:test';
import assert from 'node:assert/strict';
import {engineCardToMse,engineRulesToMse} from '../src/lib/mse/engine-card.mjs';
test('engine snapshots map current stats and preserve symbol and literal text boundaries',()=>{
  const source={id:7,name:'Trial',mana_cost:'{2}{W/U}{B/P}',type_line:'Legendary Creature — Wizard',oracle_text:'{T}: Add {U}. <b> is literal.',power_toughness:'3/4',power:'5',rarity:'mythic'};
  const result=engineCardToMse(source);
  assert.equal(result.card.casting_cost,'2W/UB/P');
  assert.equal(result.card.super_type,'Legendary Creature');assert.equal(result.card.sub_type,'Wizard');
  assert.equal(result.card.rule_text,'<sym>T</sym>: Add <sym>U</sym>. \u0001b> is literal.');
  assert.equal(result.card.power,'5');assert.equal(result.card.toughness,'4');
  assert.equal(result.card.rarity,'mythic rare');assert.equal(result.uid,7);
  assert.equal(source.oracle_text,'{T}: Add {U}. <b> is literal.');
});
test('multiple faces retain separate rules, and explicit active-face selection uses the first MSE fields',()=>{
  const snapshot={card_faces:[{name:'Front',type_line:'Instant',oracle_text:'Draw a card.'},{name:'Back',type_line:'Battle — Siege',oracle_text:'Draw two cards.',defense:'4'}]};
  const all=engineCardToMse(snapshot);
  assert.equal(all.card.name_2,'Back');assert.equal(all.card.loyalty_2,'4');
  const active=engineCardToMse(snapshot,{faceIndex:1,rulesText:'Updated {U}.',nativeCard:{name:'Display name'}});
  assert.equal(active.card.name,'Display name');assert.equal(active.card.rule_text,'Updated <sym>U</sym>.');assert.equal(active.card.name_2,undefined);
  assert.throws(()=>engineCardToMse(snapshot,{faceIndex:2}),{code:'engine-face'});
  assert.equal(engineRulesToMse('Pay {2/W}.\nThen draw.'),'Pay <sym>2/W</sym>.\nThen draw.');
});
