import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {compileDraftInput,validateMechanic} from '../src/lib/custom-mechanics.js';
const fixture=JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/msem-leitmotif-next.json',import.meta.url),'utf8'));
test('a complete resolution program is granted to the source instant and sorcery spells',()=>{
 const row=fixture.mechanics.find(row=>row.id==='msem-674');const card=row.representative_cards[0];
 const source=card.original_text.split('\n').filter(line=>!line.startsWith('Mana cost:')&&!line.startsWith('Type:')).join('\n').replaceAll('*','');
 const definitions=row.executable_definitions.map(validateMechanic);
 const face=compileDraftInput({layout:'normal',faces:[{name:card.name,typeLine:card.type_line,oracleText:source}]},definitions).faces[0];
 assert.match(face.oracleText,/^Instant and sorcery spells you control have custom keyword "leitmotif", and "If this spell was cast from your hand,/);
 assert.ok(face.oracleText.includes('when you next cast a spell that shares a color with the card exiled this way'));
 assert.ok(face.oracleText.includes('you may cast the card exiled this way without paying its mana cost.'));assert.equal(face.displayOracleText,'Instant and sorcery spells you control have leitmotif.');
 assert.equal(face.originalPrintedRulesText,source);
 assert.equal(card.source_faces[0].original_text,card.original_text);
 assert.equal(card.executable_text,['Mana cost: '+card.mana_cost,'Type: '+card.type_line,face.oracleText].join('\n'));
});
