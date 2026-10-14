import {engineCardToMse} from '../src/lib/mse/engine-card.mjs';
import {selectMseStyle} from '../src/lib/mse/style-selection.mjs';
const doubleFaces=['transform','modal_dfc'].flatMap(layout=>{
  const snapshot={id:`trial-${layout}`,layout,card_faces:[
    {name:'Day Trial',mana_cost:'{1}{U}',type_line:'Creature — Wizard',oracle_text:'Draw a card.',power:'2',toughness:'3'},
    {name:'Night Trial',type_line:'Legendary Planeswalker — Trial',oracle_text:'+1: Draw a card.\n-2: Scry 2.',loyalty:'4'},
  ]};
  return [0,1].map(faceIndex=>({id:`engine-${layout}-${faceIndex}`,snapshot,faceIndex,style:selectMseStyle(snapshot,{faceIndex}),dimensions:[375,523],text:{'card.name':snapshot.card_faces[faceIndex].name}}));
});
const alignmentCases=[
  ['short','Draw a card.'],
  ['wrapped','Whenever this creature attacks, look at the top three cards of your library. You may reveal a creature card from among them and put it into your hand. Put the rest on the bottom of your library in any order.'],
].map(([id,rules])=>({id:`alignment-${id}`,style:'magic-m15-bigtext.mse-style',dimensions:[375,523],
  context:{...engineCardToMse({name:'Alignment Trial',mana_cost:'{2}{U}',type_line:'Instant',oracle_text:rules}),styling:{center_text:'short text only'}},
  text:{'card.name':'Alignment Trial','card.text':rules+'\n'},
  // Pinned native output: reports/mse/scripted-alignment-native/reference.json.
  layout:{'card.text':{tolerance:0.001,alignment:id==='short'?'middle center':'middle left',
    geometry:{content_width:id==='short'?78:280.25,content_height:id==='short'?112.312:142.312},
    lines:id==='short'?[[96.0625,16.25,78],[121.688,0,0]]:
      [[66.0625,16.25,279],[81.0625,16.25,266],[96.0625,16.25,280.25],[111.062,16.25,273.25],[126.062,16.25,119.25],[151.688,0,0]],
  }},
}));
const frameFamilies=[
  ['urban','magic-urban',375,523],['bigtext','magic-m15-bigtext',375,523],['comicbookguy','magic-comicbookguy',375,523],['old','magic-old',375,523],['new','magic-new',375,523],['future','magic-future',375,523],
  ['kaldheim','magic-m15-showcase-kaldheim',750,1046],['invocation','magic-m15-invocation',375,523],
].map(([id,style,width,height])=>({
  id:`family-${id}`,style:`${style}.mse-style`,dimensions:[width,height],
  context:engineCardToMse({name:'Archive Scholar',mana_cost:'{2}{U}{B}',type_line:'Legendary Creature — Human Wizard',
    oracle_text:'Flying\nWhenever this creature attacks, draw a card, then discard a card.\n{1}{U}, {T}: Scry 2.',
    flavor_text:'Every frame preserves a different age.',rarity:'rare',power:'3',toughness:'4'}),
  text:{'card.name':'Archive Scholar',...(id==='urban'?{'extra_card.urban_type':'Legendary Creature - Human Wizard'}:{'card.type':'Legendary Creature — Human Wizard'}),
    'card.text':'Flying\nWhenever this creature attacks, draw a card, then discard a card.\n1U, T: Scry 2.\nEvery frame preserves a different age.',
    ...(id==='invocation'?{'card.power':'3','card.toughness':'4'}:{'card.pt':'3/4'})},
}));
// Original diagnostic cards: exercise layout families without external card art.
// Dense layout expectations below were queried from the pinned native macOS
// executable at 72 DPI; they are not applicable to the 96-DPI font profile.
export default [
  {"id": "self-sizing-intense", "style": "magic-intense-horizontal.mse-style", "dimensions": [1046, 750], "context": {"card": {"name": "Archive Scholar", "casting_cost": "", "super_type": "Legendary Creature", "sub_type": "Human Wizard", "rule_text": "Flying\nWhenever this creature attacks, draw a card, then discard a card.", "flavor_text": "Every frame preserves a different age.", "power": "3", "toughness": "4", "rarity": "rare"}, "set": {}, "styling": {}, "fieldUpdateMode": "ordered"}, "text": {"card.name": "Archive Scholar", "card.rule_text": "Flying\nWhenever this creature attacks, draw a card, then discard a card."}, "layout": {"card.rule_text": {"tolerance": 0.001, "geometry": {"top":650,"height":25,"content_height":10}, "lines": [[-34.25, 22.25, 47], [-12.25, 22.25, 529.75]]}}},
  {"id": "dense-rules", "style": "magic-m15-altered.mse-style", "dimensions": [375, 523], "context": {"card": {"name": "Keeper of Lengthy Records", "casting_cost": "3UU", "super_type": "Creature", "sub_type": "Wizard", "rule_text": "Whenever this creature attacks, look at the top three cards of your library. You may reveal a creature card from among them and put it into your hand. Put the rest on the bottom of your library in any order.\n<sym>2</sym><sym>U</sym>, <sym>T</sym>: Return another target creature you control to its owner’s hand. Activate only as a sorcery.\nAt the beginning of your end step, if you drew two or more cards this turn, create a 1/1 blue Bird creature token with flying.", "power": "3", "toughness": "5"}, "fieldUpdateMode": "ordered"}, "text": {"card.name": "Keeper of Lengthy Records"}, "layout": {"card.text": {"tolerance": 0.001, "lines": [[6.75, 15, 290.75], [20.75, 15, 292], [34.75, 15, 291.25], [48.75, 15, 259.75], [68.25, 16.4267, 303.654], [83.25, 15, 257.25], [102.75, 15, 294.25], [116.75, 15, 288], [130.75, 15, 97.75], [155.25, 0, 0]]}}},
  ...alignmentCases,
  ...doubleFaces,
  ...frameFamilies,
  {id:'long-name',style:'magic-future.mse-style',dimensions:[375,523],context:engineCardToMse({name:'The Extraordinary Archivist of Forgotten Civilizations',mana_cost:'{2}{U}{B}',type_line:'Legendary Creature — Human Wizard',oracle_text:'Draw a card.',power:'3',toughness:'4'}),text:{'card.name':'The Extraordinary Archivist of Forgotten Civilizations','card.pt':'3/4'}},
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
