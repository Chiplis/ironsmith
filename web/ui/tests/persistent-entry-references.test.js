import test from 'node:test';
import assert from 'node:assert/strict';
import {bindLinkedEntryExileReference as bind} from '../src/lib/linked-entry-exile-references.js';
const definition={linkedExileVerb:'consumed'};
const noun='the card exiled as this creature entered';
test('persistent entry references preserve exact declared action and quote boundaries',()=>{
 assert.equal(bind('Spells you cast that share a type with the card Test Source consumed cost {1} less to cast.',definition,'Test Source'),`Spells you cast that share a type with ${noun} cost {1} less to cast.`);
 assert.equal(bind('The card it consumed. The card it devoured. A token named "the card it consumed".',definition,'Test Source'),`The card exiled as this creature entered. The card it devoured. A token named "the card it consumed".`);
 assert.equal(bind('The card it consumed.',{},'Test Source'),'The card it consumed.');
});
test('return and cast alternatives cannot become sequential moves',()=>{
 const input='When Test Source dies, you may put the card it consumed into your hand. If that card is an instant or sorcery card, you may instead cast it without paying its mana cost.';
 const output=bind(input,definition,'Test Source');
 assert.equal(output,`When Test Source dies, you may choose one instruction — Put ${noun} into your hand | If there is an instant or sorcery card exiled as this creature entered, cast ${noun} without paying its mana cost.`);
 assert.equal(bind('When Test Source dies, you may put the card it consumed into your hand if it’s a creature or land card.',definition,'Test Source'),`When Test Source dies, if there is a creature or land card exiled as this creature entered, you may put ${noun} into your hand.`);
});
test('a paired creature token stat clause binds both quantities to the same entry card',()=>{
 assert.equal(bind("Create a black Zombie creature token with power equal to the consumed card's power and toughness equal to that card's toughness.",definition,'Test Source'),`Create a black Zombie creature token with power equal to the power of ${noun} and toughness equal to the toughness of ${noun}.`);
 assert.equal(bind(`Create a token named "the consumed card's power".`,definition,'Test Source'),`Create a token named "the consumed card's power".`);
});
