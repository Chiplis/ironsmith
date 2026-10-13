import {engineDoubleFacesToMse} from '../src/lib/mse/engine-card.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import {engineCardToMse,engineRulesToMse} from '../src/lib/mse/engine-card.mjs';
test('explicit colors and indicators override mismatched costs without changing default frame inference',()=>{
  const back=engineCardToMse({type_line:'Creature',color_indicator:['U','B']}).card;
  assert.equal(back.indicator,'blue, black, multicolor');assert.equal(back.card_color,'blue, black, multicolor');
  assert.equal(engineCardToMse({type_line:'Creature',mana_cost:'{2}{R}',colors:[]}).card.card_color,'colorless');
  assert.equal(engineCardToMse({type_line:'Artifact Creature',mana_cost:'{R}',colors:['U']}).card.card_color,'blue, artifact');
  assert.equal(engineCardToMse({type_line:'Creature',mana_cost:'{W/U}',colors:['W','U']}).card.card_color,undefined);
  assert.equal(engineCardToMse({type_line:'Basic Land — Forest',colors:[],color_identity:['G']}).card.card_color,undefined);
  assert.equal(engineCardToMse({type_line:'Creature',mana_cost:'{R}',colors:['R'],characteristic_signature:'colors: blue\nother: test'}).card.card_color,'blue');
  assert.equal(engineCardToMse({type_line:'Creature',mana_cost:'{R}',characteristic_signature:'colors:-\nother: test'}).card.card_color,'colorless');
  assert.equal(engineCardToMse({type_line:'Creature',color_identity:['U']}).card.card_color,undefined);
});
test('face indicators stay separate and native overrides remain authoritative',()=>{
  const snapshot={color_indicator:['W'],card_faces:[{color_indicator:['U']},{color_indicator:['R']}]};
  const both=engineCardToMse(snapshot).card;
  assert.equal(both.indicator,'blue');assert.equal(both.indicator_2,'red');
  assert.equal(engineCardToMse(snapshot,{faceIndex:1,nativeCard:{indicator:'green'}}).card.indicator,'green');
  assert.throws(()=>engineCardToMse({colors:['purple']}),{code:'engine-colors'});
});
test('engine snapshots map current stats and preserve symbol and literal text boundaries',()=>{
  const source={id:7,name:'Trial',mana_cost:'{2}{W/U}{B/P}',type_line:'Legendary Creature — Wizard',oracle_text:'{T}: Add {U}. <b> is literal.',power_toughness:'3/4',power:'5',rarity:'mythic',flavor_text:'A <b>literal</b> story.'};
  const result=engineCardToMse(source);
  assert.equal(result.card.casting_cost,'2W/UB/P');
  assert.equal(result.card.super_type,'Legendary Creature');assert.equal(result.card.sub_type,'Wizard');
  assert.equal(result.card.rule_text,'<sym>T</sym>: Add <sym>U</sym>. \u0001b> is literal.');
  assert.equal(result.card.power,'5');assert.equal(result.card.toughness,'4');
  assert.equal(result.card.flavor_text,'A \u0001b>literal\u0001/b> story.');
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

test('planeswalker costs use paragraph slots while preserving static abilities and symbol markup',()=>{
  const card=engineCardToMse({type_line:'Legendary Planeswalker — Tester',oracle_text:'Flying\n+1: Add {U}.\n0: Draw a card.\n−X: Deal X damage.\nYou may activate abilities twice.'}).card;
  assert.equal(card.rule_text,'Flying\nAdd <sym>U</sym>.\nDraw a card.\nDeal X damage.\nYou may activate abilities twice.');
  assert.equal(card.loyalty_cost_1,undefined);
  assert.equal(card.loyalty_cost_2,'+1');assert.equal(card.loyalty_cost_3,'0');assert.equal(card.loyalty_cost_4,'-X');
  assert.equal(card.level_1_text,'Flying');assert.equal(card.level_2_text,'Add <sym>U</sym>.');
  assert.equal(engineCardToMse({type_line:'Creature',oracle_text:'0: Draw a card.'}).card.rule_text,'0: Draw a card.');
});

test('planeswalker face costs use native eight-slot offsets and explicit selection resets slots',()=>{
  const snapshot={card_faces:[{type_line:'Planeswalker',oracle_text:'+1: First.'},{type_line:'Planeswalker',oracle_text:'−2: Second.'}]};
  const both=engineCardToMse(snapshot).card;
  assert.equal(both.loyalty_cost_1,'+1');assert.equal(both.loyalty_cost_9,'-2');assert.equal(both.rule_text_2,'Second.');
  assert.equal(both.level_9_text,'Second.');
  const selected=engineCardToMse(snapshot,{faceIndex:1,nativeCard:{loyalty_cost_1:'-3'}}).card;
  assert.equal(selected.loyalty_cost_1,'-3');assert.equal(selected.loyalty_cost_9,undefined);
});

test('Saga grouped chapters map labels to ability boxes without duplicating their text',()=>{
  const rules='(Add a lore counter.)\nI, II — Add {U}.\nIII — Draw a card.\nThen discard a card.';
  const result=engineCardToMse({type_line:'Enchantment — Saga',oracle_text:rules});
  assert.equal(result.card.chapter_text,'(Add a lore counter.)');
  assert.equal(result.card.level_1_text,'Add <sym>U</sym>.');
  assert.equal(result.card.level_2_text,'Draw a card.\nThen discard a card.');
  assert.equal(result.styling.chapter_textboxes,'two');assert.equal(result.styling.snap_chapter_to_box,'1,1,2');
  assert.equal(result.card.rule_text,engineRulesToMse(rules));
  assert.equal(engineCardToMse({type_line:'Enchantment — Saga',oracle_text:rules},{styling:{chapter_textboxes:'three'}}).styling.chapter_textboxes,'three');
});

test('Saga adaptation preserves face offsets, explicit selection, and non-Saga text',()=>{
  const snapshot={card_faces:[{type_line:'Creature',oracle_text:'First.'},{type_line:'Enchantment — Saga',oracle_text:'I — One.\nII — Two.'}]};
  const both=engineCardToMse(snapshot);
  assert.equal(both.card.level_9_text,'One.');assert.equal(both.styling.snap_chapter_to_box_2,'1,2');
  const selected=engineCardToMse(snapshot,{faceIndex:1});assert.equal(selected.card.level_1_text,'One.');
  assert.equal(engineCardToMse({type_line:'Instant',oracle_text:'I — One.'}).card.level_1_text,undefined);
  assert.throws(()=>engineCardToMse({type_line:'Enchantment — Saga',oracle_text:'I, III — One.'}),{code:'engine-saga'});
});


test('double-faced snapshots become reciprocal linked cards with independent face fields',()=>{
  const snapshot={id:'example',layout:'modal_dfc',card_faces:[
    {name:'Front',type_line:'Creature',power:'2',toughness:'3',oracle_text:'First.'},
    {name:'Back',type_line:'Planeswalker',loyalty:'4',oracle_text:'+1: Draw a card.'},
  ]};
  const result=engineDoubleFacesToMse(snapshot,{faceIndex:1,rulesText:'-2: Scry 2.',nativeCard:{image:'back-art'}});
  const [front,back]=result.cards;
  assert.equal(result.selected,1);
  assert.equal(front.card.name,'Front');assert.equal(front.card.name_2,undefined);
  assert.equal(front.card.rule_text,'First.');assert.equal(front.card.image,undefined);
  assert.equal(back.card.name,'Back');assert.equal(back.card.loyalty_cost_1,'-2');assert.equal(back.card.image,'back-art');
  assert.equal(front.card.linked_card_1,back.uid);assert.equal(back.card.linked_card_1,front.uid);
  assert.equal(front.card.linked_relation_1,'Back Face');assert.equal(back.card.linked_relation_1,'Front Face');
  assert.equal(front.card.transformation,'modal front');assert.equal(back.card.transformation,'modal back');
  assert.equal(engineDoubleFacesToMse({...snapshot,layout:'split'}),null);
  assert.throws(()=>engineDoubleFacesToMse(snapshot,{faceIndex:2}),{code:'engine-face'});
  assert.throws(()=>engineDoubleFacesToMse({...snapshot,card_faces:[{}]}),{code:'engine-face'});
});
