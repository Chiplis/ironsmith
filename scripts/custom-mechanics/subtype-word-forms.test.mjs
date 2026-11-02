import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {validateSubtype,referencedSubtypes,subtypeFormsMentioned} from '../../web/ui/src/lib/custom-subtypes.js';
import {sourceSubtypeDefinitions} from './source-subtypes.mjs';
const registry=JSON.parse(readFileSync(new URL('../../fixtures/custom_mechanics/source-subtypes.json',import.meta.url),'utf8'));
test('all six actual source forms flow from exact source type lines to scoped frontend metadata',()=>{
 const controls=registry.native_metadata_controls.filter(row=>/[^a-z]/.test(row.definition.singular));
 assert.equal(controls.length,6);
 for(const row of controls){
  const draft={faces:[{name:row.name,typeLine:row.source_type_line,oracleText:''}]};
  const metadata=sourceSubtypeDefinitions({},draft,row.scope);
  assert.deepEqual(metadata.find(def=>def.singular===row.definition.singular),row.definition);
  assert.equal(draft.faces[0].typeLine,row.source_type_line);
 }
});
test('Unicode, apostrophe normalization, hyphens, digits and multiword aliases have matching boundaries',()=>{
 for(const form of ['Éloi',"Ing'Wei",'As’Ahai','XC037','Cd-hr','Silver Horizon']){
  const def=validateSubtype({singular:form,plural:form+'s',family:'Planeswalker'});
  assert.ok(subtypeFormsMentioned(`a ${form} planeswalker`,def));
  assert.ok(subtypeFormsMentioned(`a non-${form} planeswalker`,def));
  assert.ok(!subtypeFormsMentioned(`prefix${form}suffix`,def));
  assert.deepEqual(referencedSubtypes({faces:[{name:form,oracleText:''}]},[def]),[]);
 }
 assert.equal(validateSubtype({singular:'As’Ahai',plural:'As’Ahais',family:'Planeswalker'}).singular,"as'ahai");
});
test('malformed forms remain rejected without creating a subtype from source names or flavor',()=>{
 for(const name of ['-Cloud','Cloud-','Cloud;;Kin','123','Cloud/*Kin'])assert.throws(()=>validateSubtype({singular:name,plural:name+'s',family:'Creature'}));
});
