import test from 'node:test';
import assert from 'node:assert/strict';
import {previewCustomDraft} from '../src/lib/custom-card-library.js';
import {activeCustomMechanics} from '../src/lib/custom-mechanics.js';
const draft={layout:'single',faces:[{name:'Custom Card',manaCost:'{B/P}',cardTypes:['Sorcery'],oracleText:'Destroy target creature.'}]};
test('ordinary card-builder preview returns a successful compilation',async()=>{
 const result=await previewCustomDraft(draft,async input=>({canCreate:true,faces:input.faces}));
 assert.equal(result.preview.canCreate,true);
 assert.equal(result.preview.faces[0].oracleText,'Destroy target creature.');
});
test('imported preview definitions remain scoped to the card',async()=>{
 const before=activeCustomMechanics();
 const result=await previewCustomDraft({...draft,customMechanicsScope:'card',customMechanics:[{keyword:'kindle',reminder:'Draw a card.'}],faces:[{...draft.faces[0],oracleText:'Kindle'}]},async input=>({canCreate:true,faces:input.faces}));
 assert.ok(result.preview.canCreate);
 assert.deepEqual(activeCustomMechanics(),before);
 assert.equal(result.preview.customMechanics[0].keyword,'kindle');
});
