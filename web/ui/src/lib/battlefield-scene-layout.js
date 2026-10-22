import { PAPER_FRONT_LANES, normalizeBattlefieldLane } from './battlefield-layout.js';
import { compactLandOffsets } from './compact-land-layout.js';
import { arenaPermanentKind } from './mobile-arena.js';

export const BATTLEFIELD_GROW_DELAY = 400;
export const BATTLEFIELD_GROW_MARGIN = 1.1;
export const BATTLEFIELD_MIN_WIDTH = 44;

const members = card => (card.member_stable_ids?.length ? card.member_stable_ids : [card.stable_id ?? card.id]).map(String);
const incarnations = card => (card.member_ids?.length ? card.member_ids : [card.id]).map(String);
const cellKey = p => `${p.row}:${p.column}`;

function attachmentHosts(cards) {
  const byMember = new Map(cards.flatMap(card => incarnations(card).map(id => [id, String(card.id)])));
  const direct = new Map();
  for (const card of cards) {
    const targets = [...new Set((card.attached_to_ids || []).map(id => byMember.get(String(id))))];
    if (targets.length === 1 && targets[0] && targets[0] !== String(card.id)) direct.set(String(card.id), targets[0]);
  }
  const hosts = new Map();
  for (const [id, target] of direct) {
    const visited = new Set([id]);
    let host = target;
    while (direct.has(host) && !visited.has(host)) { visited.add(host); host = direct.get(host); }
    if (!visited.has(host)) hosts.set(id, host);
  }
  return hosts;
}

/** Pure allocation: membership changes never repack surviving objects. Engine
 * groups remain authoritative: do not merge cards merely because names match. */
export function allocateBattlefieldScene(cards, previous, {
  columns = 6, singleRow = false, side = 'bottom', placementSlots = null,
} = {}) {
  const survivingColumns = (previous?.orderedCards || []).filter(old => cards.some(card => members(card).some(id => members(old).includes(id))))
    .map(card => previous.gridPositionById.get(String(card.id))?.column || 1);
  const frontCount = cards.filter(card => PAPER_FRONT_LANES.includes(normalizeBattlefieldLane(card.lane))).length;
  // Expand tracks before introducing another rank. Bound capacity so a token
  // army eventually scrolls instead of forcing a screen-wide microscopic row.
  const desiredColumns = Math.min(12, Math.max(frontCount, cards.length - frontCount));
  const maxCols = singleRow ? Math.max(1, cards.length, ...survivingColumns)
    : Math.max(1, columns, desiredColumns, ...survivingColumns);
  const occupied = new Set(), gridPositionById = new Map(), visualKeyById = new Map();
  const hosts = attachmentHosts(cards), visualKeys = new Set();
  const oldByMember = new Map();
  for (const card of previous?.orderedCards || []) {
    for (const id of members(card)) oldByMember.set(id, card);
  }
  const group = card => singleRow ? 'main'
    : PAPER_FRONT_LANES.includes(normalizeBattlefieldLane(card.lane)) ? 'front' : 'back';
  const firstRow = card => singleRow ? 1 : (group(card) === 'front') === (side !== 'top') ? 1 : 2;
  const claim = (card, position) => {
    if (!position || occupied.has(cellKey(position))) return false;
    gridPositionById.set(String(card.id), { ...position, groupId: group(card) });
    occupied.add(cellKey(position));
    return true;
  };
  // Explicit user placement has precedence, but cannot put two objects in one cell.
  for (const card of cards) {
    if (hosts.has(String(card.id))) continue;
    const remembered = [...members(card).map(id => `stable:${id}`), `object:${card.id}`]
      .map(id => placementSlots?.get?.(id)).find(Boolean);
    if (remembered && remembered.row >= 1 && remembered.column >= 1 && remembered.column <= maxCols) claim(card, remembered);
  }
  for (const card of cards) {
    const old = members(card).map(id => oldByMember.get(id)).find(Boolean);
    const oldPosition = old && previous.gridPositionById.get(String(old.id));
    if (!hosts.has(String(card.id)) && !gridPositionById.has(String(card.id)) && oldPosition?.groupId === group(card)) claim(card, oldPosition);
    // Stable identity remembers a slot; runtime membership distinguishes a blink
    // from a surviving group whose representative was replaced.
    const survives = old && incarnations(old).some(id => incarnations(card).includes(id));
    let visualKey = survives ? previous.visualKeyById.get(String(old.id)) : card.__battlefield_layout_hold_key || `object:${card.id}`;
    if (visualKeys.has(visualKey)) visualKey = `object:${card.id}:split:${incarnations(card).join(',')}`;
    visualKeys.add(visualKey);
    visualKeyById.set(String(card.id), visualKey);
  }
  const centerColumns = Array.from({ length: maxCols }, (_, i) => i + 1)
    .sort((a, b) => singleRow ? a - b : Math.abs(a - (maxCols + 1) / 2) - Math.abs(b - (maxCols + 1) / 2) || a - b);
  for (const card of cards) {
    if (hosts.has(String(card.id)) || gridPositionById.has(String(card.id))) continue;
    let row = firstRow(card);
    while (!centerColumns.some(column => claim(card, { row, column }))) row += singleRow ? 1 : 2;
  }
  const depths = new Map(), attachmentDepthById = new Map();
  for (const [id, host] of hosts) {
    const depth = (depths.get(host) || 0) + 1;
    depths.set(host, depth);
    attachmentDepthById.set(id, depth);
    gridPositionById.set(id, { ...gridPositionById.get(host) });
  }
  const rowCount = Math.max(singleRow ? 1 : 2, ...[...gridPositionById.values()].map(p => p.row));
  return { orderedCards: cards, gridPositionById, visualKeyById, maxCols, rowCount,
    attachmentDepthById, maxAttachmentDepth: Math.max(0, ...depths.values()),
    signature: cards.map(card => `${visualKeyById.get(String(card.id))}@${cellKey(gridPositionById.get(String(card.id)))}`).join('|') };
}

/** Solve in CSS pixels. No rendered card dimensions are inputs. */
export function fitBattlefieldScene(layout, viewport, options = {}) {
  const { baseWidth = 124, baseHeight = 108, gap = 18, top = 0, bottom = 0,
    preferredWidth = Infinity, minWidth = BATTLEFIELD_MIN_WIDTH, singleRow = false, alignStart = false } = options;
  const width = Math.max(1, viewport.width - (options.leftInset || 0)), height = Math.max(1, viewport.height - top - bottom);
  const preferred = Math.min(preferredWidth, width * (options.preferredWidthFraction ?? Infinity));
  const cols = layout.maxCols, rows = layout.rowCount;
  // Gaps compress before cards become unreadable. At the floor the board scrolls.
  const fittedGap = Math.max(2, Math.min(gap, (width - cols * minWidth) / Math.max(1, cols - 1)));
  const clusterWidth = baseWidth + (options.paddingX || 0) * 2 + (layout.maxAttachmentDepth || 0) * 22;
  const clusterHeight = baseHeight + (options.paddingY || 0) * 2 + (layout.maxAttachmentDepth || 0) * 16;
  const widthLimit = (width - (cols - 1) * fittedGap) / cols * baseWidth / clusterWidth;
  const heightLimit = (height - (rows - 1) * fittedGap) / rows * baseWidth / clusterHeight;
  const cardWidth = Math.max(Math.min(minWidth, width), Math.min(preferred, widthLimit, singleRow ? Infinity : heightLimit));
  const scale = cardWidth / baseWidth;
  return geometryForScale(layout, viewport, { ...options, baseWidth, baseHeight, gap: fittedGap, top, bottom, alignStart }, scale);
}

export function geometryForScale(layout, viewport, options, scale) {
  const { baseWidth = 124, baseHeight = 108, gap = 18, top = 0, bottom = 0, alignStart = false } = options;
  const cardWidth = baseWidth * scale, cardHeight = baseHeight * scale;
  const paddingX = (options.paddingX || 0) * scale, paddingY = (options.paddingY || 0) * scale;
  const trackWidth = cardWidth + paddingX * 2 + (layout.maxAttachmentDepth || 0) * 22 * scale;
  const trackHeight = cardHeight + paddingY * 2 + (layout.maxAttachmentDepth || 0) * 16 * scale;
  const gridWidth = layout.maxCols * trackWidth + (layout.maxCols - 1) * gap;
  const inset = Math.max(0, options.leftInset || 0);
  const left = inset + (alignStart ? 0 : Math.max(0, (viewport.width - inset - gridWidth) / 2));
  const gridHeight = layout.rowCount * trackHeight + (layout.rowCount - 1) * gap;
  const originTop = options.anchorBottom ? Math.max(top, viewport.height - bottom - gridHeight) : top;
  const cells = new Map(), poses = new Map();
  const landOffsets = options.compactLands ? compactLandOffsets(layout.orderedCards, layout.gridPositionById) : new Map();
  const landWidth = Math.max(28, Math.min(baseWidth * .8, baseHeight * .76)) * 1.2;
  for (let row = 1; row <= layout.rowCount; row++) for (let column = 1; column <= layout.maxCols; column++) {
    cells.set(`${row}:${column}`, { x: left + paddingX + (column - 1) * (trackWidth + gap), y: originTop + paddingY + (row - 1) * (trackHeight + gap), scale, width: cardWidth, height: cardHeight });
  }
  for (const card of layout.orderedCards) {
    const position = layout.gridPositionById.get(String(card.id));
    const cell = cells.get(cellKey(position)), depth = layout.attachmentDepthById?.get(String(card.id)) || 0;
    const land = options.compactLands && arenaPermanentKind(card) === 'land';
    const offset = landOffsets.get(String(card.id));
    const landX = (offset?.widthUnits || 0) * (baseWidth + (options.paddingX || 0) * 2 - landWidth) * scale
      + (offset?.gapUnits || 0) * (gap - 6) + (land ? (baseWidth - landWidth) * scale / 2 : 0);
    poses.set(layout.visualKeyById.get(String(card.id)), { ...cell,
      x: cell.x + depth * 22 * scale + landX, y: cell.y + depth * 16 * scale, depth,
      faceWidth: land ? landWidth : baseWidth,
      faceHeight: land ? landWidth / 1.2 + 9 : baseHeight,
      row: position.row, column: position.column, id: String(card.id) });
  }
  return { poses, cells, scale, cardWidth, cardHeight, gap,
    // Floating-point residue at an exact fit must not create a one-pixel
    // scroll range (and, on classic-scrollbar systems, another resize).
    width: gridWidth + inset > viewport.width + .01 ? gridWidth + inset : viewport.width,
    height: top + gridHeight + bottom > viewport.height + .01 ? top + gridHeight + bottom : viewport.height };
}

/** Retarget from the current displayed pose, never the previous destination. */
export function advanceBattlefieldPose(current, target, elapsed, reduced = false) {
  if (!current || reduced) return { ...target };
  const alpha = 1 - Math.exp(-Math.min(64, Math.max(0, elapsed)) / 65);
  const next = { ...target };
  for (const key of ['x', 'y', 'scale']) next[key] = Math.abs(target[key] - current[key]) < (key === 'scale' ? .0005 : .1)
    ? target[key] : current[key] + (target[key] - current[key]) * alpha;
  return next;
}

export function battlefieldSceneCellAtPoint(geometry, x, y) {
  for (const [key, cell] of geometry?.cells || []) {
    if (x >= cell.x && x <= cell.x + cell.width && y >= cell.y && y <= cell.y + cell.height) {
      const [row, column] = key.split(':').map(Number);
      return { row, column };
    }
  }
  return null;
}

/** Outline the visible compact face, including its characteristic-specific inset. */
export function battlefieldPlacementOutline(pose, card, compact = false) {
  const scale = pose.scale;
  let x = pose.x, y = pose.y;
  let width = (pose.faceWidth == null ? pose.width : pose.faceWidth * scale);
  let height = (pose.faceHeight == null ? pose.height : pose.faceHeight * scale);
  let borderRadius = '0px';
  if (compact) {
    const kind = arenaPermanentKind(card);
    const insetX = kind === 'planeswalker' ? width * .17 : 2 * scale;
    const insetY = kind === 'battle' ? height * .15 : 2 * scale;
    x += insetX; y += insetY; width -= insetX * 2; height -= insetY * 2;
    borderRadius = card?.is_token || card?.token ? `48% 48% ${4 * scale}px ${4 * scale}px`
      : kind === 'planeswalker' ? `${13 * scale}px ${13 * scale}px ${3 * scale}px ${3 * scale}px`
      : `${(kind === 'land' ? 2 : 4) * scale}px`;
  }
  return { x, y, width, height, borderRadius };
}
