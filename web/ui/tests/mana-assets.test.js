import test from 'node:test';
import assert from 'node:assert/strict';
import { access, readFile } from 'node:fs/promises';
import { manaAssets, manaGlyphs } from '../src/lib/mana-assets.generated.js';
import { counterDisplayLabel, manaSymbolUrl, counterSymbolUrl } from '../src/lib/mana-assets.js';

test('every supported symbol and glyph is packaged locally', async () => {
  for (const code of Object.keys(manaAssets)) {
    const url = manaSymbolUrl(code);
    assert.ok(url.startsWith('/mana/symbols/'));
    const svg = await readFile(new URL(`../public${url}`, import.meta.url), 'utf8');
    assert.match(svg, /<svg xmlns=/);
    assert.doesNotMatch(svg, /undefined|https?:\/\/(?!www.w3.org)/);
  }
  for (const glyph of manaGlyphs) await access(new URL(`../public/mana/svg/${glyph}.svg`, import.meta.url));
});

test('counter mappings cover named and keyword counters with safe fallback', () => {
  for (const kind of ['Plus One Plus One', '-1/-1', 'Lore', 'Loyalty', 'Charge', 'Shield', 'Stun', 'Flying', 'First Strike', 'Finality', 'Energy', 'Poison']) {
    assert.ok(counterSymbolUrl(kind), kind);
  }
  assert.equal(counterSymbolUrl('Unrecognized counter'), null);
  assert.equal(manaSymbolUrl('invalid'), null);
  assert.equal(manaSymbolUrl(' w/u/p '), '/mana/symbols/W-U-P.svg');
});

test('numeric power and toughness counters keep their exact visible labels', () => {
  for (const [kind, expected] of [
    ['Plus Two Plus Two', '+2/+2'],
    ['-2/-1', '-2/-1'],
    ['+0/+1', '+0/+1'],
  ]) {
    assert.equal(counterDisplayLabel(kind), expected);
  }
  assert.equal(counterDisplayLabel('charge'), null);
});
