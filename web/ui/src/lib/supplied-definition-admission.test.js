import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {suppliedCardDefinitions} from './reminder-card-creation.js';
const input=metadata=>({suppliedCardDefinitions:[{name:'Design',text:'Type: Sorcery\nDraw a card.',...metadata}],faces:[]});
test('unsupported executable dependency metadata fails closed before compilation',()=>{
 for(const metadata of [{token:true},{isToken:true},{text:'Type: Token Creature'},
  {customTokens:[{name:'Helper'}]},{customKeywordCounterDefinitions:[{name:'gifted',reminder:'Flying'}]},
  {otherFaceId:2},{linkedFaceLayout:'transform'},{sourceShape:'split'},{faces:[{name:'Front'}]}])assert.throws(()=>suppliedCardDefinitions(input(metadata)));
});
test('supported standalone metadata and provenance survive without scope broadening',()=>{
 const records=suppliedCardDefinitions(input({token:false,customTokens:[],customKeywordCounterDefinitions:[],sourceShape:'normal',sourceCardKey:'Physical original',scope:'private'}));
 assert.equal(records.length,1);assert.equal(records[0].sourceCardKey,'Physical original');assert.equal(records[0].text,'Type: Sorcery\nDraw a card.');
});

test('all forty-two actual named source definitions remain within admitted standalone scope',()=>{
 const fixture=JSON.parse(readFileSync(new URL('../../../../fixtures/custom_mechanics/msem-conjure-whole-family-source-inputs.json',import.meta.url),'utf8'));
 const records=fixture.mechanics[0].representative_cards[0].supplied_card_definitions;
 assert.equal(records.length,42);
 assert.equal(suppliedCardDefinitions({suppliedCardDefinitions:records,faces:[]}).length,42);
});
