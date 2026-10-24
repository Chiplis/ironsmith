import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { prepareFormatDeck } from '../src/lib/prepare-format-deck.js';
import { parseDeckList, parseSideboardList } from '../src/lib/decklists.js';
const catalog = JSON.parse(await readFile(new URL('../src/lib/relay/format-catalog.generated.json', import.meta.url)));
const prepare = (deckText, format = 'modern', commanderText = '') => prepareFormatDeck(format, {deckText, commanderText}, catalog);

test('removes banned and unknown sideboard cards while preserving legal cards and the main deck', () => {
  const input = '60 Mountain\nSideboard\n1 Black Lotus\n2 Lightning Bolt\n1 Definitely Unknown Card';
  const result = prepare(input);
  assert.deepEqual(parseDeckList(result.deckText), parseDeckList(input));
  assert.deepEqual(parseSideboardList(result.deckText), ['Lightning Bolt', 'Lightning Bolt']);
  assert.equal(result.sideboardCount, 2);
  assert.deepEqual(result.removedSideboard.map(({name, count}) => ({name, count})), [
    {name: 'Black Lotus', count: 1}, {name: 'Definitely Unknown Card', count: 1},
  ]);
  assert.match(result.removedSideboard[0].reasons.join(' '), /not legal in Modern/);
  assert.match(result.removedSideboard[1].reasons.join(' '), /Unknown card/);
});

test('main-deck or commander problems still exclude a deck', () => {
  assert.equal(prepare('59 Mountain\n1 Black Lotus\nSideboard\n1 Black Lotus'), null);
  assert.equal(prepare('59 Mountain\nSideboard\n1 Mountain'), null);
  assert.equal(prepare('55 Mountain\n5 Lightning Bolt\nSideboard\n1 Island'), null);
  assert.equal(prepare('99 Plains', 'commander', '1 Mountain'), null);
});

test('retains only sideboard copies that fit copy limits and sideboard capacity', () => {
  assert.deepEqual(parseSideboardList(prepare('57 Mountain\n3 Lightning Bolt\nSideboard\n3 Lightning Bolt\n1 Island').deckText), ['Lightning Bolt', 'Island']);
  assert.deepEqual(parseSideboardList(prepare('59 Mountain\n1 Sol Ring\nSideboard\n1 Sol Ring\n1 Island', 'vintage').deckText), ['Island']);
  assert.equal(prepare('60 Mountain\nSideboard\n16 Island').sideboardCount, 15);
});

test('Commander drops the sideboard and preserves commanders', () => {
  const result = prepare('99 Plains\nSideboard\n1 Island', 'commander', '1 Isamaru, Hound of Konda');
  assert.equal(result.commanderText, '1 Isamaru, Hound of Konda');
  assert.equal(result.sideboardCount, 0);
});

test('format and unreleased changes re-prepare the original list without mutating it', () => {
  const texts = {deckText: '60 Mountain\nSideboard\n1 Lightning Bolt\n1 Preview Card', commanderText: ''};
  const data = {...catalog, cards: {...catalog.cards, 'preview card': {
    name: 'Preview Card', type: 'Creature', rules: '', colors: ['R'], unreleased: true, legalities: {},
  }}};
  const side = (format, allowUnreleased) => parseSideboardList(prepareFormatDeck(format, texts, data, {allowUnreleased}).deckText);
  assert.deepEqual(side('modern', false), ['Lightning Bolt']);
  assert.deepEqual(side('modern', true), ['Lightning Bolt', 'Preview Card']);
  assert.deepEqual(side('pioneer', false), []);
  assert.deepEqual(side('modern', true), ['Lightning Bolt', 'Preview Card']);
  assert.equal(texts.deckText, '60 Mountain\nSideboard\n1 Lightning Bolt\n1 Preview Card');
});


test('removal notices count rejected copies and clear when the original sideboard becomes legal', () => {
  const result = prepare('57 Mountain\n3 Lightning Bolt\nSideboard\n3 Lightning Bolt');
  assert.equal(result.removedSideboard.length, 1);
  assert.equal(result.removedSideboard[0].count, 2);
  assert.match(result.removedSideboard[0].reasons.join(' '), /at most 4/);
  const texts = {deckText: '60 Mountain\nSideboard\n1 Lightning Bolt'};
  assert.equal(prepareFormatDeck('pioneer', texts, catalog).removedSideboard.length, 1);
  assert.deepEqual(prepareFormatDeck('modern', texts, catalog).removedSideboard, []);
});
