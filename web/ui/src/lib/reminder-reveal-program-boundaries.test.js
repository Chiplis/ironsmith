import test from 'node:test';
import assert from 'node:assert/strict';
import {independentAbilityPrograms} from './reminder-definition-programs.js';
const offer='As you reveal this card from a hidden zone, you may perform the moon augury action once each turn ending in exile as follows: [Exile this card].';
const trigger='Whenever this card performs the moon augury action, if this card is in exile, you gain 1 life.';
test('explicit reveal offer closing bracket bounds its independent following trigger',()=>{
 assert.deepEqual(independentAbilityPrograms(`${offer} ${trigger}`),[offer,trigger]);
});
test('periods and delayed triggers inside the bounded child remain opaque',()=>{
 const nested='As you reveal this card from a hidden zone, you may perform the moon augury action once each turn ending in exile as follows: [Exile this card. At the beginning of the next end step, you gain 1 life.].';
 assert.deepEqual(independentAbilityPrograms(`${nested} ${trigger}`),[nested,trigger]);
});
test('an ordinary multi-sentence effect without an explicit bounded static stays one program',()=>{
 const ordinary='Exile this card. Whenever a creature attacks this turn, you gain 1 life.';assert.deepEqual(independentAbilityPrograms(ordinary),[ordinary]);
});
