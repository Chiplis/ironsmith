import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
import {materializeCard} from '../../../scripts/custom-mechanics/cohort-input.mjs';
const cohort=JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/msem-late-grant-recipes.json',import.meta.url),'utf8'));
const row=id=>cohort.mechanics.find(row=>row.id===id);
const expand=(id,name)=>{const mechanic=row(id);const card=mechanic.representative_cards.find(card=>!name||card.name.includes(name));return materializeCard(card,[validateMechanic({keyword:mechanic.keyword,reminder:mechanic.executable_reminder})]).find(face=>face.name===card.name);};
test('actual Agonize grant installs recipient-relative graveyard permission',()=>{
 const face=expand('msem-763');assert.match(face.executable_text,/gains custom keyword "agonize", and "You may cast this card from your graveyard/);assert.match(face.executable_text,/until end of turn/);assert.deepEqual(face.custom_keyword_identities,[]);
});
test('actual Showcase own-mana-cost plus two recipe remains on granted recipient',()=>{
 const face=expand('msem-800');assert.match(face.executable_text,/this card's mana cost, \{2\}/);assert.match(face.executable_text,/have custom keyword "showcase"/);assert.doesNotMatch(face.executable_text,/text-adding cost is named showcase/);assert.deepEqual(face.custom_keyword_identities,[]);
});
test('actual static and delayed Rerun grants retain complete nested resolution program',()=>{
 for(const id of ['msem-802','msem-803']){const face=expand(id);assert.match(face.executable_text,/custom keyword "rerun"/);assert.match(face.executable_text,/If this spell was cast from your hand/);assert.match(face.executable_text,/'Whenever a creature you control dies,/);assert.deepEqual(face.custom_keyword_identities,[]);}
});
test('actual first-spell Compose grant queries recipient identity and retains Saga program',()=>{
 const face=expand('msem-841','Paul');assert.match(face.executable_text,/without custom keyword "compose"/);assert.match(face.executable_text,/gains custom keyword "compose"/);assert.match(face.executable_text,/'III — Copy the exiled card/);assert.deepEqual(face.custom_keyword_identities,[]);
});
test('actual Arise grant binds printed mana cost independently of grantor',()=>{
 const face=expand('msem-843');assert.match(face.executable_text,/Pay this card's mana cost, Exile this card from your graveyard/);assert.match(face.executable_text,/until end of turn/);assert.deepEqual(face.custom_keyword_identities,[]);
});
test('spell resolution keyword is not substituted into unrelated token homonym',()=>{
 const mechanic=row('msem-841');const def=validateMechanic({keyword:mechanic.keyword,reminder:mechanic.executable_reminder});const face=compileDraftInput({faces:[{name:'Token homonym',oracleText:'Create a Function token with compose.'}]},[def]).faces[0];assert.equal(face.oracleText,'Create a Function token with compose.');
});

test('Digital Encores retains its ordinary Compose independently of excluded Function payload',()=>{
 const mechanic=row('msem-841');
 const card=row('msem-842').representative_cards[0];
 const def=validateMechanic({keyword:mechanic.keyword,reminder:mechanic.executable_reminder});
 const face=materializeCard(card,[def]).find(face=>face.name===card.name);
 assert.match(face.executable_text,/create a Function token with [“"]Compose\.[”"]/);
 assert.match(face.executable_text,/If this spell was cast from your hand, instead of putting it into your graveyard as it resolves/);
 assert.deepEqual(face.custom_keyword_identities,['compose']);
});
