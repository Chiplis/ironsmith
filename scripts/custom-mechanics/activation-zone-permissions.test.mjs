import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../../web/ui/src/lib/custom-mechanics.js';
import {expandCustomKeywordPredicates,mentionsCustomKeywordPredicate} from '../../web/ui/src/lib/custom-keyword-predicates.js';
const authored={keyword:'Inscribe {Cost}',abilityDesignation:'inscribe',reminder:'{Cost}, Reveal this card from your hand: Exile this card from your hand. Activate only if this card is in your hand. Activate only as a sorcery.'};
test('authored occurrence designation preserves its custom keyword and is independent of card identity',()=>{
 const definition=validateMechanic(authored);
 assert.equal(definition.abilityDesignation,'inscribe');
 assert.equal(definition.reminder,authored.reminder,'canonical reminder stays plain for inline declaration compatibility');
 assert.match(definition.expansion,/Keyword ability "inscribe" —/);
 assert.equal((validateMechanic(definition).expansion.match(/Keyword ability "inscribe"/g)||[]).length,1);
 const result=compileDraftInput({faces:[{name:'Generic inscription',oracleText:'Inscribe {2}{W}'}]},[definition]);
 assert.match(result.faces[0].oracleText,/Keyword ability "inscribe" —/);
 assert.match(result.faces[0].oracleText,/\{2\}\{W\}/);
});
test('generic permission reader binds only supplied custom activation name and preserves quoted source facts',()=>{
 const definition=validateMechanic(authored);
 const source='You may activate inscribe abilities from your graveyard as though they were in your hand.';
 assert.equal(mentionsCustomKeywordPredicate(source,definition),true);
 assert.equal(expandCustomKeywordPredicates(source,[definition]),'You may activate abilities designated inscribe of cards in your graveyard as though they were in your hand.');
 assert.equal(expandCustomKeywordPredicates('You may activate unprovided abilities from your graveyard as though they were in your hand.',[definition]),'You may activate unprovided abilities from your graveyard as though they were in your hand.');
 const quote='A card has "You may activate inscribe abilities from your graveyard as though they were in your hand." in its printed text.';
 assert.equal(expandCustomKeywordPredicates(quote,[definition]),quote);
});

test('a designation cannot rename a custom keyword or redefine an official mechanic',()=>{
 assert.throws(()=>validateMechanic({...authored,abilityDesignation:'other name'}),/preserve this custom keyword name/);
 assert.throws(()=>validateMechanic({keyword:'Flying',abilityDesignation:'flying',reminder:'This creature cannot block.'}),/cannot be defined|official/i);
});
test('a granted costed activation retains its authored occurrence label inside the quoted rule',()=>{
 const definition=validateMechanic(authored);
 const input='Each creature you control has inscribe {2}{W}.';
 const output=compileDraftInput({faces:[{name:'Generic grantor',oracleText:input}]},[definition]).faces[0].oracleText;
 assert.match(output,/custom keyword "inscribe"/);
 assert.match(output,/"Keyword ability 'inscribe' — \{2\}\{W\}, Reveal this card from your hand:/);
 assert.doesNotMatch(output,/has inscribe \{2\}\{W\}/i);
 assert.equal(definition.reminder,authored.reminder);
});
