// Keyword auras for battlefield permanents (flying, hexproof, shroud, ward, indestructible, protection, attacking, blocking).
//
// A single hidden WebGL2 canvas renders every registered aura layer as a tile
// of one atlas; each tile is then copied into a small 2D canvas that sits
// inside its card. Living inside the card means the card's transforms, tap
// tilt, hover growth and stacking order all apply with no overlay math, and
// cards that cover each other occlude each other's auras correctly.

import { KeywordFxRenderer } from "./keyword-fx-renderer.js";
import { FX, BACK_FX } from "./flags.js";

export { FX, BACK_FX };

const ATLAS_WIDTH = 2048;
const TAU = Math.PI * 2;
const entries = new Set();
let canvas = null;
let renderer = null;
let unavailable = false;
let frame = null;
let lastPaint = 0;
let quality = "high";
const epoch = performance.now();

function reducedMotion() {
  return typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
}

function hexToRgb(hex) {
  const value = String(hex || "#fff1ce").replace("#", "");
  const n = parseInt(value.length === 3 ? value.replace(/./g, c => c + c) : value, 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}

function ensureRenderer() {
  if (renderer) return true;
  if (unavailable || typeof document === "undefined") return false;
  canvas = document.createElement("canvas");
  try { renderer = new KeywordFxRenderer(canvas); }
  catch (error) {
    unavailable = true;
    canvas = null;
    console.warn("[keyword fx] GPU auras unavailable", error);
    return false;
  }
  canvas.addEventListener("webglcontextlost", event => {
    event.preventDefault();
    renderer?.dispose();
    renderer = null;
  });
  canvas.addEventListener("webglcontextrestored", () => {
    try { renderer = new KeywordFxRenderer(canvas); schedule(); }
    catch { unavailable = true; }
  });
  renderer.ready.then(schedule, error => {
    unavailable = true;
    console.warn("[keyword fx] shader failed", error);
  });
  return true;
}

function schedule() {
  if (frame == null && entries.size && typeof requestAnimationFrame === "function") {
    frame = requestAnimationFrame(tick);
  }
}

function frameInterval() {
  if (reducedMotion()) return 1000;
  return quality === "low" ? 1000 / 24 : 1000 / 48;
}

function pixelScale() {
  const dpr = window.devicePixelRatio || 1;
  return quality === "low" ? 0.6 : Math.min(1.5, dpr);
}

// A flying card wanders on a slow, irregular path: incommensurate sine pairs
// per axis, phased by the card's seed, so no two cards move in step and the
// path never visibly repeats. Returns {x, y} with y as height above the table.
function flyingMotion(entry, time, still) {
  if (still) return { x: 0, y: 7 };
  const p = entry.spec.seed * TAU;
  return {
    x: 2.4 * Math.sin(time * 0.71 + p * 3.0) + 1.3 * Math.sin(time * 1.63 + p * 5.0),
    y: 7 + 1.9 * Math.sin(time * 1.13 + p) + 1.0 * Math.sin(time * 2.07 + p * 7.0),
  };
}

function tick(now) {
  frame = null;
  if (!entries.size || !renderer) return;
  if (now - lastPaint < frameInterval() && !Array.from(entries).some(entry => entry.dirty)) {
    schedule();
    return;
  }
  lastPaint = now;
  const still = reducedMotion();
  const time = still ? 0 : (now - epoch) / 1000;
  const scale = pixelScale();
  const viewW = window.innerWidth, viewH = window.innerHeight;

  // Reads first: sizes and visibility for every card.
  const jobs = [];
  for (const entry of entries) {
    const { card } = entry;
    if (!card.isConnected) continue;
    const width = card.offsetWidth, height = card.offsetHeight;
    if (!width || !height) continue;
    const rect = card.getBoundingClientRect();
    const pad = 40;
    if (rect.right < -pad || rect.bottom < -pad || rect.left > viewW + pad || rect.top > viewH + pad) continue;
    jobs.push({ entry, width, height });
  }

  // Pack every visible layer into the atlas.
  const tiles = [];
  let x = 0, y = 0, row = 0;
  for (const job of jobs) {
    const { entry, width, height } = job;
    const margin = Math.round(Math.min(26, Math.max(12, Math.min(width, height) * 0.2)));
    const size = [width + margin * 2, height + margin * 2];
    const w = Math.max(1, Math.round(size[0] * scale)), h = Math.max(1, Math.round(size[1] * scale));
    const motion = entry.spec.flags & FX.flying ? flyingMotion(entry, time, still) : { x: 0, y: 0 };
    const lift = motion.y, drift = motion.x;
    job.margin = margin; job.size = size; job.lift = lift; job.drift = drift;
    for (const [layer, target] of [[0, entry.back], [1, entry.front]]) {
      if (!target) continue;
      if (x + w > ATLAS_WIDTH) { x = 0; y += row; row = 0; }
      tiles.push({ x, y, w, h, size, card: [width, height], flags: entry.spec.flags, layer, seed: entry.spec.seed,
        lift, drift, protection: entry.spec.protection, target, job });
      x += w; row = Math.max(row, h);
    }
  }
  if (tiles.length) {
    renderer.ensureSize(ATLAS_WIDTH, y + row);
    renderer.render(tiles, time);
  }

  // Writes: lay out each aura canvas and copy its tile out of the atlas.
  for (const job of jobs) {
    const { entry, margin, size, lift, drift } = job;
    const key = `${margin}:${size[0]}:${size[1]}`;
    if (entry.layoutKey !== key) {
      entry.layoutKey = key;
      for (const target of [entry.back, entry.front]) {
        if (!target) continue;
        target.style.left = target.style.top = `${-margin}px`;
        target.style.width = `${size[0]}px`;
        target.style.height = `${size[1]}px`;
      }
    }
    const translate = lift || drift ? `${drift.toFixed(2)}px ${(-lift).toFixed(2)}px` : "";
    if (entry.translate !== translate) {
      entry.translate = translate;
      entry.card.style.translate = translate;
    }
    entry.dirty = false;
  }
  for (const tile of tiles) {
    const { target } = tile;
    if (target.width !== tile.w || target.height !== tile.h) { target.width = tile.w; target.height = tile.h; }
    const context = target.getContext("2d");
    context.clearRect(0, 0, tile.w, tile.h);
    context.drawImage(canvas, tile.x, tile.y, tile.w, tile.h, 0, 0, tile.w, tile.h);
  }
  schedule();
}

export function setKeywordFxQuality(value) {
  quality = value === "low" ? "low" : "high";
}

// Registers one card's aura canvases; returns {update(spec), dispose()}.
export function registerKeywordAura({ card, back, front, spec }) {
  if (!card || !front || !ensureRenderer()) return { update() {}, dispose() {} };
  const entry = { card, back, front, spec: normalize(spec), dirty: true, layoutKey: null, translate: "" };
  entries.add(entry);
  schedule();
  return {
    update(next) {
      entry.spec = normalize(next);
      entry.dirty = true;
      schedule();
    },
    dispose() {
      entries.delete(entry);
      if (entry.translate) card.style.translate = "";
    },
  };
}

function normalize(spec) {
  return {
    flags: spec?.flags | 0,
    seed: Number.isFinite(spec?.seed) ? spec.seed : 0,
    protection: hexToRgb(spec?.protection),
  };
}
