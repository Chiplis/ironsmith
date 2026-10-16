import test from 'node:test';
import assert from 'node:assert/strict';
import {rulesSourceCard,rulesSourceFrame} from '../src/lib/rules-source-frame.js';
import {cardFrameSnapshot} from '../src/lib/card-frame-snapshot.mjs';

for (const kind of ['initiative','monarch','ring','dungeon_room']) {
  test(`${kind} survives the frame snapshot and displays its live ability`, () => {
    const input = {name:'Secret Entrance',source_kind:kind,ability_text:'Search your library for a basic land card.',id:13};
    const snapshot = JSON.parse(cardFrameSnapshot(input));
    const card = rulesSourceCard(snapshot);
    assert.ok(rulesSourceFrame(card));
    assert.equal(card.name,input.name);
    assert.equal(card.oracle_text,input.ability_text);
    assert.equal(card.mana_cost,null);
    assert.equal(card.type_line,rulesSourceFrame(card).label);
  });
}
test('ordinary cards and unknown sources retain their normal frames', () => {
  for (const card of [{name:'The One Ring'}, {name:'Treasure',token:true}, {source_kind:'unknown'}]) {
    assert.equal(rulesSourceFrame(card),null);
    assert.equal(rulesSourceCard(card),card);
  }
});
