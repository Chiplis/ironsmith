// Light effects for the card stack (CardStackPile): comets that carry a spell
// or ability onto the stack, the charge and sigil a source shows as it fires,
// and the sparks, embers and rings a card leaves behind. One fixed canvas,
// drawn additively, alive only while an effect is.

import { StackFxRenderer } from "./stack-fx-renderer.js";

const TAU = Math.PI * 2;
let canvas = null;
let renderer = null;
let frame = null;
let unavailable = false;
const effects = new Set();

export function prefersReducedMotion() {
  if (typeof window === "undefined") return true;
  return Boolean(window.matchMedia("(prefers-reduced-motion: reduce)").matches || document.hidden);
}

function stopEffects() {
  if (frame != null) cancelAnimationFrame(frame);
  frame = null;
  for (const effect of effects) effect.cancel?.();
  effects.clear();
  if (canvas) canvas.style.display = "none";
}

function ensureCanvas() {
  if (renderer) return true;
  if (unavailable || typeof document === "undefined") return false;
  canvas = document.createElement("canvas");
  canvas.className = "stack-fx-canvas";
  canvas.setAttribute("aria-hidden", "true");
  try { renderer = new StackFxRenderer(canvas); }
  catch (error) {
    unavailable = true;
    canvas = null;
    console.warn("[stack fx] GPU effects unavailable", error);
    return false;
  }
  document.body.appendChild(canvas);
  const resize = () => {
    if (!renderer) return;
    renderer.resize(window.innerWidth, window.innerHeight, Math.min(2, window.devicePixelRatio || 1));
    canvas.style.width = `${window.innerWidth}px`;
    canvas.style.height = `${window.innerHeight}px`;
  };
  resize();
  window.addEventListener("resize", resize);
  canvas.addEventListener("webglcontextlost", event => {
    event.preventDefault();
    stopEffects();
    renderer = null;
    unavailable = true;
  });
  canvas.addEventListener("webglcontextrestored", () => {
    try { renderer = new StackFxRenderer(canvas); unavailable = false; resize(); }
    catch (error) { console.warn("[stack fx] GPU restore failed", error); }
  });
  return true;
}

function start() {
  if (frame != null || !ensureCanvas()) return;
  canvas.style.display = "block";
  frame = requestAnimationFrame(tick);
}

function tick(now) {
  renderer.begin();
  // Only effect geometry and moving-target anchors remain on the CPU. Every
  // particle's position, spin, fading and glow are evaluated by GPU shaders.
  for (const effect of effects) {
    try { if (effect.step(now) === false) effects.delete(effect); }
    catch (error) {
      effects.delete(effect);
      effect.cancel?.();
      console.error("[stack fx] effect failed", error);
    }
  }
  renderer.render(now);
  if (effects.size === 0 && now >= renderer.until) {
    frame = null;
    canvas.style.display = "none";
    return;
  }
  frame = requestAnimationFrame(tick);
}

function emit(particle) { renderer.emit(particle); }

function center(rectOrPoint) {
  if (!rectOrPoint) return null;
  if (typeof rectOrPoint === "function") return center(rectOrPoint());
  if ("x" in rectOrPoint && !("width" in rectOrPoint)) return rectOrPoint;
  return { x: rectOrPoint.left + rectOrPoint.width / 2, y: rectOrPoint.top + rectOrPoint.height / 2 };
}

const clamp01 = (value) => Math.max(0, Math.min(1, value));
const easeInOut = (t) => (t < 0.5 ? 4 * t * t * t : 1 - ((-2 * t + 2) ** 3) / 2);
const easeOut = (t) => 1 - (1 - t) ** 3;
const easeIn = (t) => t * t * t;

// ---- comets ----------------------------------------------------------------

function bezier(a, c, b, t) {
  const u = 1 - t;
  return { x: u * u * a.x + 2 * u * t * c.x + t * t * b.x, y: u * u * a.y + 2 * u * t * c.y + t * t * b.y };
}

function controlPoint(a, b, bow) {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const length = Math.hypot(dx, dy) || 1;
  // Arc upward on screen whichever way the comet flies.
  const sign = dx >= 0 ? -1 : 1;
  return {
    x: (a.x + b.x) / 2 + (-dy / length) * length * bow * sign,
    y: (a.y + b.y) / 2 + (dx / length) * length * bow * sign,
  };
}

function drawHead(x, y, radius, rgb, alpha = 1) {
  renderer.glow(x, y, radius * 4, rgb, alpha);
}

function drawLightning(points, rgb, alpha) {
  const jagged = points.map((point, index) => index === 0 || index === points.length - 1
    ? point : { x: point.x + (Math.random() - 0.5) * 14, y: point.y + (Math.random() - 0.5) * 14 });
  for (const [width, color, opacity] of [[7, rgb, alpha * 0.35], [3, rgb, alpha], [1.2, "255, 255, 255", alpha]]) {
    for (let i = 1; i < jagged.length; i++) renderer.line(jagged[i - 1], jagged[i], width, color, opacity);
  }
}

/**
 * A comet from `from` to `to` (rects, points, or functions returning either;
 * `to` is re-read every frame so a moving target is still hit).
 * style: "star" (spell), "bolt" (activated ability), "wisp" (trigger),
 *        "spiral" (cycling), "spark" (a quick hop between stack cards).
 * Resolves when it lands.
 */
export function launchComet({
  from,
  to,
  rgb = "183, 156, 255",
  accentRgb = null,
  style = "star",
  duration = null,
  durationScale = 1,
  size = 1,
} = {}) {
  const origin = center(from);
  const end0 = center(to);
  if (!origin || !end0 || prefersReducedMotion() || !ensureCanvas()) return Promise.resolve();
  const distance = Math.hypot(end0.x - origin.x, end0.y - origin.y);
  const total = (duration ?? {
    star: 420 + distance * 0.42,
    bolt: 230 + distance * 0.16,
    wisp: 620 + distance * 0.55,
    spiral: 760 + distance * 0.4,
    spark: 240,
  }[style] ?? 600) * durationScale;
  const bow = { star: 0.22, bolt: 0.04, wisp: 0.3, spiral: 0.18, spark: 0.28 }[style] ?? 0.2;
  const sparkle = accentRgb || rgb;
  // The trail is sampled along the flight path at fixed steps of progress,
  // however few frames arrive, and keeps a fixed share of the flight.
  const trail = [];
  const trailSpan = { star: 0.5, bolt: 0.18, wisp: 0.45, spiral: 0.42, spark: 0.4 }[style] ?? 0.45;
  const pointAt = (t, end) => {
    let point = bezier(origin, controlPoint(origin, end, bow), end, t);
    if (style === "wisp") {
      const dx = end.x - origin.x;
      const dy = end.y - origin.y;
      const length = Math.hypot(dx, dy) || 1;
      const wobble = Math.sin(t * Math.PI * 3) * 22 * (1 - t);
      point = { x: point.x + (-dy / length) * wobble, y: point.y + (dx / length) * wobble };
    } else if (style === "spiral") {
      // One loop-the-loop in the middle of the flight.
      const loop = Math.max(0, Math.min(1, (t - 0.25) / 0.45));
      const angle = loop * TAU;
      const radius = 34 * Math.sin(loop * Math.PI);
      point = { x: point.x + Math.sin(angle) * radius, y: point.y - (1 - Math.cos(angle)) * radius };
    }
    return point;
  };
  const progress = (raw) => (style === "bolt" ? easeIn(raw) * 0.6 + raw * 0.4 : easeInOut(raw));
  let lastRaw = 0;
  let previousT = 0;
  const began = performance.now();
  return new Promise((resolve) => {
    let landed = false;
    effects.add({
      cancel: () => resolve(),
      step(now) {
        const raw = clamp01((now - began) / total);
        const end = center(to) || end0;
        for (let sample = lastRaw + 0.012; sample < raw; sample += 0.012) {
          trail.push({ t: progress(sample), ...pointAt(progress(sample), end) });
        }
        lastRaw = raw;
        const t = progress(raw);
        const point = pointAt(t, end);
        trail.push({ t, ...point });
        while (trail.length > 2 && trail[0].t < t - trailSpan) trail.shift();

        const fresh = trail.filter((sample) => sample.t > previousT);
        previousT = t;
        if (style === "bolt") {
          // Every few samples: the jitter between them is the crackle.
          drawLightning(trail.filter((_, index) => index % 3 === 0 || index === trail.length - 1), rgb, 1);
        } else {
          // Tapering ribbon behind the head.
          for (let index = 1; index < trail.length; index += 1) {
            const a = trail[index - 1];
            const b = trail[index];
            const share = index / trail.length;
            renderer.line(a, b, (style === "spark" ? 3.5 : 11) * size * share,
              rgb, share * (style === "wisp" ? 0.6 : 0.9));
          }
        }
        // A white-hot core along the front of the trail.
        if (style !== "bolt" && trail.length > 2) {
          const core = trail.filter((sample) => sample.t >= t - trailSpan * 0.35);
          for (let i = 1; i < core.length; i++) {
            renderer.line(core[i - 1], core[i], (style === "spark" ? 1.2 : 2.4) * size, "255, 255, 255", 0.75);
          }
        }
        drawHead(point.x, point.y, (style === "spark" ? 3.5 : style === "bolt" ? 4.5 : 8) * size, rgb);
        // Shed sparks and the odd four-point star.
        const shed = Math.min(14, (style === "spark" ? 1 : style === "wisp" ? 3 : 4) * Math.max(1, Math.round(fresh.length / 2)));
        for (let index = 0; index < shed; index += 1) {
          // Spread along the stretch flown since the last frame.
          const from = fresh.length ? fresh[Math.floor(Math.random() * fresh.length)] : point;
          emit({
            kind: Math.random() < (style === "star" ? 0.12 : 0.05) ? "star" : "dot",
            x: from.x + (Math.random() - 0.5) * 6,
            y: from.y + (Math.random() - 0.5) * 6,
            vx: (Math.random() - 0.5) * 1.6,
            vy: (Math.random() - 0.5) * 1.6 + (style === "wisp" ? -0.4 : 0.15),
            size: (1 + Math.random() * 1.6) * size,
            life: 320 + Math.random() * 420,
            alpha: 0.85,
            rgb: Math.random() < 0.5 ? rgb : sparkle,
            flicker: style === "wisp",
            seed: Math.random() * 10,
          });
        }
        if (raw >= 1 && !landed) {
          landed = true;
          resolve(point);
          return false;
        }
        return true;
      },
    });
    start();
  });
}

// ---- bursts ----------------------------------------------------------------

/**
 * kind: "impact" (a comet landing), "sparks" (an ability fading out),
 * "embers" (a spell burning away, drifting up), "shards" (a card breaking).
 * `rect` spreads the emission over an area instead of a point.
 */
export function burst({ at, rect = null, rgb = "183, 156, 255", accentRgb = null, kind = "impact", count = null }) {
  if (prefersReducedMotion() || !ensureCanvas()) return;
  const origin = center(at || rect);
  if (!origin) return;
  const amount = count ?? { impact: 46, sparks: 36, embers: 70, shards: 34 }[kind] ?? 40;
  for (let index = 0; index < amount; index += 1) {
    const angle = Math.random() * TAU;
    const speed = { impact: 2 + Math.random() * 4.5, sparks: 0.6 + Math.random() * 2.2, embers: 0.2 + Math.random() * 0.9, shards: 1.5 + Math.random() * 4 }[kind] ?? 2;
    const x = rect ? rect.left + Math.random() * rect.width : origin.x;
    const y = rect ? rect.top + Math.random() * rect.height : origin.y;
    emit({
      kind: kind === "shards" ? "shard" : Math.random() < 0.14 ? "star" : "dot",
      x,
      y,
      vx: Math.cos(angle) * speed,
      vy: Math.sin(angle) * speed + (kind === "embers" ? -1.1 - Math.random() : 0),
      gravity: kind === "shards" ? 0.32 : kind === "embers" ? -0.015 : 0,
      drag: kind === "embers" ? 0.985 : kind === "shards" ? 0.99 : 0.93,
      spin: kind === "shards" ? (Math.random() - 0.5) * 0.3 : 0,
      size: kind === "shards" ? 3 + Math.random() * 5 : 1 + Math.random() * 2,
      life: { impact: 420, sparks: 700, embers: 1100, shards: 900 }[kind] + Math.random() * 400,
      alpha: kind === "shards" ? 0.75 : 0.9,
      rgb: accentRgb && Math.random() < 0.45 ? accentRgb : rgb,
      flicker: kind === "embers" || kind === "sparks",
      shrink: kind !== "shards",
      seed: Math.random() * 10,
    });
  }
  start();
}

/** A ring that expands from a point and fades: the shockwave of a landing. */
export function shockwave({ at, rgb = "183, 156, 255", radius = 90, duration = 520, width = 3 }) {
  const origin = center(at);
  if (!origin || prefersReducedMotion() || !ensureCanvas()) return;
  const began = performance.now();
  effects.add({
    step(now) {
      const t = clamp01((now - began) / duration);
      const r = radius * easeOut(t);
      renderer.ring(origin.x, origin.y, r, width * (1 - t) + 0.5, rgb, (1 - t) * 0.9);
      renderer.ring(origin.x, origin.y, r * 0.82, 1, "255, 255, 255", (1 - t) * 0.6);
      return t < 1;
    },
  });
  start();
}

/**
 * A triggered ability's sigil over its source: two counter-rotating rune
 * rings and a crown of rays. Resolves when the sigil peaks, which is when
 * the wisp should leave.
 */
export function sigil({ rect, rgb = "183, 156, 255", duration = 700 }) {
  const origin = center(rect);
  if (!origin || prefersReducedMotion() || !ensureCanvas()) return Promise.resolve();
  const base = rect?.width ? Math.max(rect.width, rect.height) * 0.55 : 40;
  const began = performance.now();
  return new Promise((resolve) => {
    let peaked = false;
    effects.add({
      cancel: () => resolve(),
      step(now) {
        const t = clamp01((now - began) / duration);
        const grow = easeOut(Math.min(1, t * 1.6));
        const fade = t < 0.55 ? 1 : 1 - (t - 0.55) / 0.45;
        const spin = t * Math.PI * 0.9;
        const point = (angle, radius) => ({ x: origin.x + Math.cos(angle) * radius, y: origin.y + Math.sin(angle) * radius });
        for (const [scale, direction, ticks] of [[1, 1, 12], [0.72, -1, 8]]) {
          const radius = base * scale * grow;
          renderer.ring(origin.x, origin.y, radius, 1.6, rgb, 0.85 * fade);
          for (let index = 0; index < ticks; index++) {
            const angle = index / ticks * TAU + spin * direction;
            renderer.line(point(angle, radius), point(angle, radius + 7), 1.6, rgb, 0.85 * fade);
          }
        }
        for (let index = 0; index < 8; index++) {
          const angle = index / 8 * TAU + spin * 0.5;
          const inner = base * 0.3 * grow;
          const outer = base * (1.25 + 0.2 * Math.sin(t * 12 + index)) * grow;
          renderer.line(point(angle, inner), point(angle, outer), 2, "255, 255, 255", 0.7 * fade, 0);
        }
        if (!peaked && t >= 0.4) {
          peaked = true;
          resolve();
        }
        return t < 1;
      },
    });
    start();
  });
}

/**
 * An activated ability's source charging: its outline brightens and throws
 * two pulses. Resolves when the charge releases.
 */
export function chargePulse({ rect, rgb = "183, 156, 255", duration = 360 }) {
  if (!rect || prefersReducedMotion() || !ensureCanvas()) return Promise.resolve();
  const began = performance.now();
  return new Promise((resolve) => {
    let released = false;
    effects.add({
      cancel: () => resolve(),
      step(now) {
        // rAF stamps can trail the performance.now() an effect began at.
        const elapsed = Math.max(0, now - began);
        const t = Math.min(1, elapsed / (duration * 1.8));
        for (const offset of [0, 0.32]) {
          const local = Math.max(0, Math.min(1, (t - offset) / 0.68));
          if (local <= 0 || local >= 1) continue;
          const grow = 4 + easeOut(local) * 18;
          renderer.rect(rect.left - grow, rect.top - grow, rect.width + grow * 2, rect.height + grow * 2,
            rgb, (1 - local) * 0.95, true);
        }
        const charge = Math.min(1, elapsed / duration);
        renderer.rect(rect.left, rect.top, rect.width, rect.height, rgb, 0.18 * Math.sin(charge * Math.PI));
        if (!released && elapsed >= duration) {
          released = true;
          resolve();
        }
        return t < 1;
      },
    });
    start();
  });
}

// Zone palettes: a spell from somewhere other than the hand trails that
// zone's light behind its owner's.
export const ZONE_ACCENTS = {
  graveyard: "140, 255, 170",
  exile: "205, 165, 255",
  library: "120, 205, 255",
  command: "255, 214, 120",
  hand: "255, 255, 255",
};
