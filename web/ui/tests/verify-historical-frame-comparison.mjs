// Check captured production DOM colours against independently reviewed scans.
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
const root=new URL('../test-results/historical-frames/',import.meta.url);
const read=async p=>JSON.parse(await readFile(p,'utf8'));
const expected=await read(new URL('./fixtures/historical-frames/expected-ink.json',import.meta.url));
const before=await read(new URL('baseline/results.json',root)),after=await read(new URL('reworked/results.json',root));
const b=new Map(before.map(r=>[r.slug,r])),a=new Map(after.map(r=>[r.slug,r]));
assert.equal(a.size,87);assert.deepEqual([...a.keys()].sort(),[...b.keys()].sort());
const white='rgb(255, 255, 255)',black='rgb(0, 0, 0)';
for(const [slug,sections]of Object.entries({...expected.corrected,...expected.englishFallbackCorrections})){
 assert.equal(b.get(slug)?.mode,'masked',slug);assert.equal(a.get(slug)?.mode,'masked',slug);
 for(const section of sections){assert.deepEqual(b.get(slug).actual[section],[black],`${slug} ${section} reproduces the old error`);assert.deepEqual(a.get(slug).actual[section],[white],`${slug} ${section} matches the source scan`);}
}
for(const [slug,sections]of Object.entries(expected.darkControls))for(const section of sections)assert.deepEqual(a.get(slug).actual[section],[black],`${slug} ${section} stays dark`);
for(const r of after){
 assert.ok(!r.error&&!r.errors.length,r.slug);assert.equal(r.mode,b.get(r.slug).mode,r.slug+' mode');
 assert.equal(r.geometry,b.get(r.slug).geometry,r.slug+' scan geometry');
 assert.deepEqual(r.actual.rules,b.get(r.slug).actual.rules,r.slug+' rules ink');
}
const summary={cards:after.length,sets:new Set(after.map(r=>r.set)).size,correctedCards:Object.keys(expected.corrected).length,correctedSections:Object.values(expected.corrected).reduce((n,s)=>n+s.length,0),masked:after.filter(r=>r.mode==='masked').length,englishFallbackColorCorrections:Object.keys(expected.englishFallbackCorrections).length,existingSyntheticFallbacks:after.filter(r=>r.mode==='custom').length,errors:0};
await writeFile(new URL('summary.json',root),JSON.stringify(summary,null,2)+'\n');console.log(summary);
