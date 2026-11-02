import test from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { validateMechanic } from '../../web/ui/src/lib/custom-mechanics.js';
import { executableDefinitions, definitionsForSource } from './source-definitions.mjs';
import { materializeCard, renameSourceKeywords } from './cohort-input.mjs';
const directory = new URL('../../fixtures/custom_mechanics/', import.meta.url);
test('all authored executable cohort definitions avoid official names and forms', () => {
  const clashes = [];
  let checked = 0;
  for (const file of readdirSync(directory).filter(file => file.endsWith('.json') && !file.endsWith('-source.json'))) {
    const fixture = JSON.parse(readFileSync(new URL(file, directory)));
    for (const mechanic of fixture.mechanics || []) for (const definition of executableDefinitions(mechanic)) {
      checked++;
      try { validateMechanic(definition); }
      catch (error) { if (/official keyword/.test(error.message)) clashes.push(`${file}:${mechanic.id}: ${error.message}`); }
    }
  }
  assert.ok(checked > 100, 'audit must cover the authored corpus');
  assert.deepEqual(clashes, []);
});
test('source renaming expands the new identity and preserves original evidence', () => {
  const reminder = 'Create a 1/1 white Soldier creature token.';
  const card = {name:'Soldier maker', original_text:`Type: Sorcery\nRecruit twice. (${reminder})`};
  const before = structuredClone(card);
  const definitions = definitionsForSource(card, [{keyword:'Muster ranks', sourceKeyword:'Recruit', sourceReminder:reminder, reminder}], new Map());
  const [face] = materializeCard(card, definitions);
  assert.match(face.executable_text, /Create a 1\/1 white Soldier creature token/);
  assert.deepEqual(face.custom_keyword_identities, ['muster ranks']);
  assert.doesNotMatch(face.executable_text, /\brecruit\b/i);
  assert.deepEqual(card, before);
});
test('paid source renaming leaves a bare official ability untouched', () => {
  const sourceReminder = '{2}{W}: Exile this permanent, then return it to the battlefield transformed.';
  const definition = {sourceKeyword:'Ascend', keyword:'Rise anew', sourceReminder, template:'{Cost}',
    reminder:'{Cost}: Exile this permanent, then return it to the battlefield transformed.', officialCostDeclaration:true};
  const source = `Ascend\nAscend {2}{W}\nAscend {2}{W} (${sourceReminder})\nAscendant`;
  assert.equal(renameSourceKeywords(source, [definition]),
    `Ascend\nAscend {2}{W}\nrise anew {2}{W} (${sourceReminder})\nAscendant`);
});
