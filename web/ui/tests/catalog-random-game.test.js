import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { buildCatalogRandomGame, catalogDeckPosition } from '../src/lib/catalog-random-game.js';
import { createSeededRng } from '../src/lib/random-game.js';
const inventory = values => values.reduce((counts, name) => ({ ...counts, [name]: (counts[name] || 0) + 1 }), {});
const land = { name: 'Forest', types: ['Land'], permanent: true, manaValue: 0 };
const creature = { name: 'Bear', types: ['Creature'], permanent: true, manaValue: 2 };
const spell = { name: 'Spell', types: ['Sorcery'], permanent: false, manaValue: 1 };

test('conserves deck copies across zones without sideboard or invented lands', () => {
  const deck = { mainboard: [{ name: 'Forest', count: 24 }, { name: 'Bear', count: 20 }, { name: 'Spell', count: 16 }], sideboard: [{ name: 'Omniscience', count: 1 }] };
  const player = catalogDeckPosition(deck, [land, creature, spell], { rng: createSeededRng('copies'), name: 'Alice' });
  assert.deepEqual(inventory(Object.values(player.zones).flat()), { Forest: 24, Bear: 20, Spell: 16 });
  assert.equal(player.zones.battlefield.filter(name => name === 'Forest').length, 4);
  assert.equal(player.zones.hand.length, 7);
  assert.equal(player.zones.library.length, 43);
  assert.ok(player.zones.battlefield.every(name => name !== 'Spell'));
});

test('real lobby catalog produces a deterministic 1v1 position from complete supported decks', async () => {
  const fetchImpl = async url => {
    const pathname = new URL(url).pathname;
    try {
      const data = await readFile(new URL(`../public${pathname}`, import.meta.url), 'utf8');
      return { ok: true, json: async () => JSON.parse(data) };
    } catch { return { ok: false, status: 404 }; }
  };
  const options = { playerNames: ['Alice', 'Bob', 'Charlie'], minScore: .96, fetchImpl };
  const first = await buildCatalogRandomGame({ ...options, rng: createSeededRng('catalog-test') });
  const second = await buildCatalogRandomGame({ ...options, rng: createSeededRng('catalog-test') });
  assert.deepEqual(first, second);
  assert.equal(first.players.length, 2);
  const catalog = await (await fetchImpl('http://localhost/catalog/modern/index.json')).json();
  const decks = await Promise.all(catalog.decks.map(entry => fetchImpl(`http://localhost/catalog/modern/${entry.detail}`).then(response => response.json())));
  for (const player of first.players) {
    const cards = inventory(Object.values(player.zones).flat());
    assert.ok(decks.some(deck => JSON.stringify(Object.entries(inventory(deck.mainboard.flatMap(card => Array(card.count).fill(card.name)))).sort())
      === JSON.stringify(Object.entries(cards).sort())), 'all placed copies must match one complete catalog deck');
    assert.equal(player.zones.hand.length, 7);
    assert.ok(player.zones.library.length >= 40);
  }
});

test('skips missing deck details and uses a valid mirror match for a one-deck catalog', async () => {
  const fetchImpl = async url => {
    const pathname = new URL(url).pathname;
    const response = data => ({ ok: true, json: async () => data });
    if (pathname === '/catalog/modern/index.json') return response({ decks: [{ detail: 'missing.json' }, { detail: 'forest.json' }] });
    if (pathname === '/catalog/modern/missing.json') return { ok: false, status: 404 };
    if (pathname === '/catalog/modern/forest.json') return response({ mainboard: [{ name: 'Forest', count: 60 }] });
    if (pathname === '/catalog/modern/search-index.json') return response({});
    try { return response(JSON.parse(await readFile(new URL(`../public${pathname}`, import.meta.url), 'utf8'))); }
    catch { return { ok: false, status: 404 }; }
  };
  const payload = await buildCatalogRandomGame({ fetchImpl, rng: () => .9 });
  assert.equal(payload.players.length, 2);
  for (const player of payload.players) assert.deepEqual(inventory(Object.values(player.zones).flat()), { Forest: 60 });
});
