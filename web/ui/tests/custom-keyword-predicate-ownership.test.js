import test from 'node:test';
import assert from 'node:assert/strict';
import {mentionsCustomKeywordPredicate,expandCustomKeywordPredicates} from '../src/lib/custom-keyword-predicates.js';
const definition={keyword:'compose'};
test('creation grant operands remain owned by their grant reader',()=>{
 for(const text of ['Create a Function token with compose.','Create a Function token named "Dr. Example" with compose.']) {
 assert.equal(mentionsCustomKeywordPredicate(text,definition),false);
 assert.equal(expandCustomKeywordPredicates(text,[definition]),text);
 }
});
test('real object predicates retain supplied exact keyword identities',()=>{
 for(const text of ['Creatures with compose you control get +1/+1.','Create a token that is a copy of target creature with compose.','Create a token that is a copy of a token with compose.']) {
  assert.equal(mentionsCustomKeywordPredicate(text,definition),true);
  assert.match(expandCustomKeywordPredicates(text,[definition]),/with custom keyword "compose"/);
 }
});
test('a creation grant and a counted object predicate have distinct owners',()=>{
 const text='Create a Function token with compose for each creature with compose.';
 assert.equal(expandCustomKeywordPredicates(text,[definition]),'Create a Function token with compose for each creature with custom keyword "compose".');
});
test('quoted rules remain opaque',()=>{
 const text='Target permanent gains "Creatures with compose you control get +1/+1."';
 assert.equal(mentionsCustomKeywordPredicate(text,definition),false);
 assert.equal(expandCustomKeywordPredicates(text,[definition]),text);
});
