import test from 'node:test';
import assert from 'node:assert/strict';
import {expandActionParameterDeclarations} from '../src/lib/reminder-action-parameters.js';
const declaration={keyword:'Archive',template:'{Object} face down',reminder:'Look at {Object}. Exile it face down. If you do, create a colorless enchantment token. Perform the archive action with the most recently exiled card.'};
test('resolution card choice stays inside optional action and counted choices iterate exactly once each',()=>{
 const single=expandActionParameterDeclarations('You may archive a creature card from your graveyard face down.',declaration);
 assert.match(single,/You may do the following: \[Choose a creature card from your graveyard/);
 assert.match(single,/Look at that card/);
 const many=expandActionParameterDeclarations('Archive any number of cards from your hand face down.',declaration);
 assert.match(many,/Choose any number of cards from your hand/);
 assert.match(many,/For each card chosen this way, do the following/);
 assert.equal((many.match(/Perform the archive action/g)||[]).length,1);
});
test('explicit actor owns inspection exile token and completion with quoted ability controller unchanged',()=>{
 const d={...declaration,reminder:declaration.reminder.replace('colorless enchantment token','colorless enchantment token with "{2}, Sacrifice this enchantment: You may play the exiled card this turn"')};
 const value=expandActionParameterDeclarations('When this creature enters, each player may archive the top card of their library face down.',d);
 assert.match(value,/For each player, do the following/);
 assert.match(value,/That player may do the following/);
 assert.match(value,/that player looks at the top card of their library/i);
 assert.match(value,/that player creates a colorless enchantment token/i);
 assert.match(value,/that player performs the archive action/i);
 assert.match(value,/"\{2}, Sacrifice this enchantment: You may play the exiled card this turn"/);
});
test('arbitrary imperative text is not an object operand',()=>{
 const source='Archive draw a card then a card from your hand face down.';
 assert.equal(expandActionParameterDeclarations(source,declaration),source);
});

test('per-object program selects once and iterates singleton reference without a success gate',()=>{
 const d={...declaration,perObjectProgram:true};
 const value=expandActionParameterDeclarations('Archive the top card of your library face down.',d);
 assert.match(value,/Choose the top card of your library/);
 assert.match(value,/For each card chosen this way, do the following/);
 assert.match(value,/Look at the iterated object/);
 assert.equal(value.includes('If you exiled'),false);
});

test('instead payloads retain one complete future event program rather than immediate effects',()=>{
 const d={...declaration,template:'{Object}',perObjectProgram:true,reminder:'Exile {Object}. Create a Memory token.'};
 const dying=expandActionParameterDeclarations('If a creature would die this turn, archive it instead.',d);
 assert.match(dying,/would die this turn, do the following instead: \[Choose it/);
 const resolving=expandActionParameterDeclarations('Whenever you cast a spell, archive it instead of putting it into your graveyard as it resolves.',d);
 assert.match(resolving,/Instead of putting the triggering spell into your graveyard as it resolves, do the following: \[Choose it/);
});
