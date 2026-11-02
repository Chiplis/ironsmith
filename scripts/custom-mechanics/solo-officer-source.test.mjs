import test from 'node:test';import assert from 'node:assert/strict';import {readFileSync} from 'node:fs';
import {materializeCard} from './cohort-input.mjs';
test('Solo Officer preserves actual display and only admits an ordinary named identity filter',()=>{
 const fixture=JSON.parse(readFileSync(new URL('../../fixtures/custom_mechanics/revolution-solo-officer-source.json',import.meta.url),'utf8'));const row=fixture.mechanics[0];assert.deepEqual(row.executable_definitions,[]);const card=row.representative_cards[0];const [face]=materializeCard(card,[],'revolution');
 assert.match(face.executable_text,/without custom keyword "undercover" that weren't cast enter face-down/);assert.ok(!face.custom_keyword_identities.includes('undercover'));assert.match(card.original_text,/without undercover/);assert.ok(!card.original_text.includes('custom keyword'));
});
