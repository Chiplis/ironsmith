import test from 'node:test';
import assert from 'node:assert/strict';
import { compileDraftInput, validateMechanic } from '../src/lib/custom-mechanics.js';
const reminder = "You may put a +1/+1 counter on a creature. If you don't, create a 1/1 green Saproling creature token.";
const definition = validateMechanic({ keyword: 'Sproutify', reminder, actionProgram: true });
const compile = (text, definitions = [definition]) => compileDraftInput({ faces: [{ name: 'Source', oracleText: text }] }, definitions).faces[0];

test('explicit program metadata proposes the whole body on a different card', () => {
  assert.equal(definition.actionProgram, true);
  const face = compile('Sproutify.');
  assert.match(face.oracleText, /^Perform the sproutify action as follows: You may put/);
  assert.match(face.oracleText, /; If you don't, create a 1\/1 green Saproling/);
  assert.deepEqual(face.customKeywordIdentities, ['sproutify']);
});

test('would watcher names the operation and does not gain its keyword identity', () => {
  const face = compile('If you would sproutify, instead create a 1/1 green Saproling creature token.');
  assert.equal(face.oracleText, 'If you would perform the sproutify action, instead create a 1/1 green Saproling creature token.');
  assert.equal(face.customKeywordIdentities, undefined);
});

test('replacement cannot silently refer to a completion-only definition', () => {
  const marker = validateMechanic({ keyword: 'Sproutify', reminder });
  for (const text of ['If you would sproutify, instead draw a card.', 'If you would perform the sproutify action, instead draw a card.']) {
    assert.throws(() => compile(text, [marker]), /actionProgram declaration/);
  }
  assert.equal(compile('Sproutify.', [marker]).oracleText, reminder);
});

test('program metadata preserves official keyword protection and quoted opacity', () => {
  assert.throws(() => validateMechanic({ keyword: 'Flying', reminder, actionProgram: true }), /official keywords cannot be redefined/);
  const text = 'Search your library for a card named "If you would sproutify, instead draw a card.".';
  assert.equal(compile(text).oracleText, text);
});
