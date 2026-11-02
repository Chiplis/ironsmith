import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {materializeCard,materializeSourceLevelLayout} from '../../../scripts/custom-mechanics/cohort-input.mjs';
import {validateMechanic} from '../src/lib/custom-mechanics.js';
const reminder='This permanent enters with a lore counter on it. As your precombat main phase begins, put a lore counter on this permanent as a turn-based action, unless it has at least as many lore counters as its final chapter number.';
test('printed lore symbols transport chapter lists independently of keyword identity',()=>{
 const result=materializeSourceLevelLayout(['Type: Enchantment'],['[Lore I-II] Draw a card.','[Lore III] You gain 2 life.']);
 assert.deepEqual(result.lines,['I, II — Draw a card.','III — You gain 2 life.']);
 assert.throws(()=>materializeSourceLevelLayout([],['[Lore III-I] Draw a card.']),/ascending Roman/);
});
test('actual Chronicle source preserves identity and contains no Saga type or clock trigger',()=>{
 const fixture=JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/msem-chronicle.json',import.meta.url),'utf8'));
 const card=fixture.mechanics[0].representative_cards.find(card=>card.source_card_key==='Astral Capricine_TTR');const original=JSON.stringify(card.source_faces);
 const [face]=materializeCard(card,[validateMechanic({keyword:'Chronicle',reminder})],'msem');
 assert.deepEqual(face.custom_keyword_identities,['chronicle']);assert.equal(JSON.stringify(card.source_faces),original);
 assert.match(face.executable_text,/Type: Enchantment Creature — Goat/);assert.doesNotMatch(face.executable_text,/— Saga/);assert.match(face.executable_text,/turn-based action/);assert.doesNotMatch(face.executable_text,/At the beginning of your precombat/);assert.match(face.executable_text,/I, II — Put/);assert.match(face.executable_text,/III — Remove all lore counters/);
});
