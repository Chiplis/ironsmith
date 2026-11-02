import test from 'node:test';
import assert from 'node:assert/strict';
import {materializeCard} from '../../../scripts/custom-mechanics/cohort-input.mjs';

test('actual cohort token transport retains supplied token programs and original source evidence',()=>{
 for (const name of ['Keepsake','Lodestone']) {
  const token={name,manaCost:null,cardTypes:['Artifact'],oracleText:'{T}: Add {C}.'};
  const card={name:'Scoped creator',scope:'msem',original_text:`Type: Sorcery\nCreate a ${name} token.`,
   custom_tokens:[{face:token,input:structuredClone(token)}]};
  const before=structuredClone(card);
  const [face]=materializeCard(card,[],'msem');
  assert.match(face.executable_text,new RegExp(`Create a colorless artifact token named ${name}\\.`));
  assert.equal(face.custom_token_definitions.length,1);
  assert.equal(face.custom_token_definitions[0].name,name);
  assert.equal(face.custom_token_definitions[0].token,true);
  assert.match(face.custom_token_definitions[0].executable_text,/Type: Artifact\n\{T}: Add \{C}\./);
  assert.deepEqual(card,before,'source facts remain unchanged');
 }
});

test('ordinary source faces do not gain an implicit token definition by name',()=>{
 const [face]=materializeCard({name:'Ordinary creator',original_text:'Type: Sorcery\nCreate a Keepsake token.'},[],'msem');
 assert.deepEqual(face.custom_token_definitions,[]);
 assert.match(face.executable_text,/Create a Keepsake token/);
});
