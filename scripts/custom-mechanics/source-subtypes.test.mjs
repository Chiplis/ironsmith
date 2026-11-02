import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {sourceSubtypeDefinitions} from './source-subtypes.mjs';
import {referencedSubtypes} from '../../web/ui/src/lib/custom-subtypes.js';
const registry=JSON.parse(readFileSync(new URL('../../fixtures/custom_mechanics/source-subtypes.json',import.meta.url),'utf8'));
test('every selected exact source type/token control obtains its authoritative family through generic ingestion',()=>{
 for(const control of registry.native_metadata_controls){
  const lines=control.test_parse_input.split('\n');const typeLine=lines.find(line=>line.startsWith('Type:')).slice(5).trim();
  const draft={faces:[{name:control.name,typeLine,oracleText:''}]};
  const metadata=sourceSubtypeDefinitions({},draft,control.scope);
  assert.deepEqual(metadata.find(def=>def.singular===control.definition.singular),control.definition);
 }
});
test('subtype library references include actual type-line metadata and counter-reminder programs',()=>{
 const draft={faces:[{name:'A Deity name alone',typeLine:'Enchantment — Deity',oracleText:'',customKeywordCounterDefinitions:[{name:'Witness',reminder:'This creature becomes a Shrimp.'}]}]};
 const metadata=sourceSubtypeDefinitions({},draft,'msem');
 assert.ok(metadata.some(def=>def.singular==='deity'&&def.family==='Enchantment'));
 assert.ok(metadata.some(def=>def.singular==='shrimp'&&def.family==='Creature'));
 assert.deepEqual(referencedSubtypes({faces:[{name:'Deity',oracleText:''}]},metadata),[]);
});
test('scope and keyword boundaries preserve unrelated names and do not invent Totemic subtype metadata',()=>{
 assert.deepEqual(sourceSubtypeDefinitions({}, {faces:[{name:'Vereaux',typeLine:'Plane — Vereaux',oracleText:''}]},'msem'),[]);
 assert.deepEqual(sourceSubtypeDefinitions({}, {faces:[{name:'Totem',typeLine:'Totemic Enchantment — Aura',oracleText:''}]},'rev'),[]);
 const metadata=sourceSubtypeDefinitions({}, {faces:[{typeLine:'Plane — Vereaux',oracleText:''}]},'revolution');
 assert.deepEqual(metadata,[{singular:'vereaux',plural:'vereauxes',family:'Planar'}]);
});
