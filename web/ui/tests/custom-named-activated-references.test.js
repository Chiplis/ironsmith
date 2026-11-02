import test from 'node:test';
import assert from 'node:assert/strict';
import {expandNamedActivatedReferences} from '../src/lib/reminder-named-activated-references.js';
import {compileDraftInput,mergeMechanics} from '../src/lib/custom-mechanics.js';
const recipe={keyword:'Rekindle',template:'{Cost}',reminder:"{Cost}, Exile this card from your graveyard: This is the rekindle activated ability. Create a token that's a copy of this card, except it has no mana cost. Perform the rekindle action with objects affected this way. Activate only as a sorcery.",forms:{eventVerb:'rekindle',eventLabel:'rekindle'}};
test('activation references use an explicit full selected-ability label and preserve filter',()=>{
 const source='Rekindle abilities of creature cards in your graveyard cost {1}{B} less.';
 assert.equal(expandNamedActivatedReferences(source,recipe),'Activated abilities named "rekindle" of creature cards in your graveyard cost {1}{B} less to activate.');
 assert.match(expandNamedActivatedReferences(source,{...recipe,reminder:recipe.reminder.replace('This is the rekindle activated ability.','This is the rekindle craft activated ability.')}),/named "rekindle craft"/);
 assert.equal(expandNamedActivatedReferences(source,{...recipe,reminder:recipe.reminder.replace('This is the rekindle activated ability. ','')}),source);
 assert.equal(expandNamedActivatedReferences('"'+source+'"',recipe),'"'+source+'"');
});
test('participant references preserve heterogeneous trigger alternatives and resolution body',()=>{
 const source='Whenever another creature you control enters from a graveyard or you rekindle a creature token, that creature gets +2/+0 until end of turn.';
 assert.equal(expandNamedActivatedReferences(source,recipe),'Whenever another creature you control enters from a graveyard or a creature token participates in the rekindle action performed by you, that creature gets +2/+0 until end of turn.');
 const body='Whenever a creature attacks, you rekindle a creature token.';
 assert.equal(expandNamedActivatedReferences(body,recipe),body,'only explicit trigger occurrence heads are adapted');
});
test('reference-only sources acquire no recipient keyword identity',()=>{
 const defs=mergeMechanics([],[recipe]);
 for(const oracleText of ['Rekindle abilities of cards in your graveyard cost {1} less.','Whenever you rekindle a creature token, draw a card.']){
  const face=compileDraftInput({faces:[{name:'Observer',oracleText}]},defs).faces[0];
  assert.equal(face.customKeywordIdentities,undefined);assert.notEqual(face.oracleText,oracleText);
 }
});

test('overlapping complete and historical source rows retain one Revive definition identity',async()=>{
 const {readFile}=await import('node:fs/promises');
 const names=['msem-revive.json','msem-casting-middle.json','msem-revive-whole-source-inputs.json'];let expected;
 for(const name of names){const data=JSON.parse(await readFile(new URL('../../../fixtures/custom_mechanics/'+name,import.meta.url),'utf8'));
  for(const row of data.mechanics)for(const definition of row.executable_definitions||[]){if(!/^Revive(?: \{Cost\})?$/i.test(definition.keyword))continue;
   if(!expected)expected=definition;else assert.deepEqual(definition,expected,'source-index overlap must not silently choose a different reminder');
  }
 }
 assert.match(expected.reminder,/This is the revive activated ability\./);assert.match(expected.reminder,/Perform the revive action with objects affected this way\./);
});
