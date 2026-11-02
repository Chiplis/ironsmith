import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {compileDraftInput} from '../../web/ui/src/lib/custom-mechanics.js';
import {executableDefinitions} from './source-definitions.mjs';
test('all sixteen preserved attain declarations identify their activation without altering their cost rider',()=>{
 const fixture=JSON.parse(fs.readFileSync(new URL('../../fixtures/custom_mechanics/msem-final-real-source-next-wave.json',import.meta.url),'utf8'));
 const rows=fixture.mechanics.filter(row=>row.keyword.toLowerCase()==='attain');assert.equal(rows.length,16);
 for(const row of rows){const defs=executableDefinitions(row);assert.equal(defs[0].abilityDesignation,'attain');
  const card=row.representative_cards[0];const face=card.source_faces[0];const original=face.original_text;
  const rules=original.split('\n').filter(line=>!/^Mana cost:|^Type:|^Power\/Toughness:/.test(line)).join('\n');
  const compiled=compileDraftInput({faces:[{name:face.name,oracleText:rules}]},defs).faces[0];
  assert.match(compiled.oracleText,/Keyword ability "attain"/);
  assert.match(compiled.oracleText,/Spend mana of at least three different colors to activate this ability/);
  // Display intentionally omits parenthesized reminder text. Compare the
  // complete retained printed display, including the original keyword/cost
  // and every sibling, rather than requiring markdown reminders in the UI.
  const printed=card.executable_faces[0].original_display_text;
  assert.equal(typeof printed,'string');
  assert.equal(compiled.displayOracleText,printed);
  assert.ok(compiled.displayOracleText.includes(rules.match(/\bAttain\s+(?:\{[^}]+\})+/)[0]));
  assert.match(original,/Spend at least three colors of mana/,'the full raw reminder remains preserved in the source record');
 }
});
