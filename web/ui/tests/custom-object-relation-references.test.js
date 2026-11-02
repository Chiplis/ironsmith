import test from 'node:test';import assert from 'node:assert/strict';
import {normalizeObjectRelation,expandSourceObjectRelationReferences} from '../src/lib/reminder-object-relations.js';
import {validateMechanic,compileDraftInput,mergeMechanics} from '../src/lib/custom-mechanics.js';
const relation={name:'scripted host',sourceZone:'exile',memberZone:'battlefield'};
const definition=()=>validateMechanic({keyword:'Scriptmark {Cost}',forms:{participle:'scriptmarked'},objectRelation:relation,reminder:'{Cost}: Draw a card.'});
test('attachment-or-relation recipient becomes two native source-scoped statics',()=>{
 const text='Equipped or scriptmarked creature gets +2/+1.';
 assert.equal(expandSourceObjectRelationReferences(text,[definition()]),'Equipped creature gets +2/+1.\nAs long as this card is in exile, creature from the scripted host collection gets +2/+1.');
 const result=compileDraftInput({faces:[{name:'Attachment witness',oracleText:text}]},[definition()]).faces[0];
 assert.equal(result.oracleText,'Equipped creature gets +2/+1.\nAs long as this card is in exile, creature from the scripted host collection gets +2/+1.');
});
test('death subject and event recipient preserve source exile scope independently',()=>{
 const def=definition();
 assert.equal(expandSourceObjectRelationReferences('When scriptmarked creature dies, create a 2/2 colorless Spirit creature token.',[def]),'When a creature from the scripted host collection dies, if this card is in exile, create a 2/2 colorless Spirit creature token.');
 assert.equal(expandSourceObjectRelationReferences('Whenever you mill a card, scriptmarked creature gets +1/+1 until end of turn.',[def]),'Whenever you mill a card, if this card is in exile, creature from the scripted host collection gets +1/+1 until end of turn.');
});
test('unregistered terms, quoted bodies and unsupported forms remain verbatim',()=>{
 const def=definition();
 for(const text of ['Unrelated creature gets +2/+1.','Target creature gains "Scriptmarked creature gets +2/+1." until end of turn.','Whenever you mill a card, another scriptmarked creature gets +1/+1 until end of turn.'])assert.equal(expandSourceObjectRelationReferences(text,[def]),text);
 assert.equal(expandSourceObjectRelationReferences('Equipped or scriptmarked creature gets +2/+1.',[{...def,objectRelation:null}]),'Equipped or scriptmarked creature gets +2/+1.');
});
test('relation metadata remains explicit validated declaration data',()=>{
 assert.deepEqual(definition().objectRelation,relation);
 for(const value of [{...relation,name:'Bad label'},{...relation,sourceZone:'somewhere'},{...relation,extra:true}])assert.throws(()=>normalizeObjectRelation(value));
});

test('an existing keyword cannot silently change relation ownership metadata',()=>{
 const def=definition();
 assert.throws(()=>mergeMechanics([def],[{...def,objectRelation:{...relation,name:'other host'}}]));
 assert.throws(()=>mergeMechanics([def],[{...def,objectRelation:null}]));
 assert.deepEqual(mergeMechanics([def],[def])[0].objectRelation,relation);
});

test('plain source relation statics preserve mixed and quoted programs under the source zone',()=>{
 const def=definition();
 assert.equal(expandSourceObjectRelationReferences('Scriptmarked creature gets +1/+1 and has vigilance.',[def]),'As long as this card is in exile, creature from the scripted host collection gets +1/+1 and has vigilance.');
 assert.equal(expandSourceObjectRelationReferences('Scriptmarked creature has "Whenever this creature attacks, draw a card.".',[def]),'As long as this card is in exile, creature from the scripted host collection has "Whenever this creature attacks, draw a card.".');
 const quoted='Target creature gains "Scriptmarked creature gets +1/+1." until end of turn.';
 assert.equal(expandSourceObjectRelationReferences(quoted,[def]),quoted);
});
