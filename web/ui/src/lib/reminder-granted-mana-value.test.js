import test from 'node:test';
import assert from 'node:assert/strict';
import {expandGrantedManaValueAbilities} from './reminder-granted-parameters.js';
const definition = {keyword:'Echo weaving',template:'{Cost}',reminder:'As an additional cost to cast this spell, you may pay {Cost}. When you cast this spell, if you paid this additional cost, copy this spell and you may choose new targets for the copy.',expansion:'As an additional cost to cast this spell, you may pay {Cost}. When you cast this spell, if you paid this additional cost, copy this spell and you may choose new targets for the copy.'};
test('recipient mana value stays inside its granted optional price and cast trigger',()=>{
 const source='Each instant and sorcery card you own has Echo weaving {X}, where X is its mana value.';
 const result=expandGrantedManaValueAbilities(source,[definition]);
 assert.match(result,/Each instant and sorcery card you own has custom keyword "Echo weaving"/);
 assert.match(result,/pay mana equal to this spell's mana value/);
 assert.match(result,/When you cast this spell, if you paid this additional cost, copy this spell/);
 assert.match(result,/This optional additional cost is named Echo weaving/);
 assert.doesNotMatch(result,/\{X\}|where X/);
});
test('emblem outer rule keeps one quote boundary with nested recipient rules',()=>{
 const source='[-7]: You get an emblem with “Each instant and sorcery card you own has Echo weaving {X}, where X is its mana value.”';
 const result=expandGrantedManaValueAbilities(source,[definition]);
 assert.match(result,/emblem with “Each instant and sorcery card you own has custom keyword 'Echo weaving', and 'As an additional cost/);
 assert.equal((result.match(/[“”]/g)||[]).length,2);
});
test('quoted names and unrelated X meanings stay opaque',()=>{
 for(const source of ['Name a card "Each card has Echo weaving {X}, where X is its mana value."','Each card has Echo weaving {X}, where X is the number of cards in your hand.'])assert.equal(expandGrantedManaValueAbilities(source,[definition]),source);
});
