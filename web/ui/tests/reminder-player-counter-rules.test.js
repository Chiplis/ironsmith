import test from 'node:test';
import assert from 'node:assert/strict';
import {expandPlayerCounterRuleDeclarations} from '../src/lib/reminder-player-counter-rules.js';
test('player counter reminder becomes actor-owned rule and independently parsed loop body',()=>{
 for(const name of ['corruption','restlessness']) {
  const source=`Target player gets three ${name} counters. *(At your first main phase, for each ${name} counter on you, mill a card. If it's nonland, you lose 1 life and the counter.)*`;
  const text=expandPlayerCounterRuleDeclarations(source);
  assert.match(text,/for that player with event condition/);assert.match(text,/first main phase of the turn/);assert.match(text,/At the beginning of each of your main phases/);assert.match(text,/do the following: mill a card/);assert.match(text,/If a nonland card was milled this way/);assert.ok(text.includes(`remove a ${name} counter from yourself`));
 }
});
test('permanent counter with same name never registers player meaning',()=>{
 const source='Put a corruption counter on a land you control.';
 assert.equal(expandPlayerCounterRuleDeclarations(source),source);
 const mismatch='Target player gets three sleep counters. (At your first main phase, for each wake counter on you, mill a card.)';
 assert.equal(expandPlayerCounterRuleDeclarations(mismatch),mismatch);
});
