import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic, compileDraftInput} from '../src/lib/custom-mechanics.js';
const def=validateMechanic({keyword:'Upgrade {Object}',reminder:"Choose a kind of counter that {Object} doesn't have on it from among {CounterKinds}. Put a counter of that kind on {Object}."});
test('explicit performer owns recipient and missing-counter choices while you keeps source controller',()=>{
 const original="Enchant player\nAt the beginning of enchanted player's upkeep, that player upgrades a creature you control.\n• +1/+1\n• Lifelink\n• Menace";
 const face=compileDraftInput({faces:[{name:'Curse of Rebalancing',oracleText:original}]},[def]).faces[0];
 assert.match(face.oracleText,/that player chooses a creature you control\. That player chooses a kind of counter/i);
 assert.match(face.oracleText,/that that creature doesn't have on it from among \+1\/\+1, lifelink and menace/i);
 assert.equal(face.displayOracleText,original);
});
