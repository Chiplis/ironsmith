import test from 'node:test';
import assert from 'node:assert/strict';
import { savedCardRecord, customCardOptions, saveCustomCard, listSavedCustomCards } from '../src/lib/custom-card-library.js';
import { registerCustomCardFlavor, customCardFlavorText } from '../src/lib/custom-card-flavor.js';

test('saved drafts preserve all faces and characteristics without retaining editable references', () => {
  const draft = { layout: 'transform_like', hasFuse: false, faces: [
    { name: 'Library Guardian', manaCost: '{2}{U}', cardTypes: ['Creature'], supertypes: ['Legendary'], subtypes: ['Wizard'], oracleText: 'Flying', power: '*', toughness: '4', loyalty: null, defense: null, colorIndicator: ['U'], artUrl: 'https://example.com/art.png', flavorText: 'A test.' },
    { name: 'Guardian Awakened', cardTypes: ['Planeswalker'], loyalty: 5, defense: 7, power: null, toughness: null },
  ] };
  const saved = savedCardRecord(draft);
  assert.deepEqual(saved.draft, draft);
  draft.faces[0].power = '9';
  assert.equal(saved.draft.faces[0].power, '*');
  assert.equal(savedCardRecord({ faces: [{name: ' library GUARDIAN '}] }).id, saved.id);
  assert.throws(() => savedCardRecord({faces: [{name: ' '}]}), /name/);
});

test('search finds both faces, ranks exact names first, and keeps same-name versions distinct', () => {
  const cards = [
    { id:'bundle:1', draft:{ faces:[{name:'Greater Guardian'}]} },
    { id:'bundle:2', draft:{ faces:[{name:'Guardian'}, {name:'Awakened'}]} },
    { id:'saved:guardian', savedAt:1, draft:{ faces:[{name:'Guardian'}]} },
  ];
  assert.deepEqual(customCardOptions(cards,'guardian').map(o => o.card.id), ['bundle:2','saved:guardian','bundle:1']);
  assert.equal(customCardOptions(cards,'AWAKENED')[0].card.id, 'bundle:2');
  assert.equal(customCardOptions(cards,'missing').length, 0);
});

test('user creations without Reddit sources can register flavor safely', () => {
  registerCustomCardFlavor([{draft:{faces:[{name:'Saved test',flavorText:'Kept flavor'}, {name:'Empty test'}]}}]);
  assert.equal(customCardFlavorText('Saved test'), 'Kept flavor');
  assert.equal(customCardFlavorText('Empty test'), '');
});

test('storage unavailability is reported and never pretends a save succeeded', async () => {
  await assert.rejects(saveCustomCard({faces:[{name:'Unsaved'}]}), /IndexedDB/);
  await assert.rejects(listSavedCustomCards(), /IndexedDB/);
});
