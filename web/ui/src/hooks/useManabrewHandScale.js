import { useSyncExternalStore } from "react";

export const MANABREW_HAND_CARD_BASE = {
  cardW: 130,
  cardH: 182,
  containerH: 220,
};

export const MANABREW_HAND_FAN_PARAMS = {
  arcRadius: 900,
  maxArcDeg: 30,
  hoverScale: 1.8,
  hoverLift: 70,
  neighborPush: 78,
  maxSpread: 90,
  minSpread: 38,
  spreadWidth: 900,
};

const REF_WIDTH = 1440;
const MIN_SCALE = 0.65;
const MAX_SCALE = 1.3;

function currentScale() {
  if (typeof window === "undefined") return 1;
  const scale = window.innerWidth / REF_WIDTH;
  return Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale));
}

function subscribe(callback) {
  if (typeof window === "undefined") return () => {};
  window.addEventListener("resize", callback);
  return () => window.removeEventListener("resize", callback);
}

export default function useManabrewHandScale() {
  return useSyncExternalStore(subscribe, currentScale, () => 1);
}
