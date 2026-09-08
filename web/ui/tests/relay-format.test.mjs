import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { validateFormatDeck, assertFormatMatch } from '../src/lib/relay/format-legality.js';
import { normalizeMatchFormat, evaluateLobbyDeckSubmission } from '../src/lib/decklists.js';
const catalog = JSON.parse(await readFile(new URL('../src/lib/relay/format-catalog.generated.json', import.meta.url)));
const deck = (name, count = 60) => Array(count).fill(name);
const check = (format, main, commanders = [], sideboard = []) => validateFormatDeck(format, main, commanders, sideboard, catalog);
test('constructed minimum size, four copies across main and sideboard, no commander slot', () => {
  assert.equal(check('modern', deck('Plains', 61)).ready, true);
  assert.equal(check('modern', deck('Plains', 59)).ready, false);
  assert.equal(check('modern', [...deck('Plains', 56), ...deck('Lightning Bolt', 4)], [], ['Lightning Bolt']).ready, false);
  assert.equal(check('modern', deck('Plains'), [], deck('Island', 16)).ready, false);
  assert.equal(check('modern', deck('Plains'), ['Isamaru, Hound of Konda']).ready, false);
  assert.equal(normalizeMatchFormat('modern'), 'modern');
  assert.equal(evaluateLobbyDeckSubmission('modern', deck('Plains', 61)).ready, true);
  assert.equal(evaluateLobbyDeckSubmission('normal', deck('Plains', 61)).ready, false);
});
test('format bans, card pool, and Vintage restriction are distinct', () => {
  assert.equal(check('modern', [...deck('Plains', 59), 'Sol Ring']).ready, false);
  assert.equal(check('vintage', [...deck('Plains', 59), 'Sol Ring']).ready, true);
  assert.equal(check('vintage', [...deck('Plains', 59), 'Sol Ring'], [], ['Sol Ring']).ready, false);
  assert.equal(check('legacy', [...deck('Plains', 59), 'Sol Ring']).ready, false);
  assert.equal(check('modern', [...deck('Plains', 59), 'Definitely Not A Card']).ready, false);
  assert.equal(check('modern', [...deck('Plains', 56), ...deck('Seven Dwarves', 4)], [], deck('Seven Dwarves', 3)).ready, true);
});
test('Commander size, identity, commander eligibility, singleton, and legal pairs', () => {
  assert.equal(check('commander', deck('Plains', 99), ['Isamaru, Hound of Konda']).ready, true);
  assert.equal(check('commander', [...deck('Plains', 98), 'Island'], ['Isamaru, Hound of Konda']).ready, false);
  assert.equal(check('commander', deck('Plains', 99), ['Plains']).ready, false);
  assert.equal(check('commander', [...deck('Plains', 97), 'Swords to Plowshares', 'Swords to Plowshares'], ['Isamaru, Hound of Konda']).ready, false);
  assert.equal(check('commander', deck('Plains', 98), ['Isamaru, Hound of Konda', 'Isamaru, Hound of Konda']).ready, false);
  assert.equal(check('commander', deck('Plains', 98), ['Tymna the Weaver', 'Thrasios, Triton Hero']).ready, true);
});
test('match gate checks format rules and rejects missing catalog', () => {
  const config = { format: 'modern', startingLife: 20, playerNames: ['A', 'B'], decks: [deck('Plains'), deck('Island')] };
  assert.doesNotThrow(() => assertFormatMatch(config, catalog));
  assert.throws(() => assertFormatMatch({ ...config, startingLife: 40 }, catalog));
  assert.throws(() => assertFormatMatch({ ...config, playerNames: ['A', 'B', 'C'] }, catalog));
  assert.equal(validateFormatDeck('modern', deck('Plains')).ready, false);
});
