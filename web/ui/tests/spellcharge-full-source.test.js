import test from 'node:test';import assert from 'node:assert/strict';import {readFileSync} from 'node:fs';
import {validateMechanic} from '../src/lib/custom-mechanics.js';import {expandNamedActionState} from '../src/lib/reminder-named-action-state.js';import {materializeCard} from '../../../scripts/custom-mechanics/cohort-input.mjs';
const fixture=JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/revolution-spellcharge-source-complete.json',import.meta.url),'utf8'));const row=fixture.mechanics[0];const cards=fixture.complete_sources;const def=validateMechanic(row.executable_definitions[0]);
test('all47actual Spellcharge sources preserve originals and consume complete declarations',()=>{
 assert.equal(cards.length,47);assert.equal(fixture.source_row_ids.length,19);assert.equal(fixture.mechanics.length,19);
 for(const card of cards){const before=structuredClone(card);const faces=materializeCard(card,[def],'revolution');assert.equal(faces.length,card.source_faces.length);assert.deepEqual(card,before);for(const face of faces){assert.doesNotMatch(face.executable_text,/^Spellcharge \d+/m);assert.doesNotMatch(face.executable_text,/becomes spellcharged/);}
  if(card.name==='Join the Forebears'){assert.doesNotMatch(faces[0].executable_text,/has spellcharge X/);assert.deepEqual(faces[0].custom_keyword_identities,[]);}
  else{assert.match(faces[0].executable_text,/if you spent at least \d+ mana to cast spells this turn/);assert.match(faces[0].executable_text,/have this permanent perform the spellcharge action/);assert.ok(faces[0].custom_keyword_identities.includes('spellcharge'));}
 }
});
test('passive completion aliases retain performer scope and ignore unrelated animations',()=>{
 assert.equal(expandNamedActionState('Whenever Watcher becomes spellcharged, untap it.',def,'Watcher'),'Whenever this permanent performs the spellcharge action, untap it.');
 assert.equal(expandNamedActionState('Whenever a Ruin you control becomes spellcharged, it gains haste until end of turn.',def,'Watcher'),'Whenever a Ruin you control performs the spellcharge action, it gains haste until end of turn.');
 for(const text of ['Whenever Watcher becomes a creature, draw a card.','Another Watcher becomes spellcharged.','This creature gains "Whenever Watcher becomes spellcharged, untap it."'])assert.equal(expandNamedActionState(text,def,'Watcher'),text);
});
