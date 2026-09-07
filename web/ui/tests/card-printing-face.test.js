import test from 'node:test';
import assert from 'node:assert/strict';
import {printingForImageFace} from '../src/lib/card-printing-face.js';
const id='12345678-1234-1234-1234-123456789abc';
const url=(side,size='normal')=>`https://cards.scryfall.io/${size}/${side}/1/2/${id}.jpg?456`;
const printing={frame:'2015',layout:'transform',name:'Front // Back',card_faces:[
  {name:'Front',mana_cost:'{U}',oracle_text:'Front rules',image_uris:{normal:url('front')}},
  {name:'Back',mana_cost:'',oracle_text:'Back rules',image_uris:{normal:url('back')}},
]};
test('selects face-specific rules, cost and name without changing the shared printing',()=>{
  const back=printingForImageFace(printing,url('back','art_crop').replace('?456','?789'));
  assert.equal(back.name,'Back');assert.equal(back.mana_cost,'');assert.equal(back.oracle_text,'Back rules');assert.equal(back.frame,'2015');
  assert.equal(printingForImageFace(printing,url('front')).name,'Front');
  assert.equal(printing.name,'Front // Back');
});
test('single-image split/adventure cards retain their complete printing metadata',()=>{
  const split={...printing,layout:'split',card_faces:printing.card_faces.map(({image_uris,...face})=>face)};
  assert.equal(printingForImageFace(split,url('front')),split);
  assert.equal(printingForImageFace(null,url('back')),null);
});
