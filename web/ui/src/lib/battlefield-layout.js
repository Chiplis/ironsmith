export const PAPER_FRONT_LANES = ["creatures", "enchantments", "planeswalkers", "battles"];
export const PAPER_BACK_LANES = ["artifacts", "lands", "other"];
export const ALL_PAPER_LANES = [...PAPER_FRONT_LANES, ...PAPER_BACK_LANES];

export function normalizeBattlefieldLane(lane) {
  const normalized = String(lane || "").toLowerCase();
  return ALL_PAPER_LANES.includes(normalized) ? normalized : "other";
}

function normalizedCardTypes(card) {
  const explicitTypes = Array.isArray(card?.card_types)
    ? card.card_types.map((type) => String(type || "").trim().toLowerCase()).filter(Boolean)
    : [];
  if (explicitTypes.length > 0) return new Set(explicitTypes);

  const typeLine = String(card?.type_line || "")
    .split(/[—-]/, 1)[0]
    .toLowerCase();
  return new Set(
    ["artifact", "battle", "creature", "enchantment", "land", "planeswalker"]
      .filter((type) => new RegExp(`\\b${type}\\b`).test(typeLine))
  );
}

/** Mirrors the engine's BattlefieldLane precedence for a card not on the battlefield yet. */
export function battlefieldLaneForCard(card) {
  const types = normalizedCardTypes(card);
  if (types.has("enchantment")) return "enchantments";
  if (types.has("creature")) return "creatures";
  if (types.has("artifact")) return "artifacts";
  if (types.has("land")) return "lands";
  if (types.has("planeswalker")) return "planeswalkers";
  if (types.has("battle")) return "battles";
  return "other";
}

export function isPermanentCard(card) {
  const types = normalizedCardTypes(card);
  return ["artifact", "battle", "creature", "enchantment", "land", "planeswalker"]
    .some((type) => types.has(type));
}

function actionKind(action) {
  return String(action?.kind || action?.action_ref?.kind || "").trim().toLowerCase();
}

/** Returns the future battlefield lane only when the held action can put a permanent there. */
export function battlefieldPlacementForDrag(dragState) {
  if (!dragState || !Array.isArray(dragState.actions)) return null;
  const kinds = new Set(dragState.actions.map(actionKind));
  const card = dragState.card || null;

  if (kinds.has("move_battlefield")) {
    return { lane: battlefieldLaneForCard(card), kind: "move_battlefield" };
  }
  if (kinds.has("play_land")) {
    return { lane: battlefieldLaneForCard(card) === "other" ? "lands" : battlefieldLaneForCard(card), kind: "play_land" };
  }
  if (!kinds.has("cast_spell") || !isPermanentCard(card)) return null;
  return { lane: battlefieldLaneForCard(card), kind: "cast_spell" };
}

/** Resolve a viewport pointer to the visual grid cell used by the battlefield. */
export function battlefieldGridSlotAtPoint({
  x,
  y,
  left,
  top,
  width,
  rows,
  columns,
  cardWidth,
  cardHeight,
  gap = 0,
  overlap = 0,
}) {
  const numeric = [x, y, left, top, width, rows, columns, cardWidth, cardHeight, gap, overlap]
    .map(Number);
  if (numeric.some((value) => !Number.isFinite(value))) return null;
  const rowCount = Math.max(1, Math.floor(Number(rows)));
  const columnCount = Math.max(1, Math.floor(Number(columns)));
  const trackWidth = Math.max(1, Number(cardWidth) - Math.max(0, Number(overlap)));
  const rowHeight = Math.max(1, Number(cardHeight));
  const columnStride = trackWidth + Math.max(0, Number(gap));
  const rowStride = rowHeight + Math.max(0, Number(gap));
  const gridWidth = (columnCount * trackWidth) + ((columnCount - 1) * Math.max(0, Number(gap)));
  const gridLeft = Number(left) + Math.max(0, (Number(width) - gridWidth) / 2);
  const relativeX = Number(x) - gridLeft;
  const relativeY = Number(y) - Number(top);
  if (relativeX < 0 || relativeY < 0) return null;

  const column = Math.floor((relativeX + (Math.max(0, Number(gap)) / 2)) / columnStride) + 1;
  const row = Math.floor((relativeY + (Math.max(0, Number(gap)) / 2)) / rowStride) + 1;
  if (column < 1 || column > columnCount || row < 1 || row > rowCount) return null;
  return { row, column };
}

export function partitionBattlefieldCards(cards = []) {
  const frontCards = [];
  const backCards = [];

  for (const card of cards) {
    const lane = normalizeBattlefieldLane(card?.lane);
    if (PAPER_FRONT_LANES.includes(lane)) {
      frontCards.push(card);
    } else {
      backCards.push(card);
    }
  }

  return {
    frontCards,
    backCards,
    frontCount: frontCards.length,
    backCount: backCards.length,
  };
}

/** Keep surviving objects in their cells; only arrivals consume vacant cells. */
export function retainBattlefieldSlots(cards, previousLayout, { columns = 6, singleRow = false } = {}) {
  const maxCols = Math.max(1, Math.floor(columns));
  const gridPositionById = new Map();
  const occupied = new Set();
  const centerColumns = Array.from({ length: maxCols }, (_, index) => index + 1)
    .sort((a, b) => Math.abs(a - (maxCols + 1) / 2) - Math.abs(b - (maxCols + 1) / 2) || a - b);
  const key = (position) => `${position.row}:${position.column}`;
  const groupFor = (card) => singleRow ? "main"
    : PAPER_FRONT_LANES.includes(normalizeBattlefieldLane(card.lane)) ? "front" : "back";
  const previousByIdentity = new Map();
  const identities = (card) => (card.member_stable_ids?.length
    ? card.member_stable_ids : [card.stable_id ?? card.id]).map(String);
  for (const card of previousLayout?.orderedCards || []) {
    const position = previousLayout.gridPositionById.get(String(card.id));
    if (position) for (const id of identities(card)) previousByIdentity.set(id, position);
  }
  for (const card of cards) {
    const position = identities(card).map((id) => previousByIdentity.get(id)).find(Boolean);
    if (!position || occupied.has(key(position))) continue;
    gridPositionById.set(String(card.id), { ...position });
    occupied.add(key(position));
  }
  for (const card of cards) {
    if (gridPositionById.has(String(card.id))) continue;
    const groupId = groupFor(card);
    let row = groupId === "back" ? 2 : 1;
    let column = centerColumns.find((candidate) => !occupied.has(`${row}:${candidate}`));
    while (column == null) {
      if (singleRow) {
        column = maxCols + 1;
        while (occupied.has(`${row}:${column}`)) column += 1;
      } else {
        row += 2;
        column = centerColumns.find((candidate) => !occupied.has(`${row}:${candidate}`));
      }
    }
    const position = { row, column, groupId };
    gridPositionById.set(String(card.id), position);
    occupied.add(key(position));
  }
  const positions = [...gridPositionById.values()];
  return {
    orderedCards: cards,
    gridPositionById,
    rowCount: Math.max(singleRow ? 1 : 2, previousLayout?.rowCount || 0, ...positions.map((p) => p.row)),
    maxCols: Math.max(maxCols, previousLayout?.maxCols || 0, ...positions.map((p) => p.column)),
    signature: positions.map((p) => key(p)).join("|"),
  };
}
