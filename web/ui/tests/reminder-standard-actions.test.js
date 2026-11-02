import test from 'node:test';
import assert from 'node:assert/strict';
import { compileDraftInput, mergeMechanics } from '../src/lib/custom-mechanics.js';
const expand = (text, definition) => compileDraftInput({faces:[{name:'Guide',oracleText:text}]}, mergeMechanics([], [definition])).faces[0].oracleText;
test('parameterized finite repetitions repeat the complete independent choice program', () => {
  const definition = { keyword:'Invigorate',template:'{Number}',reminder:'Choose a creature you control with the least power among creatures you control. Put {Number} +1/+1 counters on it.' };
  const text = expand('Invigorate 1 three times.', definition);
  assert.equal((text.match(/Choose a creature/g)||[]).length,3);
  assert.equal((text.match(/Put 1 \+1\/\+1 counters/g)||[]).length,3);
  assert.ok(!text.includes('{Number}'));
});
test('a repeated action after and preserves both optional decisions and fallbacks', () => {
  const definition = {keyword:'Crystallize',reminder:"You may put a +1/+1 counter on a creature you control. If you don't, create a Treasure token."};
  const actual = expand('Draw two cards and crystallize twice.',definition);
  assert.match(actual,/Draw two cards \. You may/);
  assert.equal((actual.match(/You may put/g)||[]).length,2);
  assert.equal((actual.match(/If you don't, create a Treasure token/g)||[]).length,2);
});
test('quoted references and keyword substrings remain intact', () => {
  const definition = {keyword:'Crystallize',reminder:'Draw a card.'};
  for (const text of ['Name a card named Crystallize.', 'Crystallized cards cost less.', 'Cards have “Draw two cards and crystallize twice.”']) assert.equal(expand(text,definition), text);
});
test('dynamic repetition bounds its complete program before following effects', () => {
  const actual = expand('Muster ranks X times. You lose X life.', {keyword:'Muster ranks',reminder:'Create a 1/1 white Soldier creature token. You gain 1 life.'});
  assert.equal(actual,'Repeat the following process X times: Create a 1/1 white Soldier creature token; You gain 1 life. You lose X life.');
});
test('subject actions preserve chosen or targeted objects without binding card prose', () => {
  const definition = {keyword:'Soar',template:'{Subject}',reminder:'Choose {Subject}. If that creature has flying, put a +1/+1 counter on it. Otherwise, put a flying counter on it.'};
  assert.equal(expand('At the beginning of combat, target creature you control soars.',definition),'At the beginning of combat, Choose target creature you control. If that creature has flying, put a +1/+1 counter on it. Otherwise, put a flying counter on it.');
  assert.equal(expand('The creature that soars.',definition),'The creature that soars.');
});
test('single action costs and unless clauses preserve the surrounding grammar', () => {
  const definition = {keyword:'Reappropriate',reminder:'Exile exactly two cards from your hand and/or graveyard.'};
  assert.equal(expand('As an additional cost to cast Guide, reappropriate or pay {3}.',definition),'As an additional cost to cast Guide, Exile exactly two cards from your hand and/or graveyard or pay {3}.');
  assert.equal(expand("Guide can't attack or block unless you reappropriate.",definition),"Guide can't attack or block unless you Exile exactly two cards from your hand and/or graveyard.");
});
test('single action activation prices preserve colon and alternative payments', () => {
  const definition = {keyword:'Reappropriate',reminder:'Exile exactly two cards from your hand and/or graveyard.'};
  assert.equal(expand('{T}, Reappropriate: Add {G}.',definition),'{T}, Exile exactly two cards from your hand and/or graveyard: Add {G}.');
  assert.equal(expand('{T} or reappropriate or pay 2 life: Add {G}.',definition),'{T} or Exile exactly two cards from your hand and/or graveyard or pay 2 life: Add {G}.');
});
