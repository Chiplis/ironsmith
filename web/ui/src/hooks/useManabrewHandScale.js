import { useSyncExternalStore } from "react";

export const MANABREW_HAND_CARD_BASE = {
  cardW: 130,
  cardH: 182,
  containerH: 220,
};

export const MANABREW_HAND_FAN_PARAMS = {
  arcRadius: 900,
  maxArcDeg: 30,
  // The hover zoom is derived from the frame renderer's layout size rather than
  // fixed here; this is only the value CSS falls back to before the hand has
  // published its own. The lift stays fixed: hand cards scale from their bottom
  // edge, so a bigger zoom already grows upwards on its own, and lifting further
  // would slide the enlarged card out from under the pointer that opened it.
  hoverScale: 1.78,
  hoverLift: 58,
  neighborPush: 92,
  maxSpread: 90,
  minSpread: 38,
  spreadWidth: 900,
};

const REF_WIDTH = 1440;
const MIN_SCALE = 0.65;
const MAX_SCALE = 1.3;

function currentScale(fullscreen = false) {
  if (typeof window === "undefined") return 1;
  if (fullscreen) {
    return Math.max(MIN_SCALE, Math.min(1.5, (window.innerHeight - 80) / MANABREW_HAND_CARD_BASE.containerH, window.innerWidth / 700));
  }
  const scale = window.innerWidth / REF_WIDTH;
  return Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale));
}

function subscribe(callback) {
  if (typeof window === "undefined") return () => {};
  window.addEventListener("resize", callback);
  return () => window.removeEventListener("resize", callback);
}

export default function useManabrewHandScale(fullscreen = false) {
  return useSyncExternalStore(subscribe, () => currentScale(fullscreen), () => 1);
}

// The resting fan is sized off the window's width alone, but the hover zoom is
// also capped by its height, so that cap needs its own subscription to survive
// a window that only got shorter.
export function useViewportHeight() {
  return useSyncExternalStore(
    subscribe,
    () => (typeof window === "undefined" ? 0 : window.innerHeight),
    () => 0,
  );
}
