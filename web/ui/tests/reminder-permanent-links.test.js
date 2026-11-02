import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
test('explicit relationship references compile to a source-relative filter',()=>{
 for(const keyword of ['overclock','tether']){
  const definition=validateMechanic({keyword,reminder:'When this creature enters, you gain 1 life.',permanentLinkRelation:'sample'});
  const participle=keyword==='overclock'?'Overclocked':'Tethered';
  const result=compileDraftInput({faces:[{name:'Source',oracleText:`${keyword}\n${participle} artifact has flying.`}]},[definition]).faces[0];
  assert.match(result.oracleText,/artifact linked by sample to this permanent has flying/);
  assert.deepEqual(result.customKeywordIdentities,[keyword]);
 }
});
test('quoted words and unconfigured relationships stay unchanged',()=>{
 const text='Overclocked artifact has flying.';assert.equal(compileDraftInput({faces:[{name:'Unknown',oracleText:text}]},[]).faces[0].oracleText,text);
 const definition=validateMechanic({keyword:'overclock',reminder:'You gain 1 life.',permanentLinkRelation:'sample'});
 const quoted='This creature has "Overclocked artifact has flying."';assert.equal(compileDraftInput({faces:[{name:'Quoted',oracleText:quoted}]},[definition]).faces[0].oracleText,quoted);
});
