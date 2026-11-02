import test from 'node:test';
import assert from 'node:assert/strict';
import {expandConditionSymbolRows} from '../src/lib/condition-symbol-rows.js';
import {compileDraftInput} from '../src/lib/custom-mechanics.js';
test('multiple symbols are independent conditions with one shared object quota and controller-only timing',()=>{
 const original='{ql}, {ql} — You control a creature.\n{ql} — You have lost life this turn.';
 const expanded=expandConditionSymbolRows(original,'Witness');
 assert.equal((expanded.match(/Special action "Mark condition/g)||[]).length,3);
 assert.equal((expanded.match(/Any time you could cast a sorcery, you may pay/g)||[]).length,3);
 assert.equal((expanded.match(/group "condition marking" share one use each turn/g)||[]).length,3);
 const names=[...expanded.matchAll(/If you do, give this permanent the "([^"]+)" status/g)].map(match=>match[1]);
 assert.equal(new Set(names).size,3);
 assert.ok(names.every(name=>expanded.includes(`this permanent has the "${name}" status`)));
 assert.match(expanded,/If you do, remove all statuses in group "condition marking" from this permanent/);
 const face=compileDraftInput({faces:[{name:'Witness',oracleText:original}]},[]).faces[0];
 assert.equal(face.displayOracleText,original);assert.equal(face.customKeywordIdentities,undefined);
 assert.equal(face.oracleText,expanded);
});
test('misaligned source symbol bar keeps its triggered opening and consumes the following Boolean paragraph',()=>{
 const text='{ql}, {ql} — When this permanent enters, draw a card.\nAn opponent controls three or more tapped nonland permanents.';
 const expanded=expandConditionSymbolRows(text,'Witness');
 assert.ok(expanded.startsWith('When this permanent enters, draw a card.\n'));
 assert.equal((expanded.match(/Take this action only if An opponent controls three or more tapped nonland permanents/g)||[]).length,2);
 assert.throws(()=>expandConditionSymbolRows('{ql} — When this permanent enters, draw a card.'),/Boolean condition/);
});
test('different source definitions do not share condition-status identities',()=>{
 const text='{ql} — You control a creature.';
 assert.notEqual(expandConditionSymbolRows(text,'First'),expandConditionSymbolRows(text,'Second'));
});
