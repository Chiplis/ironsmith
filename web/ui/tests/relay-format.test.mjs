import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { validateFormatDeck, assertFormatMatch, assertFormatDecks } from '../src/lib/relay/format-legality.js';
import { normalizeMatchFormat, evaluateLobbyDeckSubmission } from '../src/lib/decklists.js';
import { PUBLIC_FORMATS, isLegalityFormat, usesCommanders } from '../src/lib/relay/formats.js';
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
  assert.equal(evaluateLobbyDeckSubmission('normal', deck('Plains', 61)).ready, true);
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
test('Pauper EDH: uncommon commander, commons-only 99, its own bans and life total', () => {
  const iconoclast = ['Third Path Iconoclast'];
  assert.equal(PUBLIC_FORMATS.paupercommander.engineFormat, 'pauper_commander');
  assert.equal(usesCommanders('paupercommander') && usesCommanders('pauper_commander'), true);
  assert.equal(check('paupercommander', deck('Mountain', 99), iconoclast).ready, true);
  assert.equal(check('commander', deck('Mountain', 99), iconoclast).ready, false);
  assert.equal(check('paupercommander', deck('Mountain', 99), ['Krenko, Mob Boss']).ready, false);
  assert.equal(check('paupercommander', [...deck('Mountain', 98), 'Sol Ring'], iconoclast).ready, false);
  assert.equal(check('paupercommander', [...deck('Mountain', 98), 'Rhystic Study'], ['Tatyova, Benthic Druid']).ready, false);
  assert.equal(check('paupercommander', [...deck('Mountain', 98), 'Lightning Bolt'], iconoclast).ready, true);
  assert.equal(evaluateLobbyDeckSubmission('paupercommander', deck('Mountain', 99), iconoclast).ready, true);
  assert.doesNotThrow(() => assertFormatMatch({ format: 'paupercommander', startingLife: 30, playerNames: ['A', 'B', 'C'],
    decks: [deck('Mountain', 99), deck('Mountain', 99), deck('Mountain', 99)], commanders: [iconoclast, iconoclast, iconoclast] }, catalog));
});
test('match gate checks format rules and rejects missing catalog', () => {
  const config = { format: 'modern', startingLife: 20, playerNames: ['A', 'B'], decks: [deck('Plains'), deck('Island')] };
  assert.doesNotThrow(() => assertFormatMatch(config, catalog));
  assert.throws(() => assertFormatMatch({ ...config, startingLife: 40 }, catalog));
  assert.throws(() => assertFormatMatch({ ...config, playerNames: ['A', 'B', 'C'] }, catalog));
  assert.equal(validateFormatDeck('modern', deck('Plains')).ready, false);
});
test('unreleased cards need the lobby toggle and still obey copy limits', () => {
  const preview = { name: 'Preview Dragon', legalities: Object.fromEntries(Object.keys(PUBLIC_FORMATS).map(f => [f, 'not_legal'])),
    colors: ['R'], type: 'Creature — Dragon', rules: '', unreleased: true };
  const data = { ...catalog, cards: { ...catalog.cards, 'preview dragon': preview } };
  const main = [...deck('Mountain', 56), ...deck('Preview Dragon', 4)];
  const blocked = validateFormatDeck('modern', main, [], [], data);
  assert.equal(blocked.ready, false);
  assert.match(blocked.cardProblems['Preview Dragon'], /unreleased/);
  assert.equal(validateFormatDeck('modern', main, [], [], data, { allowUnreleased: true }).ready, true);
  assert.equal(validateFormatDeck('modern', [...main, 'Preview Dragon'], [], [], data, { allowUnreleased: true }).ready, false);
  const config = { format: 'modern', startingLife: 20, playerNames: ['A', 'B'], decks: [main, deck('Island')] };
  assert.throws(() => assertFormatDecks(config, data));
  assert.doesNotThrow(() => assertFormatDecks({ ...config, allowUnreleased: true }, data));
});
test('card problems name the card as the deck wrote it', () => {
  const result = check('modern', [...deck('Plains', 59), 'Sol Ring']);
  assert.deepEqual(Object.keys(result.cardProblems), ['Sol Ring']);
  assert.equal(isLegalityFormat('modern') && isLegalityFormat('commander'), true);
  assert.equal(isLegalityFormat('normal') || isLegalityFormat('planechase'), false);
});
