import test from 'node:test';
import assert from 'node:assert/strict';
import { sortMseText as sort } from '../src/lib/mse/sort-text.mjs';

test('sort specifications cover mana order, cycles and preserved input order', () => {
  assert.equal(sort('cba'), 'abc');
  assert.equal(sort('cba', 'b'), 'b');
  assert.equal(sort('GR2W10', 'XYZ[0123456789](WUBRG)'), '210RGW');
  assert.equal(sort('cbbaba', '<ab>any()'), 'abcbba');
  assert.equal(sort('cababc', '[ba]c'), 'ababcc');
  assert.equal(sort('cabb', 'ordered(abc)'), 'abbc');
  assert.equal(sort('bXaYb', 'in_place(ab)'), 'aXbYb');
  assert.equal(sort('cabb', 'reverse_order(a b c)'), 'cbba');
});

test('compound and pattern sorts retain pinned native slot consumption semantics', () => {
  assert.equal(sort('aba', 'compound(ab)any()'), 'abaa');
  assert.equal(sort('b/a', 'pattern(./. ab)'), 'a/b');
  assert.equal(sort('[]', '\\[\\]'), '[]');
  assert.throws(() => sort('abc', 'cycle(ab'), { code: 'sort-syntax' });
  assert.throws(() => sort('abc', 'compound()'), { code: 'sort-syntax' });
});
