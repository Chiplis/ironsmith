import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
const definition={keyword:'Spirited',reminder:"When this permanent enters, you may activate one of its activated abilities without paying that ability's cost."};
test('actual Hisano predicate preserves separate entering provider and trigger controller',()=>{
 const raw='If a permanent with spirited entering the battlefield causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.';
 const canonical=validateMechanic(definition).keyword;
 assert.equal(canonical,'spirited','the declaration identity uses the strict canonical form');
 const face=compileDraftInput({faces:[{name:'Hisano the White Lion',oracleText:raw}]},[definition]).faces[0];
 assert.equal(face.oracleText,`If a permanent with custom keyword "${canonical}" entering the battlefield causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.`);
 assert.equal(face.customKeywordIdentities,undefined,'a consumer is not a Spirited provider');
 assert.equal(face.originalPrintedRulesText,raw);
});
