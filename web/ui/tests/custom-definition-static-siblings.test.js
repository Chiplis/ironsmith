import test from 'node:test';
import assert from 'node:assert/strict';
import { expandGrantedTriggeredPrograms } from '../src/lib/reminder-ability-grants.js';
import { independentAbilityPrograms } from '../src/lib/reminder-definition-programs.js';
import { validateMechanic, compileDraftInput } from '../src/lib/custom-mechanics.js';
const staticRule = 'This card is an Inscription in addition to its other types.';
const activation = '{Cost}: Draw a card. Activate only as a sorcery.';
const definition = () => validateMechanic({keyword:'Scriptmark {Cost}',reminder:`${activation} ${staticRule}`});
const compile = (text, def=definition()) => compileDraftInput({faces:[{name:'Grant witness',oracleText:text}]},[def]).faces[0].oracleText;
test('typed independent static suffix is separate from complete activation',()=>{
 assert.deepEqual(independentAbilityPrograms(`${activation} ${staticRule}`),[activation,staticRule]);
 assert.equal(compile('Scriptmark {2}.'),`{2}: Draw a card. Activate only as a sorcery.\n${staticRule}`);
});
test('fixed and source-derived activation grants quote subtype static separately',()=>{
 const fixed=compile('Each sorcery card in your hand has Scriptmark {2}.');
 assert.match(fixed,/"\{2\}: Draw a card\. Activate only as a sorcery\.", and "This card is an Inscription in addition to its other types\."/);
 const derived=compile("Each instant and sorcery card you own that isn't on the battlefield has Scriptmark. The Scriptmark cost is equal to its mana cost.");
 assert.match(derived,/"Pay this card's mana cost: Draw a card\. Activate only as a sorcery\.", and "This card is an Inscription in addition to its other types\."/);
});
test('numeric and unparameterized grants share independent static boundaries',()=>{
 const numeric=validateMechanic({keyword:'Countscript {Number}',reminder:`Whenever this creature attacks, you gain {Number} life. ${staticRule}`});
 assert.match(compile('Creatures you control have Countscript 2.',numeric),/"Whenever this creature attacks, you gain 2 life\.", and "This card is an Inscription in addition to its other types\."/);
 const bare=validateMechanic({keyword:'Scribeheart',reminder:`Whenever this creature attacks, draw a card. ${staticRule}`});
 assert.match(compile('Creatures you control have Scribeheart.',bare),/"Whenever this creature attacks, draw a card\.", and "This card is an Inscription in addition to its other types\."/);
});
test('quoted text and ordinary multi-sentence bodies remain opaque',()=>{
 const triggered='Whenever this creature attacks, draw a card. Then discard a card.';
 assert.deepEqual(independentAbilityPrograms(triggered),[triggered]);
 const quoted=`{2}: Target creature gains "Draw a card. ${staticRule}" until end of turn.`;
 assert.deepEqual(independentAbilityPrograms(quoted),[quoted]);
 assert.deepEqual(independentAbilityPrograms('{2}: Draw a card. This card deals 1 damage to each opponent.'),['{2}: Draw a card. This card deals 1 damage to each opponent.']);
});

test('independent static subtype operands retain Unicode, digits and punctuation',()=>{
 for(const name of ['Écho37', "Ing'Wei", 'Cloud Court', 'Cd-hr']) {
  const rule=`This card is an ${name} in addition to its other types.`;
  assert.deepEqual(independentAbilityPrograms(`${activation} ${rule}`),[activation,rule]);
 }
});

test('normalized independent siblings remain grantable while multiline ordinary programs stay unread',()=>{
 const bare=validateMechanic({keyword:'Scribeheart',reminder:`Whenever this creature attacks, draw a card. ${staticRule}`});
 assert.match(bare.expansion,/\nThis card is an Inscription/);
 const text=compile('Creatures you control have Scribeheart.',bare);
 assert.match(text, /"Whenever this creature attacks, draw a card\.", and "This card is an Inscription in addition to its other types\."/);
 const unsupported={...bare,expansion:'Whenever this creature attacks, draw a card.\nThen discard a card.'};
 assert.equal(expandGrantedTriggeredPrograms('Creatures you control have Scribeheart.',[unsupported]),'Creatures you control have Scribeheart.');
});
