import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const cases=JSON.parse(readFileSync(new URL('./card-frame-layout-cases.json',import.meta.url)));
const cards=JSON.parse(readFileSync(new URL('../../../cards.json',import.meta.url)));
test('pinned corpus covers every catalog layout, frame generation and frame effect',()=>{
  for(const key of ['layout','frame','frame_effects']) {
    const values=items=>new Set(items.flatMap(c=>c[key]||[]));
    const covered=values(cases);
    assert.deepEqual([...values(cards)].filter(value=>!covered.has(value)),[],key);
  }
  assert.equal(new Set(cases.map(c=>c.slug)).size,cases.length);
  for(const c of cases)assert.ok(c.source.includes(c.id),c.slug);
});
