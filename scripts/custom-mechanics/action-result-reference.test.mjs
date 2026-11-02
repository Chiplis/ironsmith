import assert from 'node:assert/strict';
import test from 'node:test';
import {expandActionParameterDeclarations} from '../../web/ui/src/lib/reminder-action-parameters.js';
const definition={keyword:'Unlock',template:'{Object}',reminder:'Turn the card chosen for this source face up.',resultReference:'the revealed card'};
test('action final-result reference binds within the same ability only',()=>{
 const result=expandActionParameterDeclarations('{6}: Unlock Ancient Maze. If the revealed card is green, put it into your hand.\nWhenever the revealed card attacks, draw a card.',definition,'Ancient Maze');
 assert.match(result,/Turn the card chosen for this source face up\. If it is green/);
 assert.match(result,/\nWhenever the revealed card attacks/);
 assert.doesNotMatch(result,/Reveal the/);
});
test('unrelated, quoted, invalid and cost occurrences keep their references',()=>{
 for(const original of ['If the revealed card is green, draw a card.','"Unlock Ancient Maze. If the revealed card is green, draw a card."','Unlock this unlikely sentence. If the revealed card is green, draw a card.','As an additional cost to cast this spell, Unlock Ancient Maze. If the revealed card is green, draw a card.']){
  assert.equal(expandActionParameterDeclarations(original,definition,'Ancient Maze'),original);
 }
});
