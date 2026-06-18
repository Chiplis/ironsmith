// Resolves on-screen anchor rects for per-player zones (library, hand,
// graveyard, exile). Animation flights launch from / land on these rects.
import { getPlayerTargetRect } from "@/hooks/useCardPositions";

function visibleRect(el) {
  if (!el) return null;
  const rect = el.getBoundingClientRect();
  if (!rect || rect.width < 8 || rect.height < 8) return null;
  if (rect.bottom < 0 || rect.top > window.innerHeight) return null;
  return rect;
}

function queryVisible(selector) {
  for (const el of document.querySelectorAll(selector)) {
    const rect = visibleRect(el);
    if (rect) return rect;
  }
  return null;
}

function perspectiveHandDockRect() {
  const shell = document.querySelector(".hand-reveal-shell");
  const rect = visibleRect(shell);
  if (!rect) return null;

  const centerX = rect.left + (rect.width / 2);
  const width = Math.min(
    156,
    Math.max(96, Number(shell?.offsetWidth) || 0, rect.width * 0.28)
  );
  const height = 52;
  const bottom = Math.min(window.innerHeight - 8, rect.bottom);
  return new DOMRect(centerX - (width / 2), bottom - height, width, height);
}

export function getZoneAnchorRect(playerKey, zone, { isPerspective = false } = {}) {
  const key = String(playerKey ?? "");
  const zoneId = String(zone || "").toLowerCase();

  // The perspective player's hand is the hand dock at the bottom of the
  // screen, not the zone strip entry.
  if (zoneId === "hand" && isPerspective) {
    const handRect = perspectiveHandDockRect()
      || queryVisible(".hand-reveal-shell")
      || queryVisible(".hand-zone-surface");
    if (handRect) return handRect;
  }

  // An expanded zone panel, when open, is the most truthful anchor.
  const panelRect = queryVisible(
    `[data-zone-anchor-player="${key}"][data-zone-id="${zoneId}"], `
    + `[data-zone-anchor-player="${key}"] [data-zone-id="${zoneId}"]`
  );
  if (panelRect) return panelRect;

  // Otherwise the compact zone count chip ("Deck 53", "GY 4", ...).
  const chipRect = queryVisible(`[data-zone-anchor="${zoneId}"][data-zone-anchor-player="${key}"]`);
  if (chipRect) return chipRect;

  // Fall back to the player's HUD target, then to a viewport-edge guess.
  const playerRect = getPlayerTargetRect(key);
  if (playerRect) return playerRect;

  return new DOMRect(window.innerWidth / 2 - 40, isPerspective ? window.innerHeight - 80 : 40, 80, 56);
}

export function rectCenter(rect) {
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
}
