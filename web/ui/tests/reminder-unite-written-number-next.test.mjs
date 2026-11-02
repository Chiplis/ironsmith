import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {materializeCard} from '../../../scripts/custom-mechanics/cohort-input.mjs';
const fixture=JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/revolution-abilities.json',import.meta.url),'utf8'));

test('all nineteen exact Unite sources bind their full declarations and preserve source text',()=>{
  const rows=fixture.mechanics.filter(row=>['rev-unite-108','rev-unite-109','rev-unite-110'].includes(row.id));
  assert.equal(rows.length,3);
  const keys=new Set();
  for(const row of rows)for(const card of row.representative_cards){
    const original=JSON.stringify(card);
    const faces=materializeCard(card,row.executable_definitions,'revolution');
    assert.equal(JSON.stringify(card),original);
    assert.ok(faces.length>0);
    const text=faces.map(face=>face.executable_text).join('\n');
    const count=row.id==='rev-unite-109'?'2':row.id==='rev-unite-110'?'3':'one or more';
    assert.ok(text.includes(`tap ${count} untapped creatures you control`),`${card.name}: ${text}`);
    assert.ok(text.includes('perform the unite action with creatures tapped this way'),`${card.name}: ${text}`);
    assert.doesNotMatch(text,/^Unite with /im);
    keys.add(card.source_card_key||card.id);
  }
  assert.equal(keys.size,19);
});
