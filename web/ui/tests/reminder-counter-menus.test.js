import test from 'node:test';import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
const def=validateMechanic({keyword:'Develop {Object}',reminder:"Choose a kind of counter that {Object} doesn't have on it from among {CounterKinds}. Put a counter of that kind on {Object}."});
test('closed counter menus bind the recipient and all distinct source bullet operands',()=>{
 const original='Whenever this creature attacks, develop it.\n• +1/+1\n• Flying';const face=compileDraftInput({faces:[{name:'Student',oracleText:original}]},[def]).faces[0];assert.match(face.oracleText,/from among \+1\/\+1 and flying/);assert.match(face.oracleText,/Put a counter of that kind on it\./);assert.equal(face.displayOracleText,original);assert.ok(!face.oracleText.includes('•'));
});
test('invalid or repeated menu operands remain unread',()=>{
 for(const bullets of ['• Flying\n• Flying','• Flying then draw a card.']){const original=`Develop it.\n${bullets}`;const face=compileDraftInput({faces:[{name:'Student',oracleText:original}]},[def]).faces[0];assert.ok(face.oracleText.includes('•'));}
});
test('counter identities retain explicit parameter operands',()=>{
 const face=compileDraftInput({faces:[{name:'Student',oracleText:'Develop it.\n• Ward {2}\n• Distorted'}]},[def]).faces[0];assert.match(face.oracleText,/from among ward \{2\} and distorted/);
});
