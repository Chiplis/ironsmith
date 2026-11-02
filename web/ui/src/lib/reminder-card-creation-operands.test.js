import test from 'node:test';
import assert from 'node:assert/strict';
import {expandExtendedCardCreationOperands as expand} from './reminder-card-creation-operands.js';
const config={label:'making',verb:'make',names:'Design|Forest|Island|Swamp'};
test('conjugated explicit actor owns creation and library placement',()=>{
 assert.equal(expand('Target opponent makes a card named Design on top of their library.',config),'Create a real card named "Design" on top of target opponent\'s library as part of the named making action.');
 assert.equal(expand('Each player makes a card named Design and puts it on top of their library.',config),"For each player, do the following: [Create a real card named \"Design\" on top of that player's library as part of the named making action].");
});
test('owner optional name choice stays one whole scoped operation',()=>{
 const text=expand('Its owner may make a card named Forest, Island, or Swamp on top of their library. Draw a card.',config);
 assert.match(text,/^Its owner may do the following: \[Choose one of the following:/);
 assert.match(text,/as part of the named making action\]\]\. Draw a card\.$/);
 assert.equal((text.match(/on top of its owner's library/g)||[]).length,3);
});
test('reordered and counted duplicates preserve destination and shuffle scope',()=>{
 assert.equal(expand('Make into your hand a duplicate of target land.',config),'Create a real card duplicating the target land in your hand as part of the named making action.');
 assert.equal(expand('You may make two duplicates of it into your main library, then shuffle.',config),'You may do the following: [Create two real cards duplicating the it on top of your library as part of the named making action; Shuffle your library].');
 assert.equal(expand('Make six duplicates of target instant card in a graveyard into your main library.',config),'Create six real cards duplicating the target instant card in a graveyard on top of your library as part of the named making action.');
});

test('per-card quantity uses typed prior-object count rather than fixed one',()=>{
 assert.equal(expand('They make a card named Design into their hand for each card exiled this way.',config),"Create the number of cards exiled this way real cards named \"Design\" in that player's hand as part of the named making action.");
 assert.equal(expand('They make two cards named Design into their hand for each card exiled this way.',config),'They make two cards named Design into their hand for each card exiled this way.');
});

test('player history consumes completed nonempty action receipts without source identity',()=>{
 const history={...config,past:'made'};
 assert.equal(expand("If you've made a card this turn, draw a card.",history),'If you have performed the making action with an amount of 1 or more this turn, draw a card.');
 assert.equal(expand("Unless you haven't made a card this turn, draw a card.",history),"Unless you have not performed the making action with an amount of 1 or more this turn, draw a card.");
 assert.equal(expand('If another player has made a card this turn, draw a card.',history),'If another player has made a card this turn, draw a card.');
});
