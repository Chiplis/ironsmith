import test from 'node:test';
import assert from 'node:assert/strict';
import { expandTypeParameterPrograms, expandChosenTypeParameterPrograms, hasTypeParameterProgram } from '../src/lib/reminder-parameter-programs.js';
import { compileDraftInput, mergeMechanics, validateMechanic } from '../src/lib/custom-mechanics.js';
const definition = {keyword:'Survey', template:'a {Type} card', reminder:'Look at the top card of your library. You may reveal a {Type} card from among them and put it into your hand. You may put the rest into your graveyard.'};
test('draft compilation preserves identity and display text for complete parameter programs', () => {
  const mechanics = mergeMechanics([], [definition]);
  for (const source of [
    "Whenever Guide attacks, survey a creature or enchantment card X times, where X is Guide's power.",
    'Survey an artifact card, then do the same for creature, enchantment, instant, land, planeswalker, and sorcery.',
  ]) {
    const [face] = compileDraftInput({ faces: [{ name: 'Guide', oracleText: source }] }, mechanics).faces;
    assert.equal(face.oracleText, expandTypeParameterPrograms(source, definition));
    assert.deepEqual(face.customKeywordIdentities, ['survey']);
    assert.equal(face.displayOracleText, source);
  }
});
test('typed dynamic repetition retains source expression around the complete program', () => {
  const source = "Whenever Guide attacks, survey a creature or enchantment card X times, where X is Guide's power.";
  const actual = expandTypeParameterPrograms(source, definition);
  assert.ok(hasTypeParameterProgram(source, definition));
  assert.equal(actual, "Whenever Guide attacks, Repeat the following process X times, where X is Guide's power. Look at the top card of your library. You may reveal a creature or enchantment card from among them and put it into your hand. You may put the rest into your graveyard.");
});
test('type lists produce seven ordered independent complete programs', () => {
  const source = 'Survey an artifact card, then do the same for creature, enchantment, instant, land, planeswalker, and sorcery.';
  const actual = expandTypeParameterPrograms(source, definition);
  assert.equal((actual.match(/Look at the top card/g)||[]).length, 7);
  assert.equal((actual.match(/You may put the rest/g)||[]).length, 7);
  let previous = -1;
  for (const quality of ['artifact','creature','enchantment','instant','land','planeswalker','sorcery']) {
    const next = actual.indexOf(` ${quality} card from among them`);
    assert.ok(next > previous); previous = next;
  }
});
test('unbound variables, control-flow suffixes and names inside quotes remain untouched', () => {
  for (const source of [
    'Survey a creature card X times.',
    "Survey a creature card X times, where Y is Guide's power.",
    "Survey a creature card X times, where X is Guide's power, then draw a card.",
    'Survey a creature card, then do the same for artifact unless you pay {2}.',
    'Create a token named "Survey a creature card 3 times."',
  ]) assert.equal(expandTypeParameterPrograms(source, definition), source);
});

test('a local chosen quality binds exactly one resolution named choice and two complete programs', () => {
  const source = 'Whenever Scout attacks, choose land or nonland, then Survey a card of that kind.';
  const actual = expandTypeParameterPrograms(source, definition);
  assert.equal(actual, 'Whenever Scout attacks, Choose land or nonland. If land was chosen, Look at the top card of your library. You may reveal a land card from among them and put it into your hand. You may put the rest into your graveyard. Otherwise, Look at the top card of your library. You may reveal a nonland card from among them and put it into your hand. You may put the rest into your graveyard.');
  assert.equal((actual.match(/Choose land or nonland/g)||[]).length, 1);
  assert.equal((actual.match(/Look at the top card/g)||[]).length, 2);
  assert.equal((actual.match(/You may put the rest/g)||[]).length, 2);
  assert.ok(!/choose one|[•—]/i.test(actual), 'must not produce announcement-time printed modal text');
});
test('chosen quality expansion preserves original display and custom action identity through frontend integration', () => {
  const source = 'Whenever Scout attacks, choose land or nonland, then Survey a card of that kind.';
  const result = compileDraftInput({faces:[{name:'Scout',oracleText:source}]}, [validateMechanic(definition)]).faces[0];
  assert.equal(result.displayOracleText, source);
  assert.ok(result.customKeywordIdentities.some(name => name.toLowerCase() === 'survey'));
  assert.ok(result.oracleText.includes('If land was chosen,'));
  assert.ok(result.oracleText.includes('Otherwise, Look at the top card'));
  assert.ok(!/Survey a card of that kind/i.test(result.oracleText));
});
test('chosen quality requires an adjacent unambiguous typed selector and terminal bound action', () => {
  for (const source of [
    'Survey a card of that kind.',
    'Choose land or nonland. Draw a card. Survey a card of that kind.',
    'Choose land or nonland, then draw a card, then Survey a card of that kind.',
    'Choose land or land, then Survey a card of that kind.',
    'Choose artifact or creature or land, then Survey a card of that kind.',
    'Choose land or nonland, then Survey a card of that kind unless you pay {2}.',
    'Choose land or nonland, then Survey a card of that kind, then draw a card.',
    'Create a token named "Choose land or nonland, then Survey a card of that kind."',
  ]) assert.equal(expandChosenTypeParameterPrograms(source, definition), source);
  const nonType = {keyword:'Survey',template:'{Number}',reminder:'Draw {Number} cards.'};
  const source = 'Choose land or nonland, then Survey a card of that kind.';
  assert.equal(expandChosenTypeParameterPrograms(source, nonType), source);
});
