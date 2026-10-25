import test from 'node:test';
import assert from 'node:assert/strict';
import {cardGlintFace} from '../src/lib/card-glint/card.mjs';
import {cardFrameSnapshot} from '../src/lib/card-frame-snapshot.mjs';
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
test('Build drafts carry their printing style, border and rarity to the painter', async () => {
  const {cardGlintDraftFace,draftTypeLine} = await import('../src/lib/card-glint/draft.mjs');
  const face = {name:'Forge Adept',manaCost:'{1}{R}',colorIndicator:[],supertypes:['Legendary'],cardTypes:['Creature'],
    subtypes:['Human','Wizard'],oracleText:'Haste',artUrl:'https://example.test/art.webp',power:'2',toughness:'1',loyalty:'',defense:''};
  assert.equal(draftTypeLine(face),'Legendary Creature — Human Wizard');
  const painted = cardGlintDraftFace({layout:'single',faces:[face],
    presentation:{finish:'cold',printEffects:false,style:'fut',frame:'futureshifted',borderColor:'white',rarity:'mythic'}});
  assert.deepEqual(painted.colors,['R']); assert.deepEqual(painted.manaSymbols,['1','R']);
  assert.equal(painted.illustration,'https://example.test/art.webp');
  assert.equal(painted.rarity,'mythic'); assert.equal(painted.legendary,true);
  assert.deepEqual(painted.printingChoice,{style:'fut',frame:'futureshifted',borderColor:'white'});
  // "Current printing" keeps the card's own set, art and frame.
  const current = cardGlintDraftFace({layout:'single',faces:[{...face,artUrl:''}]},0,
    {printing:{set:'m21',rarity:'uncommon',frame:'2015',artist:'Printed Artist'},imageUrl:'https://cards.scryfall.io/normal/front/a/b/id.jpg'});
  assert.equal(current.printingChoice,undefined); assert.equal(current.setCode,'m21'); assert.equal(current.rarity,'uncommon');
  assert.equal(current.illustration,'https://cards.scryfall.io/art_crop/front/a/b/id.jpg');
  const split = cardGlintDraftFace({layout:'split',faces:[face,{...face,name:'Other Half'}],presentation:{style:'8ed'}});
  assert.deepEqual(split.printingChoice,{style:'8ed'});
  assert.deepEqual(split.faces.map(part => [part.name,part.printingChoice]),[['Forge Adept',undefined],['Other Half',undefined]]);
});
test('printing choices default to shiny with print effects and reject unknown values', async () => {
  const {normalizeCardPresentation,isDefaultCardPresentation} = await import('../src/lib/card-glint/printing-choices.mjs');
  assert.deepEqual(normalizeCardPresentation(undefined),{finish:'gloss',printEffects:true,style:'match',frame:null,borderColor:null,rarity:null});
  assert.deepEqual(normalizeCardPresentation({finish:'chrome',style:'Bad Style!',frame:'x',borderColor:'gold',rarity:'legendary',printEffects:'yes'}),
    normalizeCardPresentation());
  assert.equal(isDefaultCardPresentation({finish:'gloss'}),true);
  assert.equal(isDefaultCardPresentation({printEffects:false}),false);
});
test('in-game faces use printing choices only for the copy added from Build', async () => {
  const {setCardPresentation,clearCardPresentations,snapshotStableIds} = await import('../src/lib/card-glint/printing-choices.mjs');
  const choice = {finish:'opal',style:'4ed',borderColor:'white',rarity:'uncommon'};
  setCardPresentation(41,['Lightning Bolt'],choice);
  const bolt = stable => cardGlintFace({name:'Lightning Bolt',stable_id:stable,type_line:'Instant',mana_cost:'{R}'},{printing:{frame:'2015'}});
  assert.deepEqual(bolt(41).printingChoice,{style:'4ed',borderColor:'white'}); assert.equal(bolt(41).rarity,'uncommon');
  assert.equal(bolt(42).printingChoice,undefined,'other copies keep their own printing');
  // A rewound or reused ID on another card never inherits the choice.
  assert.equal(cardGlintFace({name:'Shock',stable_id:41},{printing:{frame:'2015'}}).printingChoice,undefined);
  clearCardPresentations();
  assert.equal(bolt(41).printingChoice,undefined,'a new match forgets the choice');
  // Additions waiting on a decision bind to the first new copy with that name.
  const existing = snapshotStableIds({players:[{hand_cards:[{stable_id:7,name:'Lightning Bolt'}]}]});
  setCardPresentation(0,['Lightning Bolt'],choice,{existing});
  assert.equal(bolt(7).printingChoice,undefined,'an existing copy is not bound');
  assert.deepEqual(bolt(90).printingChoice,{style:'4ed',borderColor:'white'});
  assert.equal(bolt(91).printingChoice,undefined,'only one copy is bound');
  clearCardPresentations();
});
test('painted faces opt into automatic calibration when the renderer supports it', async () => {
  const {presentCardGlintFace} = await import('../src/lib/card-glint/printing-choices.mjs');
  const calls = [];
  const api = {printingCalibrationApiVersion:1,printingStyles:[{id:'match'},{id:'4ed'}],
    resolvePrintingStyle:(source,style,choice) => {calls.push([style.id,choice]); return {face:{...source,calibration:choice.overrides?.calibration}};}};
  assert.deepEqual(presentCardGlintFace(api,{name:'Plain'}),{name:'Plain',calibration:'auto'});
  assert.deepEqual(calls.at(-1),['match',{overrides:{calibration:'auto'}}],'unstyled cards resolve as their own printing');
  const styled = presentCardGlintFace(api,{name:'Styled',printingChoice:{style:'4ed',borderColor:'white'}});
  assert.deepEqual(calls.at(-1),['4ed',{overrides:{calibration:'auto',borderColor:'white'}}]);
  assert.equal(styled.calibration,'auto'); assert.equal('printingChoice' in styled,false);
  assert.deepEqual(presentCardGlintFace({...api,printingCalibrationApiVersion:undefined},{name:'Legacy'}),{name:'Legacy'},
    'older renderers keep legacy colors');
  assert.deepEqual(presentCardGlintFace({...api,resolvePrintingStyle:() => {throw Error('unsupported layout');}},{name:'Saga'}),{name:'Saga'},
    'unsupported layouts keep their legacy painting');
});

test('a new Build card with no real printing behind it still paints', async () => {
  const {cardGlintDraftFace} = await import('../src/lib/card-glint/draft.mjs');
  // CardPrintingStage passes null until a real card's printing resolves.
  const face = cardGlintDraftFace({layout:'single',faces:[{name:'Custom Card',cardTypes:['Creature'],power:'2',toughness:'2'}]},0,null);
  assert.equal(face.name,'Custom Card');
});

test('hand snapshots take supertypes and subtypes from their printing', () => {
  const ring = cardGlintFace({name:'The One Ring',card_types:['Artifact'],mana_cost:'{4}'},
    {printing:{type_line:'Legendary Artifact',set:'ltr'}});
  assert.equal(ring.typeLine,'Legendary Artifact');
  assert.equal(ring.legendary,true);
  const karn = cardGlintFace({name:'Karn, the Great Creator',card_types:['Planeswalker'],mana_cost:'{4}'},
    {printing:{type_line:'Legendary Planeswalker — Karn',set:'war'}});
  assert.equal(karn.typeLine,'Legendary Planeswalker — Karn');
  // An effect changed the card's types: the live line wins over the printing.
  const animated = cardGlintFace({name:'Mishra\'s Factory',card_types:['Artifact','Creature'],mana_cost:''},
    {printing:{type_line:'Land',set:'atq'}});
  assert.equal(animated.typeLine,'Artifact Creature');
});

test('loyalty abilities retain their layout when live types become creature',()=>{
  const card={name:'Animated walker',type_line:'Legendary Creature — Ninja',power:'3',toughness:'4',
    oracle_text:'Hexproof\n+1: Draw a card.\n0: Surveil 2.\n−2: Tap target creature.'};
  const face=cardGlintFace(card);
  assert.equal(face.layout,'planeswalker');
  assert.equal(face.typeLine,card.type_line);
  assert.equal(face.creature,true);
  assert.equal(face.power,'3');assert.equal(face.toughness,'4');
  assert.equal(face.rulesText,card.oracle_text);
  assert.equal(cardGlintFace({...card,oracle_text:'{0}: Draw a card.'}).layout,'normal');
  assert.equal(cardGlintFace({...card,oracle_text:'Pay 2 life: Draw a card.'}).layout,'normal');
  assert.equal(cardGlintFace({...card,layout:'adventure'}).layout,'adventure');
  assert.equal(cardGlintFace({...card,oracle_text:'[-X]: Draw X cards.'}).layout,'planeswalker');
});


test('basic land frames retain their palette before and after printing metadata arrives',()=>{
  for (const [subtype,color] of [['Plains','W'],['Island','U'],['Swamp','B'],['Mountain','R'],['Forest','G']]) {
    const type_line=`Basic Land — ${subtype}`;
    // Hand/zone snapshots may only acquire their subtype from the printing.
    const lightweight=JSON.parse(cardFrameSnapshot({name:subtype,card_types:['land'],colors:[]}));
    const printed=cardGlintFace(lightweight,{printing:{type_line}});
    assert.deepEqual(printed.frameColors,[color]);
    assert.deepEqual(printed.colors,[]);
    const full={...lightweight,type_line};
    assert.deepEqual(cardGlintFace(full).frameColors,[color]);
    assert.deepEqual(cardGlintFace({...full,color_identity:[]}).frameColors,[color]);
    assert.deepEqual(cardGlintFace({...lightweight,color_identity:[color]}).frameColors,[color]);
    assert.deepEqual(cardGlintFace(lightweight,{printing:{type_line:'Land',color_identity:[color]}}).frameColors,[color]);
  }
  assert.deepEqual(cardGlintFace({type_line:'Land',color_identity:[]},{printing:{color_identity:['G']}}).frameColors,[]);
  assert.deepEqual(cardGlintFace({type_line:'Basic Land',name:'Wastes'}).frameColors,[]);
  assert.deepEqual(cardGlintFace({type_line:'Land — Forest',intrinsic_mana:[]}).frameColors,[]);
});
