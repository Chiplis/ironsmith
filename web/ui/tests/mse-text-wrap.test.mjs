import test from 'node:test';
import assert from 'node:assert/strict';
import { wrapTextCharacters } from '../src/lib/mse/text-wrap.mjs';
const chars = text => [...text].map(c => ({ width: c === '\n' ? 0 : 10, height: 12, break: c === '\n' ? 'hard' : c === ' ' ? 'space' : 'maybe' }));
const lines = result => result.lines.map(l => [l.start,l.end,l.width,l.top,l.height,l.break]);

test('wrapping moves whole words, hides boundary spaces, and tracks positions', () => {
  assert.deepEqual(lines(wrapTextCharacters(chars('AB CD'), { width: 40 })), [[0,2,20,0,12,'wrap'],[3,5,20,12,12,'no']]);
  assert.deepEqual(lines(wrapTextCharacters(chars('ABCDE'), { width: 25 })), [[0,2,20,0,12,'wrap'],[2,4,20,12,12,'wrap'],[4,5,10,24,12,'no']]);
  assert.equal(wrapTextCharacters(chars('ABCDE'), { width: 25, stopIfTooLong: true }).fits, false);
});
test('explicit breaks, single-line overflow and vertical characters follow native rules', () => {
  assert.deepEqual(lines(wrapTextCharacters(chars('A\nB'), { width: 100, lineHeightHard: 2 })), [[0,1,10,0,12,'hard'],[2,3,10,24,12,'no']]);
  assert.equal(wrapTextCharacters(chars('AB CD'), { width: 20, multiLine: false }).lines.length, 1);
  assert.equal(wrapTextCharacters(chars('AB CD'), { width: 20, multiLine: false, stopIfTooLong: true }).fits, false);
  assert.deepEqual(lines(wrapTextCharacters(chars('AB'), { width: 20, vertical: true })), [[0,1,10,0,12,'wrap'],[1,2,10,12,12,'wrap'],[2,2,0,24,12,'no']]);
});
