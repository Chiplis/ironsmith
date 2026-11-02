import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic} from '../src/lib/custom-mechanics.js';
import {bindLinkedEntryExileReference} from '../src/lib/linked-entry-exile-references.js';
const source={keyword:'Phantasmic',reminder:'As an additional cost to cast this spell, you may exile a creature you control until this permanent leaves the battlefield.',linkedExileVerb:'phantasmed',linkedExileScope:'source'};
test('persistent declared source nouns retain source linkage rather than entry history',()=>{
 const definition=validateMechanic(source);
 assert.equal(definition.linkedExileScope,'source');
 assert.equal(bindLinkedEntryExileReference("At the beginning of your upkeep, create a token that's a copy of the phantasmed card.",definition,'Independent Issuer'),"At the beginning of your upkeep, create a token that's a copy of the card exiled with this permanent.");
 assert.equal(bindLinkedEntryExileReference('Return the card Independent Issuer phantasmed to its owner\'s hand.',definition,'Independent Issuer'),'Return the card exiled with this permanent to its owner\'s hand.');
 assert.equal(bindLinkedEntryExileReference('It gains "Sacrifice the phantasmed card."',definition,'Independent Issuer'),'It gains "Sacrifice the phantasmed card."');
});
test('entry domains stay separate and unknown reference domains fail closed',()=>{
 const entry=validateMechanic({...source,linkedExileScope:'entry'});
 assert.equal(bindLinkedEntryExileReference('If it phantasmed a creature card, draw a card.',entry,'Independent Issuer'),'If a creature card was exiled as this creature entered by its ability with designation "phantasmic", draw a card.');
 assert.throws(()=>validateMechanic({...source,linkedExileScope:'all-exiled-cards'}));
});
