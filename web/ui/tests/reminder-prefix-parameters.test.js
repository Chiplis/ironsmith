import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic, compileDraftInput} from '../src/lib/custom-mechanics.js';
const definition = validateMechanic({keyword:'Cadence', prefixTemplate:'{Type}', reminder:"Whenever you cast an {Type} spell, if that spell's mana value is greater than this creature's power, put a +1/+1 counter on this creature."});
const expand = text => compileDraftInput({faces:[{name:'Probe',oracleText:text}]},[definition]).faces[0];
test('prepositive type list binds one authored trigger without losing its scope', () => {
  const face = expand('Instant, sorcery, and Dragon cadence');
  assert.equal(face.oracleText, "Whenever you cast an Instant, sorcery or Dragon spell, if that spell's mana value is greater than this creature's power, put a +1/+1 counter on this creature.");
  assert.deepEqual(face.customKeywordIdentities,['cadence']);
});
test('same reader handles another keyword and a singular operand', () => {
  const def = validateMechanic({keyword:'Resonate',prefixTemplate:'{Type}',reminder:'Whenever you cast a {Type} spell, draw a card.'});
  const face = compileDraftInput({faces:[{name:'Probe',oracleText:'Goblin resonate'}]},[def]).faces[0];
  assert.equal(face.oracleText,'Whenever you cast a Goblin spell, draw a card.');
});
test('quoted text and prose do not become declarations', () => {
  for (const text of ['This creature has cadence.', 'Whenever you cast cadence', '"Instant, sorcery, and Dragon cadence"', 'Instant, sorcery, and Dragon cadence deals damage.']) {
    assert.equal(expand(text).oracleText,text);
    assert.equal(expand(text).customKeywordIdentities,undefined);
  }
});
test('prefix definition must actually bind only a type', () => {
 assert.throws(()=>validateMechanic({keyword:'Resonate',prefixTemplate:'{Type}',reminder:'Draw a card.'}));
 assert.throws(()=>validateMechanic({keyword:'Resonate',prefixTemplate:'{Cost}',reminder:'Pay {Cost}.'}));
});
