import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
const reminder='You may cast this spell for {Cost} rather than pay its mana cost. If you cast this spell this way, it has base power and toughness 4/4. If you cast this spell this way, it is legendary.\nAny time you could cast a sorcery, you may pay {Cost} to take a special action. If you do, this permanent has base power and toughness 4/4. Take this action only once.';
const def=validateMechanic({keyword:'Astral Poise',template:'{Cost}',reminder});
const compile=(name,text)=>compileDraftInput({customMechanics:[def],faces:[{name,oracleText:text}]},[def]).faces[0];
test('source payment exception becomes an explicit priority permission without activation rewriting',()=>{
 const text="Astral Poise {2}\nFlexible Performer's astral poise cost can be paid any time you could cast an instant.";
 const face=compile('Flexible Performer',text);
 assert.match(face.oracleText,/Any time you have priority, you may pay \{2\} to take a special action/);
 assert.doesNotMatch(face.oracleText,/cost can be paid/);
 assert.equal(face.displayOracleText,text);
});
test('an unrelated subject cannot change this sources action timing',()=>{
 const face=compile('Other Performer',"Astral Poise {2}\nFlexible Performer's astral poise cost can be paid any time you could cast an instant.");
 assert.match(face.oracleText,/Any time you could cast a sorcery/);
 assert.match(face.oracleText,/Flexible Performer's astral poise cost/);
});
test('nonmana prices and own alternative-price origin permission remain complete',()=>{
 const face=compile('Reserved Performer','Astral Poise—{2}{W}{B}, Pay 4 life.\nYou may cast Reserved Performer from your graveyard for its astral poise cost.');
 assert.match(face.oracleText,/from your graveyard for \{2\}\{W\}\{B\}, Pay 4 life rather than pay its mana cost/);
 assert.match(face.oracleText,/pay \{2\}\{W\}\{B\}, Pay 4 life to take a special action/);
});
test('a compound recipient grant gives only the functional ability in each zone',()=>{
 const face=compile('Grantor','Creature cards in your hand and attacking creatures you control have Astral Poise {3}{G}{U}.');
 assert.match(face.oracleText,/Creature cards in your hand have the alternative casting cost/);
 assert.match(face.oracleText,/attacking creatures you control have "Any time/);
 assert.doesNotMatch(face.oracleText,/have Astral Poise/);
});
