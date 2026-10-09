import { arenaPermanentKind } from './mobile-arena.js';

// Pack consecutive occupied slots without erasing manually reserved empty slots.
// Coefficients scale with CSS card dimensions as the viewport changes.
export function compactLandOffsets(cards, positions) {
  const rows = new Map(), offsets = new Map();
  for (const card of cards) {
    const position = positions.get(String(card.id));
    if (!position) continue;
    if (!rows.has(position.row)) rows.set(position.row, []);
    rows.get(position.row).push({ id: String(card.id), column: position.column, land: arenaPermanentKind(card) === 'land' });
  }
  function pack(run) {
    const lands = run.filter(card => card.land).length;
    const pairs = run.reduce((sum, card, i) => sum + Number(Boolean(card.land && run[i - 1]?.land)), 0);
    let previousLands = 0, previousPairs = 0;
    run.forEach((card, i) => {
      if (card.land && run[i - 1]?.land) previousPairs++;
      offsets.set(card.id, { widthUnits: lands / 2 - previousLands - Number(card.land) / 2, gapUnits: pairs / 2 - previousPairs });
      if (card.land) previousLands++;
    });
  }
  for (const row of rows.values()) {
    row.sort((a, b) => a.column - b.column);
    let run = [];
    for (const card of row) {
      if (run.length && card.column !== run.at(-1).column + 1) { pack(run); run = []; }
      run.push(card);
    }
    pack(run);
  }
  return offsets;
}
