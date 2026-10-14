import test from 'node:test';
import assert from 'node:assert/strict';
import {cardFrameSnapshot,cardFrameFaceIndex,cardFrameRules} from '../src/lib/card-frame-snapshot.mjs';

test('presentation snapshots retain printing and live fields without serializing unrelated game state',()=>{
  const card={id:1,name:'Live',power:5,toughness:4,frame:'1997',mse_style:'custom.mse-style',
    printing:{set:'mh2'},compiled_text:['First','Second']};
  card.game=card;
  const parsed=JSON.parse(cardFrameSnapshot(card));
  assert.equal(parsed.power,5);assert.equal(parsed.frame,'1997');assert.equal(parsed.mse_style,'custom.mse-style');
  assert.deepEqual(parsed.printing,{set:'mh2'});assert.equal(parsed.game,undefined);
  assert.equal(cardFrameRules(parsed),'First\nSecond');
  assert.equal(cardFrameRules({...parsed,oracle_text:''}),'');
});

test('face selection follows the displayed back image or live name without splitting adventure layouts',()=>{
  const card={name:'Back',layout:'transform',card_faces:[{name:'Front'},{name:'Back'}]};
  assert.equal(cardFrameFaceIndex(card),1);
  assert.equal(cardFrameFaceIndex({...card,name:'Front'},'https://cards.scryfall.io/normal/back/a/b/id.jpg'),1);
  assert.equal(cardFrameFaceIndex({...card,layout:'adventure'}),0);
});

test('presentation snapshots keep a land\'s live intrinsic mana for its frame color',()=>{
  const snapshot=JSON.parse(cardFrameSnapshot({name:'Yavimaya, Cradle of Growth',type_line:'Legendary Land — Forest',intrinsic_mana:['G'],oracle_text:'Each land is a Forest in addition to its other land types.'}));
  assert.deepEqual(snapshot.intrinsic_mana,['G']);
  assert.equal(JSON.parse(cardFrameSnapshot({name:'Wastes',type_line:'Basic Land',intrinsic_mana:[]})).intrinsic_mana,undefined);
});
