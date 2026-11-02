import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
import {normalizeDeclaredManaPrice,splitReminderPriceDeclaration} from '../src/lib/reminder-costs.js';
const definition=validateMechanic({keyword:'Reconstitute',reminder:'{Cost}, Exile this card from your graveyard: Create a token that\'s a copy of this card. Activate only as a sorcery.'});
test('separate ability cost rider remains outside the declared activation price',()=>{
 const face=compileDraftInput({faces:[{name:'Example',oracleText:'Reconstitute {8}{B}{B}. This ability costs {1} less to activate for each other card in your graveyard.'}]},[definition]).faces[0];
 assert.equal(face.oracleText,"{8}{B}{B}, Exile this card from your graveyard: Create a token that's a copy of this card. Activate only as a sorcery. This ability costs {1} less to activate for each other card in your graveyard.");
});
test('compact mana is normalized only inside an explicit price slot',()=>{
 const face=compileDraftInput({faces:[{name:'Example',oracleText:'Reconstitute 2R'}]},[definition]).faces[0];
 assert.ok(face.oracleText.startsWith('{2}{R}, Exile'));
 assert.equal(normalizeDeclaredManaPrice('2 life'),'2 life');
 assert.equal(normalizeDeclaredManaPrice('2R, Pay 3 life'),'{2}{R}, Pay 3 life');
 assert.equal(normalizeDeclaredManaPrice('Discard a card'),'Discard a card');
 assert.equal(normalizeDeclaredManaPrice('2R unless you attacked this turn'),'2R unless you attacked this turn');
});
test('quoted names own their punctuation before a following ability rider',()=>{
 const value='Discard a card named "Dr. This Ability". This ability costs {1} less to activate.';
 assert.deepEqual(splitReminderPriceDeclaration(value,definition),{price:'Discard a card named "Dr. This Ability"',following:'This ability costs {1} less to activate.'});
});
