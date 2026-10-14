import test from 'node:test';
import assert from 'node:assert/strict';
import {selectMsePresentation, selectMseStyle} from '../src/lib/mse/style-selection.mjs';
import {printingMseCardFields,printingMseFooterFields,printingMseSetFields} from '../src/lib/mse/printing-style.mjs';

const ordinary = {name:'Live card',type_line:'Creature — Wizard',layout:'normal'};
const main = 'magic-m15-altered.mse-style';

test('the same set can select regular, showcase, borderless and extended-art frames',()=>{
  const card={...ordinary,set:'dmu',frame:'2015'};
  assert.equal(selectMseStyle(card),main);
  assert.equal(selectMseStyle({...card,frame_effects:['showcase']}),'magic-m15-showcase-dominaria-stained-glass.mse-style');
  assert.deepEqual(selectMsePresentation({...card,border_color:'borderless'}).styling,{frames:'borderless art'});
  assert.deepEqual(selectMsePresentation({...card,frame_effects:['extendedart']}).styling,{frames:'frameless art'});
});

test('printing frame metadata identifies retro reprints independently of release date',()=>{
  assert.equal(selectMseStyle({...ordinary,set:'mh2',released_at:'2021-06-18',frame:'1997'}),'magic-old.mse-style');
  assert.equal(selectMseStyle({...ordinary,set:'lea',frame:'1993'}),'magic-old-abu.mse-style');
  assert.equal(selectMseStyle({...ordinary,set:'8ed',frame:'2003'}),'magic-new.mse-style');
  assert.equal(selectMseStyle({...ordinary,set:'fut',frame:'future'}),'magic-future.mse-style');
  assert.equal(selectMseStyle({...ordinary,set:'fut',frame:'2003'}),'magic-new.mse-style');
});

test('resolved image metadata overrides embedded printing without replacing live card text',()=>{
  const card={...ordinary,frame:'2003',printing:{set:'khm',frame:'2015',frame_effects:['showcase']}};
  assert.equal(selectMseStyle(card),'magic-m15-showcase-kaldheim.mse-style');
  const copy=structuredClone(card);
  assert.equal(selectMseStyle(card,{printing:{set:'dmu',frame:'1997',frame_effects:[]}}),'magic-old.mse-style');
  assert.equal(selectMseStyle(card,{printing:{set:'dmu',frame:'2015'}}),main);
  assert.deepEqual(card,copy);
});

test('border colors are preserved without passing style names into the native color parser',()=>{
  assert.deepEqual(printingMseCardFields({...ordinary,border_color:'white'}),{border_color:'white'});
  assert.deepEqual(printingMseCardFields({...ordinary,border_color:'white'},{border_color:'black'}),{border_color:'black'});
  assert.deepEqual(printingMseCardFields({...ordinary,border_color:'borderless'}),{});
  assert.deepEqual(printingMseCardFields({...ordinary,border_color:'white'},{}),{});
});

test('absent and ambiguous treatments fall back to a general layout without carrying styling',()=>{
  for(const metadata of [
    {set:'unknown',frame_effects:['showcase']},
    {set:'snc',frame_effects:['showcase']}, // Multiple Capenna treatments.
    {set:'sta',lang:'ja',frame_effects:['showcase']},
    {set:'unknown',full_art:true},
  ])assert.deepEqual(selectMsePresentation({...ordinary,...metadata}),{style:main,styling:{},source:'general'});
  assert.deepEqual(selectMsePresentation({...ordinary,set:'khm',frame_effects:['showcase']},{availableStyles:[main]}),
    {style:main,styling:{},source:'general'});
  assert.equal(selectMseStyle(ordinary,{availableStyles:[]}),null);
});

test('an installed showcase is chosen, while explicit custom packages bypass automatic availability filtering',()=>{
  const card={...ordinary,set:'khm',frame_effects:['showcase']};
  const viking='magic-m15-showcase-kaldheim.mse-style';
  assert.equal(selectMseStyle(card,{availableStyles:[viking,main]}),viking);
  assert.deepEqual(selectMsePresentation(card,{style:'custom.mse-style',defaultStyle:'other.mse-style',availableStyles:[]}),
    {style:'custom.mse-style',styling:{},source:'explicit'});
  assert.equal(selectMseStyle(card,{defaultStyle:'custom.mse-style',availableStyles:[]}), 'custom.mse-style');
});

test('printing treatments cannot flatten multipart cards or override the selected face layout',()=>{
  const metadata={set:'khm',frame:'1997',frame_effects:['showcase'],border_color:'borderless'};
  for(const [layout,type,expected] of [
    ['adventure','Creature','magic-m15-adventure'],
    ['split','Instant','magic-m15-split-fusable'],
    ['normal','Battle — Siege','magic-m15-mainframe-battles'],
    ['normal','Enchantment — Saga','magic-m15-saga'],
    ['normal','Planeswalker','magic-m15-planeswalker'],
  ])assert.equal(selectMseStyle({...ordinary,...metadata,layout,type_line:type}),`${expected}.mse-style`);
  assert.equal(selectMseStyle({...metadata,layout:'transform',card_faces:[{type_line:'Creature'},{type_line:'Planeswalker'}]},
    {faceIndex:1}),'magic-m15-mainframe-planeswalker.mse-style');
});

test('dedicated masterpiece sets select their known package without treating normal set cards as masterpieces',()=>{
  for(const [set,expected] of [['mps','magic-m15-Kaladesh'],['mp2','magic-m15-invocation'],['exp','magic-m15-expedition']])
    assert.equal(selectMseStyle({...ordinary,set}),`${expected}.mse-style`);
  assert.equal(selectMseStyle({...ordinary,set:'kld'}),main);
});

test('the footer credits the displayed printing in its era\'s number format',()=>{
  const modern={artist:'Steve Ditko',rarity:'mythic',collector_number:'9',set:'mar',lang:'ja',frame:'2015',released_at:'2025-09-26'};
  assert.deepEqual(printingMseFooterFields({rarity:'common'},modern),{illustrator:'Steve Ditko',rarity:'mythic rare',custom_card_number:'0009'});
  assert.deepEqual(printingMseSetFields({},modern),{automatic_card_numbers:'no',card_number_style:'0001',set_code:'MAR',set_language:'JP',copyright:'™ & © 2025 Wizards of the Coast'});
  const m15={artist:'Titus Lunter',rarity:'rare',collector_number:'259',set:'mh2',lang:'en',frame:'2015',released_at:'2021-06-18'};
  assert.equal(printingMseFooterFields({},m15).custom_card_number,'259');
  assert.equal(printingMseSetFields({},m15).card_number_style,'001');
  assert.equal(printingMseFooterFields({},{collector_number:'12a',frame:'2015',released_at:'2020-01-01'}).custom_card_number,'012a');
  assert.equal(printingMseFooterFields({},{collector_number:'213',frame:'2003'}).custom_card_number,'213');
  // Without resolved metadata the snapshot's own printing fields are used.
  assert.deepEqual(printingMseFooterFields({artist:'Kev Walker',rarity:'uncommon'},null),{illustrator:'Kev Walker',rarity:'uncommon'});
  assert.deepEqual(printingMseSetFields({lang:'xx'},null),{});
});

test('the copyright line follows the printing\'s release year and era',()=>{
  assert.equal(printingMseSetFields({},{released_at:'2021-06-18'}).copyright,'™ & © 2021 Wizards of the Coast');
  assert.equal(printingMseSetFields({},{released_at:'2005-10-07'}).copyright,'™ & © 1993-2005 Wizards of the Coast, Inc.');
  assert.equal(printingMseSetFields({},{released_at:''}).copyright,undefined);
});
