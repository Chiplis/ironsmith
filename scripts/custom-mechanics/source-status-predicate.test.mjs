import test from 'node:test';
import assert from 'node:assert/strict';
import {executableDefinitions,indexSourceDefinitions,definitionsForSource} from './source-definitions.mjs';
import {materializeCard} from './cohort-input.mjs';
const definition={keyword:'Astral weave',template:'{Cost}',forms:{participle:'astral woven'},reminder:'As an additional cost to cast this spell, you may pay {Cost}. When you cast this spell, if you paid this additional cost, copy this spell and you may choose new targets for the copy.',statusPredicate:{spellCopy:true,customKeyword:'astral weave'}};
const card={id:'generic-source',source_card_key:'generic-source',name:'Independent source',original_text:'Type: Sorcery\nAstral weave {2}\nIf this spell is astral woven, draw a card.'};
const row={id:'opaque-row',keyword:'Astral weave',executable_definitions:[definition],representative_cards:[card]};
test('source definition transport keeps explicit copy status semantics through validation',()=>{
 const index=indexSourceDefinitions([{path:'source',fixture:{mechanics:[row]}}]);
 const definitions=definitionsForSource(card,executableDefinitions(row),index);
 assert.deepEqual(definitions[0].statusPredicate,definition.statusPredicate);
 const [face]=materializeCard(card,definitions,'msem');
 assert.match(face.executable_text,/If this spell is a copy with custom keyword "astral weave", draw a card/);
});
test('same source cannot carry conflicting predicate meanings under one definition',()=>{
 const other={...row,executable_definitions:[{...definition,statusPredicate:{spellCopy:true,customKeyword:'other identity'}}]};
 const index=indexSourceDefinitions([{path:'one',fixture:{mechanics:[row]}},{path:'two',fixture:{mechanics:[other]}}]);
 assert.throws(()=>definitionsForSource(card,[],index));
});
