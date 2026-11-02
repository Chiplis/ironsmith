import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {compileDraftInput,validateMechanic} from '../../web/ui/src/lib/custom-mechanics.js';
const fixture=JSON.parse(readFileSync(new URL('../../fixtures/custom_mechanics/msem-module.json',import.meta.url),'utf8'));
const definition=validateMechanic(fixture.mechanics[0].executable_definitions[0]);
test('all complete source submodule bars retain their activation costs and ordinary entry selection',()=>{
 const actors=fixture.mechanics[0].representative_cards.filter(card=>/^Type:.*[—–]\s*Module\s*$/mi.test(card.original_text));
 assert.equal(actors.length,15);
 for(const card of actors){
  assert.equal(card.source_faces[0].original_text,card.original_text);
  const face=card.executable_faces[0];const text=face.executable_text;
  assert.equal((text.match(/This activated ability has designation module/g)||[]).length,3,card.name);
  assert.equal((text.match(/Once group 0 step/g)||[]).length,3,card.name);
  assert.match(text,/When this permanent enters the battlefield, you may activate an activated ability with designation module of this permanent without paying its costs during module actions/);
  assert.equal((text.match(/Activate only as a sorcery or during module actions/g)||[]).length,3);
  assert.equal(face.custom_keyword_identities?.length||0,0);
 }
});
test('layout operands preserve nonmana costs, labels and top abilities without broad subtype matches',()=>{
 const text='Flash\n{T}, Sacrifice a creature: First\nTarget creature gets +2/+2 until end of turn.\nDiscard a card: Second\nDraw a card.\n{1}: Third\nYou gain 3 life.';
 const face=compileDraftInput({faces:[{name:'Control',typeLine:'Enchantment — Module',oracleText:text}]},[definition]).faces[0];
 assert.ok(face.oracleText.startsWith('Flash\n'));
 assert.match(face.oracleText,/— \{T\}, Sacrifice a creature: Target creature/);
 assert.match(face.oracleText,/— Discard a card: Draw a card/);
 const other=compileDraftInput({faces:[{name:'Control',typeLine:'Enchantment — Nonmodule',oracleText:text}]},[definition]).faces[0];assert.equal(other.oracleText,text);
 assert.throws(()=>compileDraftInput({faces:[{name:'Control',typeLine:'Enchantment — Module',oracleText:'{1}: First\nDraw a card.'}]},[definition]),/three|3/);
});
