import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdtempSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {execFileSync} from 'node:child_process';
const materializer=fileURLToPath(new URL('../materialize-custom-mechanic-cohort.mjs',import.meta.url));
const source=JSON.parse(readFileSync(new URL('../../fixtures/custom_mechanics/revolution-solo-officer-source.json',import.meta.url),'utf8'));
function run(fixture){
 const directory=mkdtempSync(join(tmpdir(),'reviewed-ordinary-source-'));
 try{const path=join(directory,'candidate.json');writeFileSync(path,JSON.stringify(fixture));execFileSync(process.execPath,[materializer,path],{stdio:'pipe'});return JSON.parse(readFileSync(path,'utf8'));}
 finally{rmSync(directory,{recursive:true,force:true});}
}
test('reviewed actual ordinary source materializes without a keyword definition and preserves source evidence',()=>{
 const fixture=structuredClone(source);assert.deepEqual(fixture.mechanics[0].executable_definitions,[]);
 const original=structuredClone(fixture.mechanics[0].representative_cards[0]);
 const result=run(fixture).mechanics[0].representative_cards[0];
 assert.equal(result.rewrite_status,'frontend_materialized_unverified');assert.ok(result.executable_faces.length);
 assert.match(result.executable_text,/without custom keyword "undercover"/i);
 assert.equal(result.original_text,original.original_text);assert.deepEqual(result.source_faces,original.source_faces);
 assert.deepEqual(result.custom_keyword_identities,[],'ordinary correction does not grant a custom keyword');
});
test('no-author-data pending and excluded rules remain unchanged',()=>{
 const fixture=structuredClone(source);const row=fixture.mechanics[0];
 const pending=structuredClone(row);pending.id+='-pending';
 for(const card of pending.representative_cards){for(const face of card.source_faces)delete face.reviewed_source_corrections;card.rewrite_status='pending_source_review';card.executable_text=null;}
 const excluded=structuredClone(row);excluded.id+='-excluded';excluded.support_status='unsupported_requires_new_rules';
 fixture.mechanics=[pending,excluded];const expected=structuredClone(fixture);assert.deepEqual(run(fixture),expected);
});
test('admitting reviewed metadata does not bypass exact correction validation',()=>{
 const fixture=structuredClone(source);const card=fixture.mechanics[0].representative_cards[0];
 card.source_faces[0].reviewed_source_corrections[0].source_context='Unrelated source sentence.';
 const original=structuredClone(card);const result=run(fixture).mechanics[0].representative_cards[0];
 assert.equal(result.rewrite_status,'frontend_binding_failed');assert.equal(result.executable_text,null);
 assert.equal(result.original_text,original.original_text);assert.deepEqual(result.source_faces,original.source_faces);
 assert.match(result.rewrite_error,/Source correction needs exact reviewed context and evidence/);
});
