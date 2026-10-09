import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const corpus=JSON.parse(readFileSync(new URL('./frame-history-corpus.json',import.meta.url)));
test('historical audit pins diverse printings and both image identities',()=>{
 assert.equal(corpus.targetPerFamily,25);
 assert.equal(new Set(corpus.cases.map(c=>c.slug)).size,corpus.cases.length);
 for(const generation of ['1993','1997','2003','2015','future'])assert.ok(corpus.families.some(f=>f.key==='frame-'+generation));
 for(const f of corpus.families){
  assert.equal(f.ids.length,new Set(f.ids).size,f.key);
  assert.equal(f.selected,Math.min(corpus.targetPerFamily,f.distinctCards),f.key);
  for(const id of f.ids)assert.ok(corpus.cases.some(c=>c.id===id&&c.families.includes(f.key)),f.key+' '+id);
 }
 for(const c of corpus.cases){
  assert.ok(c.source.includes(c.id),c.slug);
  assert.ok(c.art?.includes(c.id),c.slug);
  assert.ok(!['art_series','planar','scheme','vanguard','token','double_faced_token','emblem'].includes(c.layout),c.slug);
 }
});
