import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {materializeCard} from './cohort-input.mjs';
import {executableDefinitions,indexSourceDefinitions,definitionsForSource} from './source-definitions.mjs';
const fixture=JSON.parse(readFileSync(new URL('../../fixtures/custom_mechanics/revolution-stitch-fullsource.json',import.meta.url),'utf8'));
const entries=fixture.mechanics.flatMap(row=>row.representative_cards.map(card=>({row,card})));
const index=indexSourceDefinitions([{path:'stitch',fixture}]);
function materialize({row,card}){return materializeCard(card,definitionsForSource(card,executableDefinitions(row),index),'revolution')[0];}
test('all15 formal source faces retain originals and expand declared operations',()=>{
 assert.equal(entries.length,15);
 for(const entry of entries){const original=JSON.stringify(entry.card.source_faces);const face=materialize(entry);assert.equal(JSON.stringify(entry.card.source_faces),original);assert.doesNotMatch(face.executable_text,/\bstitch\s+(?:\d+|X)\b/i);}
});
test('alternative resolving price and creation-time grant stay ordinary and operation-scoped',()=>{
 const price=materialize(entries.find(entry=>entry.card.name==='Stonegraf Awakener')).executable_text;
 assert.match(price,/pay the cost "exile a Zombie card from your graveyard" rather than the cost of performing the stitch action/i);
 const grant=materialize(entries.find(entry=>entry.card.name==='Master Skaabstitcher')).executable_text;
 assert.match(grant,/As you perform the stitch action, tokens you create gain/i);
 assert.match(grant,/instant or sorcery card in exile from the stitched collection/i);
});
test('resolution-local X and authored entry modifier retain their complete source scope',()=>{
 const local=materialize(entries.find(entry=>entry.card.name==="It's Alive!")).executable_text;
 assert.match(local,/With X equal to that creature['’]s power, do the following/i);
 const tapped=materialize(entries.find(entry=>entry.card.name==='Nephalia Heartsewer')).executable_text;
 assert.match(tapped,/The token enters tapped/i);
});
