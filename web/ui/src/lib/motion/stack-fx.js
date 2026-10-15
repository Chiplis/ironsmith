// Light effects for the card stack (CardStackPile): comets that carry a spell
// or ability onto the stack, the charge and sigil a source shows as it fires,
// and the sparks, embers and rings a card leaves behind. One fixed canvas,
// drawn additively, alive only while an effect is.

const TAU = Math.PI * 2;
const MAX_PARTICLES = 900;

let canvas = null;
let ctx = null;
let dpr = 1;
let frame = null;
let lastTime = 0;
const effects = new Set();
const particles = [];

// Reduced motion, or a page nobody is looking at (animation frames are
// paused there, so a flourish would only play late on return): no light.
export function prefersReducedMotion() {
  if (typeof window === "undefined") return false;
  return Boolean(window.matchMedia?.("(prefers-reduced-motion: reduce)").matches || document.hidden);
}

function ensureCanvas() {
  if (canvas || typeof document === "undefined") return Boolean(canvas);
  canvas = document.createElement("canvas");
  canvas.className = "stack-fx-canvas";
  canvas.setAttribute("aria-hidden", "true");
  document.body.appendChild(canvas);
  ctx = canvas.getContext("2d");
  const resize = () => {
    dpr = Math.min(2, window.devicePixelRatio || 1);
    canvas.width = Math.round(window.innerWidth * dpr);
    canvas.height = Math.round(window.innerHeight * dpr);
    canvas.style.width = `${window.innerWidth}px`;
    canvas.style.height = `${window.innerHeight}px`;
  };
  resize();
  window.addEventListener("resize", resize);
  return true;
}

function start() {
  if (frame != null || !ensureCanvas()) return;
  lastTime = performance.now();
  canvas.style.display = "block";
  frame = requestAnimationFrame(tick);
}

function tick(now) {
  const dt = Math.min(48, now - lastTime);
  lastTime = now;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.globalCompositeOperation = "lighter";
  // One broken effect must not stop the loop (and every effect after it).
  for (const effect of [...effects]) {
    try {
      if (effect.step(now, dt) === false) effects.delete(effect);
    } catch (error) {
      effects.delete(effect);
      console.error("[stack fx] effect failed", error);
    }
  }
  try {
    stepParticles(dt);
  } catch (error) {
    particles.length = 0;
    console.error("[stack fx] particles failed", error);
  }
  ctx.globalCompositeOperation = "source-over";
  if (effects.size === 0 && particles.length === 0) {
    frame = null;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    canvas.style.display = "none";
    return;
  }
  frame = requestAnimationFrame(tick);
}

function rgba(rgb, alpha) {
  return `rgba(${rgb}, ${Math.max(0, Math.min(1, alpha))})`;
}

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

// ---- particles -------------------------------------------------------------

function emit(particle) {
  if (particles.length >= MAX_PARTICLES) particles.shift();
  particles.push({ age: 0, rotation: 0, spin: 0, gravity: 0, drag: 0.94, ...particle });
}

function drawStar(x, y, size, alpha, rgb) {
  ctx.strokeStyle = rgba(rgb, alpha);
  ctx.lineWidth = Math.max(0.6, size * 0.22);
  ctx.beginPath();
  ctx.moveTo(x - size, y);
  ctx.lineTo(x + size, y);
  ctx.moveTo(x, y - size);
  ctx.lineTo(x, y + size);
  ctx.stroke();
}

function stepParticles(dt) {
  for (let index = particles.length - 1; index >= 0; index -= 1) {
    const p = particles[index];
    p.age += dt;
    if (p.age >= p.life) {
      particles.splice(index, 1);
      continue;
    }
    const k = dt / 16;
    p.vx *= p.drag ** k;
    p.vy = p.vy * p.drag ** k + p.gravity * k;
    p.x += p.vx * k;
    p.y += p.vy * k;
    p.rotation += p.spin * k;
    const life = 1 - p.age / p.life;
    const alpha = p.alpha * (p.flicker ? 0.55 + 0.45 * Math.sin(p.age * 0.05 + p.seed) : 1) * life;
    const size = p.size * (p.shrink === false ? 1 : 0.35 + 0.65 * life);
    if (p.kind === "star") {
      drawStar(p.x, p.y, size * 2.4, alpha, p.rgb);
    } else if (p.kind === "shard") {
      ctx.save();
      ctx.translate(p.x, p.y);
      ctx.rotate(p.rotation);
      ctx.fillStyle = rgba(p.rgb, alpha);
      ctx.beginPath();
      ctx.moveTo(0, -size * 1.6);
      ctx.lineTo(size, size);
      ctx.lineTo(-size * 0.8, size * 0.6);
      ctx.closePath();
      ctx.fill();
      ctx.restore();
    } else {
      const glow = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, size * 3);
      glow.addColorStop(0, rgba("255, 255, 255", alpha));
      glow.addColorStop(0.25, rgba(p.rgb, alpha * 0.9));
      glow.addColorStop(1, rgba(p.rgb, 0));
      ctx.fillStyle = glow;
      ctx.beginPath();
      ctx.arc(p.x, p.y, size * 3, 0, TAU);
      ctx.fill();
    }
  }
}

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
  const glow = ctx.createRadialGradient(x, y, 0, x, y, radius * 4);
  glow.addColorStop(0, rgba("255, 255, 255", alpha));
  glow.addColorStop(0.18, rgba("255, 255, 255", alpha * 0.9));
  glow.addColorStop(0.35, rgba(rgb, alpha * 0.8));
  glow.addColorStop(1, rgba(rgb, 0));
  ctx.fillStyle = glow;
  ctx.beginPath();
  ctx.arc(x, y, radius * 4, 0, TAU);
  ctx.fill();
}

function drawLightning(points, rgb, alpha) {
  if (points.length < 2) return;
  const jagged = points.map((point, index) => (
    index === 0 || index === points.length - 1
      ? point
      : { x: point.x + (Math.random() - 0.5) * 14, y: point.y + (Math.random() - 0.5) * 14 }
  ));
  for (const [width, color] of [[7, rgba(rgb, alpha * 0.35)], [3, rgba(rgb, alpha)], [1.2, rgba("255, 255, 255", alpha)]]) {
    ctx.strokeStyle = color;
    ctx.lineWidth = width;
    ctx.lineJoin = "round";
    ctx.beginPath();
    jagged.forEach((point, index) => (index ? ctx.lineTo(point.x, point.y) : ctx.moveTo(point.x, point.y)));
    ctx.stroke();
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
  size = 1,
} = {}) {
  const origin = center(from);
  const end0 = center(to);
  if (!origin || !end0 || prefersReducedMotion()) return Promise.resolve();
  const distance = Math.hypot(end0.x - origin.x, end0.y - origin.y);
  const total = duration ?? {
    star: 420 + distance * 0.42,
    bolt: 230 + distance * 0.16,
    wisp: 620 + distance * 0.55,
    spiral: 760 + distance * 0.4,
    spark: 240,
  }[style] ?? 600;
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
            ctx.strokeStyle = rgba(rgb, share * (style === "wisp" ? 0.6 : 0.9));
            ctx.lineWidth = (style === "spark" ? 3.5 : 11) * size * share;
            ctx.lineCap = "butt";
            ctx.beginPath();
            ctx.moveTo(a.x, a.y);
            ctx.lineTo(b.x, b.y);
            ctx.stroke();
          }
        }
        // A white-hot core along the front of the trail.
        if (style !== "bolt" && trail.length > 2) {
          const core = trail.filter((sample) => sample.t >= t - trailSpan * 0.35);
          ctx.strokeStyle = rgba("255, 255, 255", 0.75);
          ctx.lineWidth = (style === "spark" ? 1.2 : 2.4) * size;
          ctx.lineCap = "round";
          ctx.lineJoin = "round";
          ctx.beginPath();
          core.forEach((sample, index) => (index ? ctx.lineTo(sample.x, sample.y) : ctx.moveTo(sample.x, sample.y)));
          ctx.stroke();
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
  if (prefersReducedMotion()) return;
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
  if (!origin || prefersReducedMotion()) return;
  const began = performance.now();
  effects.add({
    step(now) {
      const t = clamp01((now - began) / duration);
      const r = radius * easeOut(t);
      ctx.strokeStyle = rgba(rgb, (1 - t) * 0.9);
      ctx.lineWidth = width * (1 - t) + 0.5;
      ctx.beginPath();
      ctx.arc(origin.x, origin.y, r, 0, TAU);
      ctx.stroke();
      ctx.strokeStyle = rgba("255, 255, 255", (1 - t) * 0.6);
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.arc(origin.x, origin.y, r * 0.82, 0, TAU);
      ctx.stroke();
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
  if (!origin || prefersReducedMotion()) return Promise.resolve();
  const base = rect?.width ? Math.max(rect.width, rect.height) * 0.55 : 40;
  const began = performance.now();
  return new Promise((resolve) => {
    let peaked = false;
    effects.add({
      step(now) {
        const t = clamp01((now - began) / duration);
        const grow = easeOut(Math.min(1, t * 1.6));
        const fade = t < 0.55 ? 1 : 1 - (t - 0.55) / 0.45;
        const spin = t * Math.PI * 0.9;
        ctx.save();
        ctx.translate(origin.x, origin.y);
        for (const [scale, direction, ticks] of [[1, 1, 12], [0.72, -1, 8]]) {
          const radius = base * scale * grow;
          ctx.rotate(spin * direction);
          ctx.strokeStyle = rgba(rgb, 0.85 * fade);
          ctx.lineWidth = 1.6;
          ctx.beginPath();
          ctx.arc(0, 0, radius, 0, TAU);
          ctx.stroke();
          for (let index = 0; index < ticks; index += 1) {
            const angle = (index / ticks) * TAU;
            ctx.beginPath();
            ctx.moveTo(Math.cos(angle) * radius, Math.sin(angle) * radius);
            ctx.lineTo(Math.cos(angle) * (radius + 7), Math.sin(angle) * (radius + 7));
            ctx.stroke();
          }
          ctx.rotate(-spin * direction);
        }
        // Rays
        for (let index = 0; index < 8; index += 1) {
          const angle = (index / 8) * TAU + spin * 0.5;
          const inner = base * 0.3 * grow;
          const outer = base * (1.25 + 0.2 * Math.sin(t * 12 + index)) * grow;
          const gradient = ctx.createLinearGradient(Math.cos(angle) * inner, Math.sin(angle) * inner, Math.cos(angle) * outer, Math.sin(angle) * outer);
          gradient.addColorStop(0, rgba("255, 255, 255", 0.7 * fade));
          gradient.addColorStop(1, rgba(rgb, 0));
          ctx.strokeStyle = gradient;
          ctx.lineWidth = 2;
          ctx.beginPath();
          ctx.moveTo(Math.cos(angle) * inner, Math.sin(angle) * inner);
          ctx.lineTo(Math.cos(angle) * outer, Math.sin(angle) * outer);
          ctx.stroke();
        }
        ctx.restore();
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
  if (!rect || prefersReducedMotion()) return Promise.resolve();
  const began = performance.now();
  return new Promise((resolve) => {
    let released = false;
    effects.add({
      step(now) {
        // rAF stamps can trail the performance.now() an effect began at.
        const elapsed = Math.max(0, now - began);
        const t = Math.min(1, elapsed / (duration * 1.8));
        for (const offset of [0, 0.32]) {
          const local = Math.max(0, Math.min(1, (t - offset) / 0.68));
          if (local <= 0 || local >= 1) continue;
          const grow = 4 + easeOut(local) * 18;
          ctx.strokeStyle = rgba(rgb, (1 - local) * 0.95);
          ctx.lineWidth = 2.5 * (1 - local) + 0.5;
          const radius = 10;
          const x = rect.left - grow;
          const y = rect.top - grow;
          const w = rect.width + grow * 2;
          const h = rect.height + grow * 2;
          ctx.beginPath();
          if (ctx.roundRect) ctx.roundRect(x, y, w, h, radius);
          else ctx.rect(x, y, w, h);
          ctx.stroke();
        }
        // Inner flash while charging.
        const charge = Math.min(1, elapsed / duration);
        ctx.fillStyle = rgba(rgb, 0.18 * Math.sin(charge * Math.PI));
        ctx.fillRect(rect.left, rect.top, rect.width, rect.height);
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
