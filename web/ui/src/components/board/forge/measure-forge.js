import { clipRect, unionRect } from '../../../lib/forge-board-layout.js';

const SEATS = '[data-arena-owner], [data-player-drop-target]';
const ZONES = '.battlefield-row, .hand-zone-surface, [data-zone-id], [data-zone-anchor], .zone-pile-slot';
const CARD_FOOTPRINT = '.game-card, .battlefield-row-card, .battlefield-counter-rail, .battlefield-group-badge, .battlefield-token-side-badge';
const OBSTACLES = `${CARD_FOOTPRINT}, .zone-viewer, .battlefield-overlay-zone, .table-action-bar, .battlefield-panel-header, .battlefield-human-action-dock, .hand-reveal-shell, .zone-pile-slot, .table-middle-toolbars, .toolbar-brand, button, [role="button"], input, select, [data-player-target]`;
export const FORGE_MEASURE_SELECTOR = `${ZONES}, ${OBSTACLES}, ${SEATS}`;
export function createForgeMeasurer(root) {
  const ids = new WeakMap();
  let nextId = 0;
  return () => {
    const bounds = root.getBoundingClientRect();
    const width = bounds.width, height = bounds.height;
    const clips = new Map();
    function rectOf(el) {
      if (el.checkVisibility && !el.checkVisibility({ checkVisibilityCSS: true, checkOpacity: true })) return null;
      const r = el.getBoundingClientRect();
      if (!r.width || !r.height) return null;
      let visible = { left: r.left, top: r.top, right: r.right, bottom: r.bottom };
      for (let parent = el.parentElement; parent && parent !== root; parent = parent.parentElement) {
        if (!clips.has(parent)) {
          const style = getComputedStyle(parent);
          clips.set(parent, { rect: parent.getBoundingClientRect(), x: /auto|scroll|hidden|clip/.test(style.overflowX), y: /auto|scroll|hidden|clip/.test(style.overflowY) });
        }
        const clip = clips.get(parent);
        if (clip.x) { visible.left = Math.max(visible.left, clip.rect.left); visible.right = Math.min(visible.right, clip.rect.right); }
        if (clip.y) { visible.top = Math.max(visible.top, clip.rect.top); visible.bottom = Math.min(visible.bottom, clip.rect.bottom); }
      }
      return clipRect({ left: visible.left - bounds.left, top: visible.top - bounds.top, right: visible.right - bounds.left, bottom: visible.bottom - bounds.top }, width, height);
    }
    const zones = [];
    for (const el of root.querySelectorAll(ZONES)) {
      const bounds = rectOf(el);
      if (!bounds) continue;
      const battlefield = el.matches('.battlefield-row');
      // Containers with a battlefield child would duplicate its platform.
      if (!battlefield && el.querySelector('.battlefield-row')) continue;
      if (el.matches('[data-zone-anchor]') && !['library', 'command'].includes(el.dataset.zoneAnchor)) continue;
      if (el.matches('.zone-viewer, .battlefield-overlay-zone')) continue;
      const pile = el.querySelector('[data-zone-pile]');
      const type = battlefield ? 'battlefield' : el.matches('.hand-zone-surface') ? 'hand'
        : pile?.dataset.zonePile || el.dataset.zoneAnchor || el.dataset.zoneId || 'zone';
      const ownerNode = el.closest('[data-arena-owner], [data-zone-anchor-player], [data-player-drop-target]');
      const owner = pile?.dataset.zoneOwner ?? ownerNode?.dataset.arenaOwner
        ?? ownerNode?.dataset.zoneAnchorPlayer ?? ownerNode?.dataset.playerDropTarget ?? 'local';
      let rect = bounds;
      if (battlefield) {
        const cx = (bounds.left + bounds.right) / 2;
        rect = { left: Math.max(bounds.left, cx - 95), right: Math.min(bounds.right, cx + 95), top: bounds.top, bottom: Math.min(bounds.bottom, bounds.top + 110) };
        let occupied = null;
        for (const card of el.querySelectorAll(CARD_FOOTPRINT)) {
          const cardRect = rectOf(card);
          if (cardRect) occupied = occupied ? unionRect(occupied, cardRect) : cardRect;
        }
        if (occupied) rect = occupied;
        // Mobile uses separate creature/land/support lanes. Empty lanes should
        // not paint overlapping reserve platforms across its compact HUD.
        else if (el.closest('.mobile-mtga-battlefield-band')) continue;
      }
      if (!ids.has(el)) ids.set(el, `zone-${++nextId}`);
      zones.push({ key: `${ids.get(el)}:${owner}:${type}`, owner, type, battlefield, rect: clipRect(rect, width, height, battlefield ? 14 : 6) });
    }
    const obstacles = [...root.querySelectorAll(OBSTACLES)].map(rectOf).filter(Boolean);
    const cardShadows = [...root.querySelectorAll('.battlefield-row .game-card')].map(card => rectOf(card.querySelector('.arena-permanent') || card)).filter(Boolean).slice(0, 200);
    const seatsByOwner = new Map();
    for (const el of root.querySelectorAll(SEATS)) {
      const rect = rectOf(el);
      if (!rect) continue;
      const owner = el.dataset.arenaOwner ?? el.dataset.playerDropTarget;
      const previous = seatsByOwner.get(owner);
      seatsByOwner.set(owner, previous ? unionRect(previous, rect) : rect);
    }
    const seats = [...seatsByOwner].map(([owner, rect]) => ({ owner, rect }));
    return { width, height, zones, obstacles, cardShadows, seats };
  };
}
