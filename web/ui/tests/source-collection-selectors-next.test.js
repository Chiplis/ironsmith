import test from 'node:test';
import assert from 'node:assert/strict';
import {expandActionParameterDeclarations} from '../src/lib/reminder-action-parameters.js';
const definition={keyword:'Store',template:'{Object}',reminder:'Exile {Object} from your graveyard until this creature leaves the battlefield. Maintain the cargo collection with cards exiled this way: Whenever this creature deals combat damage to a player, choose a card in exile from the cargo collection, then put it into its owner\'s hand.'};
test('declared ordinary card selections retain native counts and qualities',()=>{
 for(const selector of ['an artifact or creature card','a creature card with mana value 3 or less','up to one instant card and up to one sorcery card','up to eight noncreature, nonland cards','two cards at random','a card that was put into your graveyard this turn']){
  const text=`Store ${selector}.`;
  const expanded=expandActionParameterDeclarations(text,definition,'Carrier');
  assert.ok(expanded.startsWith(`Exile ${selector} from your graveyard`),expanded);
  assert.match(expanded,/Maintain the cargo collection/);
 }
});
test('a later instruction does not become a card selection operand',()=>{
 const expanded=expandActionParameterDeclarations('Store a card, then draw a card.',definition,'Carrier');
 assert.match(expanded,/Exile a card from your graveyard/);
 assert.match(expanded,/Then draw a card/);
});

test('declared subject-verb-object programs select one independent recipient',()=>{
 const recipient={keyword:'Store',template:'{Subject} stores {Object}',reminder:'Exile {Object} from your graveyard until {Subject} leaves the battlefield. Maintain the cargo collection with cards exiled this way: Whenever {Subject} deals combat damage to a player, choose a card in exile from the cargo collection, then put it into its owner\'s hand.'};
 const expanded=expandActionParameterDeclarations('Target artifact creature you control stores Carrier.',recipient,'Carrier');
 assert.match(expanded,/^Choose Target artifact creature you control\. Exile Carrier/);
 assert.match(expanded,/until that creature leaves the battlefield/);
 assert.match(expanded,/Whenever that creature deals combat damage/);
});

test('an alternative payment protects the entire declared recipient program',()=>{
 const recipient={keyword:'Store',template:'{Subject} stores {Object}',reminder:'Exile {Object} from your graveyard until {Subject} leaves the battlefield. Maintain the cargo collection with cards exiled this way: Whenever {Subject} deals combat damage to a player, draw a card.'};
 const expanded=expandActionParameterDeclarations('For each of those cards, a creature you control stores that card unless an opponent pays 3 life.',recipient,'Carrier');
 assert.match(expanded,/For each of those cards, Unless an opponent pays 3 life, do the following: Choose a creature you control; Exile that card/);
 assert.match(expanded,/; Maintain the cargo collection/);
});
