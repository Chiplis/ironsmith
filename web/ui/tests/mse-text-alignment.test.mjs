import test from 'node:test';
import assert from 'node:assert/strict';
import { alignTextLine } from '../src/lib/mse/text-alignment.mjs';
import { parseAlignment } from '../src/lib/mse/alignment.mjs';
const line = { start: 0, end: 3, endOrSoft: 3, positions: [0,10,20,30], width: 30, break: 'wrap' };
const chars = [{break:'maybe'},{break:'space'},{break:'maybe'}];
const align = (name, overrides = {}, source = line) => alignTextLine(source, chars, { alignment: parseAlignment(name), width: 60, ...overrides });

test('horizontal alignment supports centering, right alignment and mask bounds', () => {
  assert.deepEqual(align('center').positions, [15,25,35,45]);
  assert.deepEqual(align('right').positions, [30,40,50,60]);
  assert.deepEqual(align('right',{maskRight:50}).positions, [20,30,40,50]);
  assert.deepEqual(align('center',{width:20}).positions, [-5,5,15,25]);
  assert.deepEqual(line.positions, [0,10,20,30]);
});
test('justification distributes space according to native break and overflow conditions', () => {
  assert.deepEqual(align('justify-all').positions, [0,20,40,60]);
  assert.deepEqual(align('justify').positions, [0,10,50,60]);
  assert.equal(align('justify',{},{...line,break:'hard'}).justifying, false);
  assert.equal(align('justify force',{},{...line,break:'hard'}).justifying, true);
  assert.equal(align('justify if-overflow').justifying, false);
  assert.deepEqual(align('right shrink-overflow',{width:20}).positions, line.positions);
});

import { alignTextParagraph, alignTextLines, textContentHeight } from '../src/lib/mse/text-alignment.mjs';
const paragraph = [
  { ...line, top: 0, height: 10, break: 'wrap' },
  { ...line, top: 10, height: 10, break: 'no' },
];
test('vertical alignment and spacing expansion preserve native paragraph rules', () => {
  const opts = { width: 60, height: 60, y: 5, alignment: parseAlignment('left middle') };
  assert.deepEqual(alignTextParagraph(paragraph, chars, opts).map(l => l.top), [25,35]);
  assert.deepEqual(alignTextParagraph(paragraph, chars, { ...opts, alignment: parseAlignment('bottom') }).map(l => l.top), [45,55]);
  assert.deepEqual(alignTextParagraph(paragraph, chars, { ...opts, lineHeightSoftMax: 2 }).map(l => l.top), [20,40]);
  assert.deepEqual(alignTextParagraph(paragraph, chars, { ...opts, height: 25, lineHeightSoftMax: 3 }).map(l => l.top), [5,20]);
  assert.deepEqual(paragraph.map(l => l.top), [0,10]);
  assert.equal(textContentHeight([...paragraph, { ...line, top: 20, height: 0 }]), 20);
});
test('fixed-height paragraphs align in separate slots', () => {
  const input = [{ ...paragraph[0], break: 'hard' }, { ...paragraph[1], break: 'no' }];
  const result = alignTextLines(input, chars, { width: 60, height: 100, y: 5, paragraphHeight: 30, alignment: parseAlignment('middle') });
  assert.deepEqual(result.map(l => l.top), [15,45]);
});
