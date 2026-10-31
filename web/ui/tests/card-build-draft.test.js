import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {localCardBuildSource,cardBuildDraft} from '../src/lib/card-build-draft.js';
const load=async name=>cardBuildDraft(localCardBuildSource(JSON.parse(await readFile(new URL(`../public/cards/${name}.json`,import.meta.url)))));
test('local Yavimaya source keeps its name, land type, rules and art',async()=>{
 const draft=await load('yavimaya-cradle-of-growth');
 assert.equal(draft.faces[0].name,'Yavimaya, Cradle of Growth');
 assert.deepEqual(draft.faces[0].supertypes,['Legendary']);
 assert.deepEqual(draft.faces[0].cardTypes,['Land']);
 assert.equal(draft.faces[0].manaCost,'');
 assert.equal(draft.faces[0].oracleText,'Each land is a Forest in addition to its other land types.');
 assert.ok(draft.faces[0].artUrl);
});
test('local source preserves both linked faces, stats, indicators and split layouts',async()=>{
 const delver=await load('delver-of-secrets');
 assert.equal(delver.layout,'transform');
 assert.deepEqual(delver.faces.map(face=>[face.name,face.power,face.toughness]),[['Delver of Secrets','1','1'],['Insectile Aberration','3','2']]);
 assert.deepEqual(delver.faces[1].colorIndicator,['U']);
 assert.equal(delver.faces[1].oracleText,'Flying');
 const split=await load('fire-ice');
 assert.equal(split.layout,'split');
 assert.deepEqual(split.faces.map(face=>[face.name,face.manaCost,face.cardTypes]),[['Fire','{1}{R}',['Instant']],['Ice','{1}{U}',['Instant']]]);
});
test('incomplete image metadata cannot become an unnamed editable draft',()=>{
 assert.throws(()=>cardBuildDraft({oracle_text:'Rules'}),/name or type/);
});

test('Adventure import preserves an explicit Adventure authoring layout', async () => {
 const draft = await load('bonecrusher-giant');
 assert.equal(draft.layout, 'adventure');
 assert.equal(draft.faces.length, 2);
 assert.ok(draft.faces[1].subtypes.includes('Adventure'));
});

for (const layout of ['omen', 'prepare', 'flip', 'modal_dfc', 'transform', 'room', 'aftermath']) {
 test(`${layout} import retains its authoring identity`, () => {
  const card = {layout, card_faces: [{name:'Main',type_line:'Creature'}, {name:'Spell',type_line:layout === 'omen' ? 'Sorcery — Omen' : 'Sorcery'}]};
  assert.equal(cardBuildDraft(card).layout, layout);
 });
}

test('catalog multipart cards retain their mechanic instead of the source grouping fallback', async () => {
 for (const [name, layout] of [
  ['scavenger-regent','omen'], ['emeritus-of-ideation','prepare'],
  ['sejiri-shelter','modal_dfc'], ['akki-lavarunner','flip'],
  ['consign-oblivion','aftermath'], ['derelict-attic-widow-s-walk','room'],
 ]) {
  const draft = await load(name);
  assert.equal(draft.layout, layout, name);
  assert.equal(draft.faces.length, 2, name);
 }
 const fuse = await load('wear-tear');
 assert.equal(fuse.layout, 'split');
 assert.equal(fuse.hasFuse, true);
});
