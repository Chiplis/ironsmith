import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import {
  splitDeckColorNote, deckCardBaseName, circledColorCount, deckColorNoteProblem,
  submissionColorNoteIssues, describeColorNoteIssues,
} from '../src/lib/deck-color-notes.js';
import { parseDeckList, parseDeckPrintPreferences } from '../src/lib/decklists.js';
import { cardRouteKey } from '../src/lib/scryfall.js';
import { validateFormatDeck, deckConfigurationProblems } from '../src/lib/relay/format-legality.js';

const catalog = JSON.parse(await readFile(new URL('../src/lib/relay/format-catalog.generated.json', import.meta.url)));
const SPIRES_RULES = 'As you create your deck, circle two of the colors below.';

test('a decklist entry splits into the card name and its circled colors', () => {
  assert.deepEqual(splitDeckColorNote('Cryptic Spires ({W}{B})'), { name: 'Cryptic Spires', colors: ['W', 'B'] });
  assert.deepEqual(splitDeckColorNote(' Cryptic Spires ( {u} {R} ) '), { name: 'Cryptic Spires', colors: ['U', 'R'] });
  for (const plain of ['Cryptic Spires', 'Cryptic Spires (MKM)', '({W}{B})', 'Spires ({W}{X})']) {
    assert.equal(splitDeckColorNote(plain).colors, null, plain);
  }
  assert.equal(deckCardBaseName('Cryptic Spires ({W}{B})'), 'Cryptic Spires');
  assert.equal(circledColorCount(SPIRES_RULES), 2);
  assert.equal(circledColorCount('A deck can have any number of cards named Rats.'), null);
});

test('a circling card needs exactly that many different colors; no other card takes a note', () => {
  assert.equal(deckColorNoteProblem('Cryptic Spires', SPIRES_RULES, 'Cryptic Spires ({W}{B})'), null);
  assert.match(deckColorNoteProblem('Cryptic Spires', SPIRES_RULES, 'Cryptic Spires'), /Cryptic Spires \(\{W\}\{U\}\)/);
  assert.ok(deckColorNoteProblem('Cryptic Spires', SPIRES_RULES, 'Cryptic Spires ({W}{W})'));
  assert.ok(deckColorNoteProblem('Cryptic Spires', SPIRES_RULES, 'Cryptic Spires ({W}{U}{B})'));
  assert.ok(deckColorNoteProblem('Plains', '', 'Plains ({W}{B})'));
  assert.equal(deckColorNoteProblem('Plains', '', 'Plains'), null);
});

test('the note stays on deck entries but never reaches card assets or printings', () => {
  const deck = parseDeckList('4 Cryptic Spires ({W}{B}) (MKM) 255\n2x Cryptic Spires ({U}{R})');
  assert.deepEqual(deck, [...Array(4).fill('Cryptic Spires ({W}{B})'), ...Array(2).fill('Cryptic Spires ({U}{R})')]);
  assert.equal(cardRouteKey('Cryptic Spires ({W}{B})'), cardRouteKey('Cryptic Spires'));
  const [preference] = parseDeckPrintPreferences('4 Cryptic Spires ({W}{B}) (MKM) 255');
  assert.equal(preference.name, 'Cryptic Spires');
  assert.equal(preference.setCode, 'MKM');
});

test('a missing note is configuration, not illegality; noted copies share the copy limit', () => {
  const lands = Array(56).fill('Plains');
  const unnoted = [...Array(4).fill('Cryptic Spires'), ...lands];
  const missing = validateFormatDeck('pauper', unnoted, [], [], catalog);
  assert.equal(missing.ready, true, 'the deck stays legal in its format');
  assert.deepEqual(missing.cardProblems, {});
  assert.match(missing.configurationProblems['Cryptic Spires'], /circled/);
  assert.match(deckConfigurationProblems(unnoted, [], [], catalog).cardProblems['Cryptic Spires'], /circled/);
  assert.deepEqual(deckConfigurationProblems(Array(4).fill('Cryptic Spires ({B}{G})'), [], [], catalog).errors, []);
  const noted = validateFormatDeck('pauper', [...Array(4).fill('Cryptic Spires ({B}{G})'), ...lands], [], [], catalog);
  assert.equal(noted.ready, true, noted.errors.join(' '));
  assert.deepEqual(noted.configurationErrors, []);
  const split = ['Cryptic Spires ({B}{G})', 'Cryptic Spires ({B}{G})', 'Cryptic Spires ({W}{U})', 'Cryptic Spires ({W}{U})', 'Cryptic Spires ({W}{U})'];
  const tooMany = validateFormatDeck('pauper', [...split, ...lands], [], [], catalog);
  assert.equal(tooMany.ready, false);
  assert.ok(tooMany.errors.some((error) => /at most 4/.test(error)), tooMany.errors.join(' '));
});

test('submission checks ask the engine and summarize its issues', async () => {
  const asked = [];
  const game = {
    async deckColorNoteIssues(names) {
      asked.push(...names);
      return names.filter((name) => name === 'Cryptic Spires')
        .map((cardName) => ({ cardName, error: 'Cryptic Spires needs the 2 colors circled for it' }));
    },
  };
  const issues = await submissionColorNoteIssues(game, {
    deck: ['Cryptic Spires', 'Cryptic Spires', 'Plains'], sideboard: ['Island'], commanders: [],
  });
  assert.deepEqual(asked, ['Cryptic Spires', 'Plains', 'Island']);
  assert.equal(describeColorNoteIssues(issues), 'Cryptic Spires needs the 2 colors circled for it.');
  assert.deepEqual(await submissionColorNoteIssues(null, { deck: ['Cryptic Spires'] }), []);
});
