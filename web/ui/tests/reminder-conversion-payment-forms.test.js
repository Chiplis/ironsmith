import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
const definition=(count,noun)=>validateMechanic({keyword:'Waste',template:`${count} non{Name} ${noun} you control`,reminder:`Choose ${count} ${noun} you control not named {Name}. They become a basic land named Wastes. Perform the waste action with permanents affected this way.`,paymentReminder:`Have ${count} ${noun} you control not named {Name} become a basic land named Wastes and perform the waste action.`});
const expand=(text,definition)=>compileDraftInput({faces:[{name:'Payment witness',oracleText:text}]},[definition]).faces[0].oracleText;
test('alternative conversion payment preserves the conjoined mana price and replacement casting form',()=>{
 const text="You may pay {U} and waste three nonwastes lands you control rather than pay this spell's mana cost.";
 assert.equal(expand(text,definition('three','lands')),"You may pay {U} and Have three lands you control not named wastes become a basic land named Wastes and perform the waste action rather than pay this spell's mana cost.");
});
test('optional additional conversion payment keeps its original announced amount and optionality',()=>{
 const text='You may waste any number of nonwastes lands you control as an additional cost to cast this spell.';
 assert.equal(expand(text,definition('any number of','lands')),'You may Have any number of lands you control not named wastes become a basic land named Wastes and perform the waste action as an additional cost to cast this spell.');
});
test('additional cost prefix is supported only when a payment meaning is explicitly declared',()=>{
 const text='As an additional cost to cast this spell, waste two nonwastes lands you control.';
 assert.match(expand(text,definition('two','lands')),/cast this spell, Have two lands/);
 const bare=validateMechanic({keyword:'Waste',template:'two non{Name} lands you control',reminder:'Choose two lands you control not named {Name}.'});
 assert.equal(expand(text,bare),text);
});
