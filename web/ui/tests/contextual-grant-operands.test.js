import test from 'node:test';import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
const input=oracleText=>({faces:[{name:'Selector',oracleText}]});
test('ordinary explicit selectors retain complete filters and are read as action operands',()=>{
 const def=validateMechanic({keyword:'Archive {Object}',reminder:'Exile {Object}.'});
 for(const operand of ['another target creature','enchanted creature','the other','each creature you control','target artifact or enchantment','target nontoken creature an opponent controls','that creature','the top card of your library','target creature an opponent controls','target card from your graveyard','target spell','all creature cards in graveyards','up to eight target creatures your opponents control','up to two target attacking nontoken creatures an opponent controls','a card from your hand']){
  const face=compileDraftInput(input(`Archive ${operand}.`),[def]).faces[0];assert.ok(!face.oracleText.includes('Archive'),operand);assert.ok(face.oracleText.includes(operand)||/^each |^all /.test(operand),operand);
 }
});
test('plural targets select once and execute the entire recipient program in one iteration',()=>{
 const def=validateMechanic({keyword:'Fold {Object}',reminder:"Exile {Object}. Return that card to the battlefield under its owner's control."});
 const face=compileDraftInput(input('Fold up to two target creatures you control. Draw a card.'),[def]).faces[0];assert.equal((face.oracleText.match(/target creatures/g)||[]).length,1);assert.equal(face.oracleText,"Choose up to two target creatures you control. For each permanent chosen this way, do the following: [Exile that permanent; Return that card to the battlefield under its owner's control]. Draw a card.");
});
test('another targeted operands keep their selection restriction when reused',()=>{
 const def=validateMechanic({keyword:'Protect {Object}',reminder:'Tap {Object}. Put a +1/+1 counter on {Object}.'});const face=compileDraftInput(input('Protect another target creature.'),[def]).faces[0];assert.match(face.oracleText,/Choose another target creature\. Tap that creature\. Put a \+1\/\+1 counter on that creature/);
});
test('a source-qualified short self name remains an explicit object operand',()=>{
 const def=validateMechanic({keyword:'Develop {Object}',reminder:'Put a +1/+1 counter on {Object}.'});const face=compileDraftInput({faces:[{name:'Student, Far Traveler',oracleText:'Develop Student.'}]},[def]).faces[0];assert.match(face.oracleText,/Put a \+1\/\+1 counter on Student/);
});
test('plural subject predicates choose the exact target group once before the complete conditional',()=>{
 const def=validateMechanic({keyword:'Retreat {Subject}',reminder:"If {Subject} is a nonblocking permanent, tap {Subject}, then put a +1/+1 counter on it."});const face=compileDraftInput(input('Up to two target lands you control retreat.'),[def]).faces[0];assert.equal((face.oracleText.match(/target lands/g)||[]).length,1);assert.equal(face.oracleText,'Choose Up to two target lands you control. For each permanent chosen this way, do the following: [If that permanent is a nonblocking permanent, tap that permanent, then put a +1/+1 counter on it].');
});

test('an immediate first loop action freezes its implicit object before a nested choice',()=>{
 const def=validateMechanic({keyword:'Bind {Object} onto a creature you control',reminder:'Choose a creature you control, then that creature gains "Whenever this creature attacks, draw a card." for as long as {Object} remains exiled and that creature remains on the battlefield.'});
 const original='For each card exiled this way, you may bind it onto a creature you control.';
 const face=compileDraftInput(input(original),[def]).faces[0];assert.ok(face.oracleText.includes('the iterated card remains exiled'));
 const later='For each card exiled this way, choose a creature you control, then bind it onto a creature you control.';const laterFace=compileDraftInput(input(later),[def]).faces[0];assert.ok(!laterFace.oracleText.includes('the iterated card'));
});
