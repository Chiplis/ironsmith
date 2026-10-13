import {engineCardToMse} from '../src/lib/mse/engine-card.mjs';
import {selectMseStyle} from '../src/lib/mse/style-selection.mjs';
const doubleFaces=['transform','modal_dfc'].flatMap(layout=>{
  const snapshot={id:`trial-${layout}`,layout,card_faces:[
    {name:'Day Trial',mana_cost:'{1}{U}',type_line:'Creature — Wizard',oracle_text:'Draw a card.',power:'2',toughness:'3'},
    {name:'Night Trial',type_line:'Legendary Planeswalker — Trial',oracle_text:'+1: Draw a card.\n-2: Scry 2.',loyalty:'4'},
  ]};
  return [0,1].map(faceIndex=>({id:`engine-${layout}-${faceIndex}`,snapshot,faceIndex,style:selectMseStyle(snapshot,{faceIndex}),dimensions:[375,523],text:{'card.name':snapshot.card_faces[faceIndex].name}}));
});
const frameFamilies=[
  ['old','magic-old',375,523],['new','magic-new',375,523],['future','magic-future',375,523],
  ['kaldheim','magic-m15-showcase-kaldheim',750,1046],['invocation','magic-m15-invocation',375,523],
].map(([id,style,width,height])=>({
  id:`family-${id}`,style:`${style}.mse-style`,dimensions:[width,height],
  context:engineCardToMse({name:'Archive Scholar',mana_cost:'{2}{U}{B}',type_line:'Legendary Creature — Human Wizard',
    oracle_text:'Flying\nWhenever this creature attacks, draw a card, then discard a card.\n{1}{U}, {T}: Scry 2.',
    flavor_text:'Every frame preserves a different age.',rarity:'rare',power:'3',toughness:'4'}),
  text:{'card.name':'Archive Scholar','card.type':'Legendary Creature — Human Wizard',
    'card.text':'Flying\nWhenever this creature attacks, draw a card, then discard a card.\n1U, T: Scry 2.\nEvery frame preserves a different age.',
    ...(id==='invocation'?{'card.power':'3','card.toughness':'4'}:{'card.pt':'3/4'})},
}));
// Original diagnostic cards: exercise layout families without external card art.
export default [
  ...doubleFaces,
  ...frameFamilies,
  {id:'engine-printing',style:'magic-m15-altered.mse-style',dimensions:[375,523],context:engineCardToMse({name:'Collected Trial',mana_cost:'{1}{U}',type_line:'Creature — Wizard',oracle_text:'When this creature enters, draw a card.',flavor_text:'Every margin holds another story.',rarity:'mythic',power:'2',toughness:'3'}),text:{'card.name':'Collected Trial','card.card_code':'M 0001 ','card.text':'When this creature enters, draw a card.\nEvery margin holds another story.'}},
  {id:'engine-color-indicator',style:'magic-m15-altered.mse-style',dimensions:[375,523],context:engineCardToMse({name:'Blue Reflection',type_line:'Creature — Spirit',color_indicator:['U'],oracle_text:'Flying',power:'2',toughness:'2'}),text:{'card.name':'Blue Reflection','card.type':'Creature — Spirit'}},
  {id:'engine-colorless',style:'magic-m15-altered.mse-style',dimensions:[375,523],context:engineCardToMse({name:'Colorless Trial',type_line:'Creature — Eldrazi',mana_cost:'{2}{R}',colors:[],oracle_text:'Devoid',power:'3',toughness:'2'}),text:{'card.name':'Colorless Trial','card.casting_cost':'2R'}},
  {id:'engine-modal',style:'magic-m15-altered.mse-style',dimensions:[375,523],context:engineCardToMse({name:'A Choice of Paths',mana_cost:'{1}{U}',type_line:'Instant',oracle_text:'Choose one —\n• Draw two cards, then discard a card.\n• Return target creature to its owner’s hand.'}),text:{'card.name':'A Choice of Paths','card.text':'Choose one —\n• Draw two cards, then discard a card.\n• Return target creature to its owner’s hand.\n'}},
  {id:'engine-adventure',style:'magic-m15-adventure.mse-style',dimensions:[375,523],context:engineCardToMse({layout:'adventure',card_faces:[{name:'Traveling Scholar',mana_cost:'{2}{U}',type_line:'Creature — Wizard',oracle_text:'When this creature enters, draw a card.',power:'2',toughness:'2'},{name:'Study Abroad',mana_cost:'{U}',type_line:'Sorcery — Adventure',oracle_text:'Scry 2.'}]}),text:{'card.name':'Traveling Scholar','card.name_2':'Study Abroad','card.text':'When this creature enters, draw a card.\n','card.text_2':'Scry 2.\n','card.pt':'2/2'}},
  {id:'engine-aftermath',style:'magic-m15-aftermath.mse-style',dimensions:[375,523],context:engineCardToMse({layout:'split',card_faces:[{name:'Remember',mana_cost:'{U}',type_line:'Instant',oracle_text:'Draw a card.'},{name:'Recollect',mana_cost:'{2}{U}',type_line:'Sorcery',oracle_text:'Aftermath\nDraw two cards.'}]}),text:{'card.name':'Remember','card.name_2':'Recollect','card.text':'Draw a card.\n','card.text_2':'Aftermath (Cast this spell only from your graveyard. Then exile it.)\nDraw two cards.\n'}},
  {id:'engine-saga',style:'magic-m15-saga.mse-style',dimensions:[375,523],context:engineCardToMse({name:'Shared Chapters',mana_cost:'{2}{U}',type_line:'Enchantment — Saga',oracle_text:'I, II — Draw a card.\nIII — Scry 2.'}),text:{'card.name':'Shared Chapters','card.level_1_text':'Draw a card.','card.level_2_text':'Scry 2.'}},
  {id:'engine-mainframe-planeswalker',style:'magic-m15-mainframe-planeswalker.mse-style',dimensions:[375,523],context:engineCardToMse({name:'Mainframe Trial',mana_cost:'{2}{U}{U}',type_line:'Legendary Planeswalker — Tester',oracle_text:'+1: Draw a card.\n0: Scry 2.\n−3: Draw three cards.',loyalty:'4'}),text:{'card.name':'Mainframe Trial','card.loyalty_cost_1':'+1','card.loyalty_cost_3':'-3','card.text':'Draw a card.\nScry 2.\nDraw three cards.\n'}},
  {id:'engine-planeswalker',style:'magic-m15-planeswalker.mse-style',dimensions:[375,523],context:engineCardToMse({name:'Adapter Trial',mana_cost:'{2}{U}{U}',type_line:'Legendary Planeswalker — Tester',oracle_text:'+1: Draw a card.\n0: Scry 2.\n−3: Draw three cards.',loyalty:'4'}),text:{'card.name':'Adapter Trial','card.loyalty_cost_1':'+1','card.loyalty_cost_3':'-3','card.text':'Draw a card.\nScry 2.\nDraw three cards.\n'}},
  {id:'standard',style:'magic-m15-altered.mse-style',dimensions:[375,523],context:{card:{name:'Frame Trial',casting_cost:'1U',super_type:'Creature',sub_type:'Wizard',rule_text:'When Frame Trial enters, draw a card.',power:'2',toughness:'3'}},text:{'card.name':'Frame Trial','card.text':'When Frame Trial enters, draw a card.\n','card.pt':'2/3'}},
  {id:'battle',style:'magic-m15-mainframe-battles.mse-style',dimensions:[523,375],context:{card:{name:'Rendering Trial',casting_cost:'2U',super_type:'Battle',sub_type:'Siege',rule_text:'When Rendering Trial enters, draw two cards.\n<sym>T</sym>: Add <sym>U</sym>.',loyalty:'4'}},text:{'card.name':'Rendering Trial','card.loyalty':'4','card.text':'When Rendering Trial enters, draw two cards.\nT: Add U.\n'}},
  {id:'saga',style:'magic-m15-saga.mse-style',dimensions:[375,523],context:{card:{name:'A Chronicle of Tests',casting_cost:'2U',super_type:'Enchantment',sub_type:'Saga',level_1_text:'Draw a card.',level_2_text:'Scry 2.',level_3_text:'Draw two cards.'}},text:{'card.name':'A Chronicle of Tests','card.level_1_text':'Draw a card.','card.level_2_text':'Scry 2.','card.level_3_text':'Draw two cards.'}},
  {id:'planeswalker',style:'magic-m15-planeswalker.mse-style',dimensions:[375,523],context:{card:{name:'Adept of Verification',casting_cost:'2UU',super_type:'Legendary Planeswalker',sub_type:'Adept',rule_text:'Draw a card.\nScry 2.\nDraw three cards.',loyalty:'4',loyalty_cost_1:'+1',loyalty_cost_2:'0',loyalty_cost_3:'-3'}},text:{'card.name':'Adept of Verification','card.loyalty':'4','card.text':'Draw a card.\nScry 2.\nDraw three cards.\n'}},
  {id:'split',style:'magic-m15-split-fusable.mse-style',dimensions:[523,375],context:{card:{name:'Observe',casting_cost:'U',super_type:'Instant',rule_text:'Draw a card.',name_2:'Recall',casting_cost_2:'2U',super_type_2:'Sorcery',rule_text_2:'Draw two cards.'}},text:{'card.name':'Observe','card.name_2':'Recall','card.text':'Draw a card.\n','card.text_2':'Draw two cards.\n'}},
];
