import test from 'node:test';
import assert from 'node:assert/strict';
import { compactLandOffsets } from '../src/lib/compact-land-layout.js';
function pack(types, columns = types.map((_, i) => i + 1)) {
 const cards = types.map((type_line, id) => ({id, type_line}));
 return compactLandOffsets(cards, new Map(cards.map(card => [String(card.id), {row:1, column:columns[card.id]}])));
}
test('adjacent lands pack at six pixels rather than retaining creature-width slots', () => {
 const result = pack(['Land','Land','Land']);
 const left = [...result.values()].map((offset, i) => i * 162 + 22 + offset.widthUnits * 44 + offset.gapUnits * 12);
 assert.equal(left[1] - left[0], 106);
 assert.equal(left[2] - left[1], 106);
});
test('mixed rows preserve the full-sized artifact and animated land footprints', () => {
 const result = pack(['Land','Land','Artifact','Land Creature']);
 const offsets = [...result.values()];
 const widths = [100,100,144,144];
 const left = offsets.map((offset,i) => i * 162 + (144-widths[i])/2 + offset.widthUnits*44 + offset.gapUnits*12);
 assert.equal(left[1]-left[0]-100,6);
 assert.equal(left[2]-left[1]-100,18);
 assert.equal(left[3]-left[2]-144,18);
});
test('manually reserved gaps are retained', () => {
 const result = pack(['Land','Land'], [1,4]);
 assert.deepEqual([...result.values()], [{widthUnits:0,gapUnits:0},{widthUnits:0,gapUnits:0}]);
});
