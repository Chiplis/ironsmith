import test from 'node:test';
import assert from 'node:assert/strict';
import {materializeCard} from '../../../scripts/custom-mechanics/cohort-input.mjs';
const original='Put the rest on the bottom of in any order.';
const source_context=`Reveal the top three cards of your library. Put one into your hand. ${original}`;
function card(){return {name:'Context witness',source_shape:'normal',source_faces:[{name:'Context witness',original_text:`Type: Creature\nPower/Toughness: 1/1\n${source_context}`,reviewed_source_corrections:[{original,replacement:'Put the rest on the bottom of your library in any order.',source_context,evidence:'The sole referenced ordered source zone is your library; the rest is that exact revealed group.',review_status:'source_context_reviewed'}]}]};}
test('reviewed source ellipsis correction affects only executable candidate and retains frozen original',()=>{
 const source=card(),before=structuredClone(source);const [face]=materializeCard(source,[]);assert.match(face.executable_text,/bottom of your library in any order/);assert.deepEqual(source,before);assert.match(source.source_faces[0].original_text,/bottom of in any order/);
});
test('unreviewed, mismatched, or multiply matching correction cannot silently rewrite source',()=>{
 const unreviewed=card();unreviewed.source_faces[0].reviewed_source_corrections[0].review_status='pending';assert.throws(()=>materializeCard(unreviewed,[]),/reviewed context/);
 const mismatch=card();mismatch.source_faces[0].reviewed_source_corrections[0].source_context='Another library. '+original;assert.throws(()=>materializeCard(mismatch,[]),/exact source occurrence/);
 const duplicate=card();duplicate.source_faces[0].original_text+='\n'+original;assert.throws(()=>materializeCard(duplicate,[]),/exact source occurrence/);
});
