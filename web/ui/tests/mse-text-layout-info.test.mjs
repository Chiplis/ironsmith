import test from 'node:test';
import assert from 'node:assert/strict';
import { textLayoutInfo, dummyTextLayout } from '../src/lib/mse/text-layout-info.mjs';

test('layout hierarchy groups clauses, paragraphs and blocks at their native breaks', () => {
  const info = textLayoutInfo(['soft','hard','line','no'].map((kind,i)=>({width:10+i,top:5+i*10,height:8,break:kind})));
  assert.equal(info.lines.length,4);
  assert.equal(info.clauses.length,4);
  assert.equal(info.paragraphs.length,3);
  assert.equal(info.blocks.length,2);
  assert.equal(info.blocks[0].paragraphs.length,2);
  assert.equal(info.paragraphs[0].lines.length,2);
  assert.deepEqual(info.separators,[34]);
  assert.equal(info.height,43);
  assert.equal(info.lines[0].middle,9);
  assert.equal(dummyTextLayout().lines.length,1);
  assert.equal(dummyTextLayout().width,0);
});
