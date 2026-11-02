import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic, compileDraftInput} from '../src/lib/custom-mechanics.js';
const definition = keyword => validateMechanic({keyword: `${keyword} {Subject}`, reminder: 'You may pay the cost "Discard a nonland card or sacrifice a creature other than {Subject}". If you do, put a +1/+1 counter on {Subject} and draw a card.'});
const expand = (text, keyword='Gorge') => compileDraftInput({faces:[{name:'Witness',oracleText:text}]}, [definition(keyword)]).faces[0];
test('subject paid result remains inside the exact recipient scope without an outer acceptance',()=>{
 const original = 'You may have target creature you control gorge. When it does, you may destroy target artifact or creature with mana value less than or equal to that creature\'s power.';
 const face=expand(original);assert.equal(face.displayOracleText,original);
 assert.match(face.oracleText,/Choose target creature you control/);assert.match(face.oracleText,/When you do, you may destroy target/);
 assert.ok(!/When it does|You may do the following|gorge/i.test(face.oracleText));
 assert.equal((face.oracleText.match(/You may pay the cost/g)||[]).length,1);
});
test('same-recipient repetition chooses once and owns two independent payments',()=>{
 const face=expand('You may have target creature you control gorge, then you may have it gorge again.');
 assert.equal((face.oracleText.match(/Choose target creature/g)||[]).length,1);
 assert.equal((face.oracleText.match(/You may pay the cost/g)||[]).length,2);
 assert.ok(!/gorge|again/i.test(face.oracleText));
});
test('variable bounded groups keep one payment program per announced recipient',()=>{
 const face=expand('You may have any number of up to X target creatures you control gorge.');
 assert.match(face.oracleText,/Choose up to X target creatures you control/);assert.match(face.oracleText,/For each permanent chosen this way, do the following: \[/);
 assert.ok(!/You may do the following|gorge/i.test(face.oracleText));
});
test('subject completion composition works for unrelated declared names and preserves quotes',()=>{
 assert.match(expand('You may have this creature provision. When it does, gain 2 life.','Provision').oracleText,/When you do, gain 2 life/);
 const quoted='This creature has "You may have it gorge. When it does, draw a card."';assert.equal(expand(quoted).oracleText,quoted);
});

test('modal bullet is a structural boundary rather than part of the action subject',()=>{
 const original="Choose one —\n• Draw a card.\n• You may have any number of up to X target creatures you control gorge.";
 const face=expand(original);
 assert.match(face.oracleText,/Choose one —\n• Draw a card\.\n• Choose up to X target creatures you control\./);
 assert.match(face.oracleText,/For each permanent chosen this way, do the following:/);
 assert.ok(!/\bgorge\b|any number of up to X/i.test(face.oracleText));
 assert.equal(face.displayOracleText,original);
});
