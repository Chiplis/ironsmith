import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import useScryfallImageUrl from "@/hooks/useScryfallImageUrl";
import { RIFT_DISSOLVE_EXILE_EFFECT_MS } from "@/lib/game-animations";

const EXILE_EFFECT_DURATION_MS = RIFT_DISSOLVE_EXILE_EFFECT_MS;
const EXILE_EFFECT_KIND = "rift-dissolve-exile";
const ANGELIC_DESTROY_EFFECT_DURATION_MS = 2400;
const ANGELIC_DESTROY_EFFECT_KIND = "angelic-destroy";
const MAX_SHADER_EFFECTS = 8;
const TARGET_WAIT_TIMEOUT_MS = 1100;
// The inspector mounts with hover-art-drop-in (360ms translate3d -36px → 0)
// and hover-art-slice-in (320ms translateY -28px → 0). getBoundingClientRect
// reflects the *currently transformed* position, so polling captures a
// mid-animation rect unless we wait for those to settle first.
const INSPECTOR_ENTRY_SETTLE_MS = 420;
const MAX_INSPECTOR_SCALE = 2.8;
const CLEANUP_TAIL_MS = 220;

const VERTEX_SHADER_SOURCE = `
attribute vec2 a_position;
void main() {
  gl_Position = vec4(a_position, 0.0, 1.0);
}
`;

const FRAGMENT_SHADER_SOURCE = `
precision highp float;

const int MAX_EFFECTS = 8;

uniform vec2 u_resolution;
uniform float u_dpr;
uniform float u_time;
uniform int u_count;
uniform vec4 u_sourceRects[MAX_EFFECTS];
uniform vec4 u_targetRects[MAX_EFFECTS];
uniform vec4 u_clipRects[MAX_EFFECTS];
uniform float u_progress[MAX_EFFECTS];
uniform float u_seed[MAX_EFFECTS];
uniform float u_effectKinds[MAX_EFFECTS];
uniform vec3 u_accentColors[MAX_EFFECTS];
uniform sampler2D u_inspectorImage;
uniform vec4 u_inspectorRect;
uniform float u_inspectorReady;

float saturate(float v) {
  return clamp(v, 0.0, 1.0);
}

float easeOutCubic(float t) {
  t = saturate(t);
  return 1.0 - pow(1.0 - t, 3.0);
}

float easeInCubic(float t) {
  t = saturate(t);
  return t * t * t;
}

float easeInOut(float t) {
  t = saturate(t);
  return t * t * (3.0 - 2.0 * t);
}

float hash(vec2 p) {
  p = fract(p * vec2(123.34, 456.21));
  p += dot(p, p + 45.32);
  return fract(p.x * p.y);
}

float sdSegment(vec2 p, vec2 a, vec2 b) {
  vec2 pa = p - a;
  vec2 ba = b - a;
  float h = clamp(dot(pa, ba) / max(dot(ba, ba), 0.0001), 0.0, 1.0);
  return length(pa - ba * h);
}

float rectMask(vec2 p, vec4 rect, float feather) {
  vec2 minP = rect.xy;
  vec2 maxP = rect.xy + rect.zw;
  vec2 inside = smoothstep(minP, minP + feather, p) * (1.0 - smoothstep(maxP - feather, maxP, p));
  return inside.x * inside.y;
}

float ringRect(vec2 p, vec4 rect, float width, float feather) {
  float outer = rectMask(p, vec4(rect.x - width, rect.y - width, rect.z + width * 2.0, rect.w + width * 2.0), feather);
  float inner = rectMask(p, rect, feather);
  return saturate(outer - inner);
}

vec3 effectColor(vec2 p, vec4 src, vec4 dst, vec4 clip, float progress, float seed, vec3 accentColor) {
  vec2 srcC = src.xy + src.zw * 0.5;
  vec2 dstC = dst.xy + dst.zw * 0.5;
  vec2 path = dstC - srcC;
  float pathLength = max(length(path), 1.0);
  vec2 dir = path / pathLength;
  vec2 side = vec2(-dir.y, dir.x);
  float along = dot(p - srcC, dir);
  float lateral = dot(p - srcC, side);
  float pathT = saturate(along / pathLength);
  float travel = easeInCubic(saturate((progress - 0.04) / 0.76));
  float headAlong = pathLength * travel;

  float targetFillProgress = smoothstep(0.74, 0.94, progress) * smoothstep(1.0, 0.82, progress);
  float clipMask = rectMask(p, clip, 5.0);
  vec2 clipC = clip.xy + clip.zw * 0.5;
  vec2 landingSpace = vec2((p.x - dstC.x) / max(clip.z, 1.0), (p.y - dstC.y) / max(clip.w, 1.0));
  float landingEnvelope = exp(-dot(landingSpace, landingSpace) * 7.5);
  float inspectorEnvelope = rectMask(p, clip, 22.0);
  float arrivalFade = mix(1.0, 0.08, smoothstep(0.68, 0.98, pathT));
  float inspectorAbsorb = mix(1.0, 0.12, inspectorEnvelope * smoothstep(0.58, 0.92, progress));

  float streamWidth = mix(src.z * 0.72, max(dst.z * 0.42, 42.0), pathT);
  float lateralNorm = abs(lateral) / max(streamWidth, 1.0);
  float lateralEnvelope = exp(-lateralNorm * lateralNorm * 1.18);
  float pathEnvelope = smoothstep(-48.0, 34.0, along) * smoothstep(pathLength + 66.0, pathLength - 8.0, along);
  float tail = smoothstep(430.0, 0.0, headAlong - along) * smoothstep(-22.0, 62.0, headAlong - along);
  float front = exp(-abs(along - headAlong) * 0.018);
  float turbulence = sin(pathT * 19.0 + seed * 37.0 + progress * 12.0) * 20.0
                   + sin(pathT * 47.0 + seed * 91.0) * 8.0;

  vec2 particleCoord = vec2(
    along / 8.4 - travel * pathLength / 6.8,
    (lateral + turbulence) / 8.4
  );
  vec2 cell = floor(particleCoord);
  vec2 cellJitter = vec2(hash(cell + seed * 127.0 + 5.1), hash(cell + seed * 149.0 + 9.7)) - 0.5;
  vec2 local = fract(particleCoord) - 0.5 - cellJitter * 0.42;
  float particleSeed = hash(cell + seed * 113.0);
  float particleSize = mix(8.0, 17.0, hash(cell + 31.0));
  float fireflyCore = exp(-dot(local, local) * particleSize);
  float fireflyGlow = exp(-dot(local, local) * (particleSize * 0.18));
  float fireflyVisible = step(0.36, particleSeed);
  float fireflyTwinkle = 0.72 + 0.28 * sin(progress * 21.0 + particleSeed * 6.28318 + seed * 17.0);
  fireflyCore *= fireflyVisible * fireflyTwinkle;
  fireflyGlow *= fireflyVisible * fireflyTwinkle;

  float streamActive = smoothstep(0.06, 0.18, progress) * smoothstep(0.96, 0.78, progress);
  float fireflyPath = (tail * 0.92 + front * 0.64);
  float particleStream = fireflyCore
                       * fireflyPath
                       * lateralEnvelope
                       * pathEnvelope
                       * streamActive
                       * arrivalFade
                       * inspectorAbsorb;
  float haloStream = fireflyGlow
                   * fireflyPath
                   * lateralEnvelope
                   * pathEnvelope
                   * smoothstep(0.1, 0.28, progress)
                   * smoothstep(0.94, 0.7, progress)
                   * streamActive
                   * arrivalFade
                   * inspectorAbsorb;

  float sourceMask = rectMask(p, src, 8.0);
  float sourceBreak = saturate((progress - 0.10) / 0.72);
  float sourceCoord = dot(p - src.xy, dir) / max(src.z, src.w);
  float sourceEdgeNoise = sin((p.y + seed * 193.0) * 0.11) * 0.045
                        + sin((p.x + seed * 71.0) * 0.17) * 0.028;
  float sourceCut = smoothstep(sourceBreak - 0.16 + sourceEdgeNoise, sourceBreak + 0.10 + sourceEdgeNoise, sourceCoord);
  float sourceAsh = sourceMask * sourceCut * smoothstep(0.08, 0.22, progress) * smoothstep(0.94, 0.72, progress);

  vec2 cloudSpace = (p - clipC) + seed * 73.0;
  vec2 cloudCell = floor(cloudSpace / 9.0);
  vec2 cloudJitter = vec2(hash(cloudCell + seed * 269.0 + 4.6), hash(cloudCell + seed * 281.0 + 7.2)) - 0.5;
  vec2 cloudLocal = fract(cloudSpace / 9.0) - 0.5 - cloudJitter * 0.44;
  float cloudSeed = hash(cloudCell * 1.31 + seed * 4.0);
  float cloudParticle = exp(-dot(cloudLocal, cloudLocal) * (12.0 + cloudSeed * 10.0));
  cloudParticle *= step(0.34, cloudSeed)
                 * targetFillProgress
                 * clipMask
                 * inspectorEnvelope
                 * mix(landingEnvelope, 1.0, 0.22)
                 * 0.14;

  vec3 white = vec3(1.0);
  vec3 pearl = vec3(1.0, 0.96, 0.82);
  vec3 ash = vec3(0.82, 0.82, 0.76);
  float shimmer = 0.84 + 0.16 * sin(pathT * 37.0 + seed * 41.0 + progress * 15.0);
  vec3 color = vec3(0.0);
  color += mix(ash, pearl, 0.82) * sourceAsh * 1.02;
  color += white * particleStream * 1.72 * shimmer;
  color += accentColor * haloStream * 0.62;
  color += white * particleStream * front * 0.58;
  color += mix(white, accentColor, 0.42) * cloudParticle * mix(0.7, 0.3, u_inspectorReady);
  return color;
}

void main() {
  vec2 pixel = gl_FragCoord.xy / max(u_dpr, 0.0001);
  vec2 p = vec2(pixel.x, u_resolution.y - pixel.y);
  vec3 color = vec3(0.0);
  float groupProgress = 1.0;
  float hasAngelicDestroy = 0.0;
  float globalWindow = 0.0;

  for (int i = 0; i < MAX_EFFECTS; i++) {
    if (i >= u_count) break;
    float rawProgress = u_progress[i];
    float effectKind = u_effectKinds[i];
    float pi = effectKind < 0.5 ? saturate(rawProgress / 0.8) : rawProgress;
    groupProgress = min(groupProgress, rawProgress);
    hasAngelicDestroy = max(hasAngelicDestroy, step(0.5, effectKind));
    if (effectKind < 0.5) {
      color += effectColor(p, u_sourceRects[i], u_targetRects[i], u_clipRects[i], pi, u_seed[i], u_accentColors[i]);
    }
  }

  float revealStart = mix(0.84, 0.54, hasAngelicDestroy);
  float revealEnd = mix(1.0, 0.74, hasAngelicDestroy);

  // All cards in the current batch must reach the inspector before the black
  // foreground mask dissolves away to reveal the DOM-rendered inspector art.
  float globalReveal = smoothstep(revealStart, revealEnd, groupProgress);
  globalWindow = 1.0;
  float alpha = saturate(max(max(color.r, color.g), color.b));

  if (u_inspectorReady > 0.5
      && u_inspectorRect.z > 0.0
      && u_inspectorRect.w > 0.0
      && globalWindow > 0.0) {
    vec2 uv = (p - u_inspectorRect.xy) / u_inspectorRect.zw;
    if (uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0) {
      float aspect = u_inspectorRect.z / max(u_inspectorRect.w, 1.0);
      vec2 centered = uv - 0.5;
      float centerDist = length(vec2(centered.x * aspect, centered.y));
      float maxDist = length(vec2(0.5 * aspect, 0.5));

      vec2 cell = floor(p / 4.25);
      vec2 coarseCell = floor(p / 13.0);
      float cellSeed = hash(cell + 11.0);
      float coarseSeed = hash(coarseCell + 47.0);
      float waveNoise = sin((p.x * 0.041) + (p.y * 0.033) + coarseSeed * 6.28318) * 0.028;
      float jaggedNoise = (cellSeed - 0.5) * 0.11 + (coarseSeed - 0.5) * 0.08 + waveNoise;

      float revealRadius = mix(-0.10, maxDist + 0.16, easeOutCubic(globalReveal));
      float pixelReveal = saturate(1.0 - smoothstep(
        revealRadius - 0.08 + jaggedNoise,
        revealRadius + 0.035 + jaggedNoise,
        centerDist
      ));

      float dissolveNoise = hash(floor(p / 2.7) + 89.0);
      float freckleWindow = smoothstep(0.04, 0.36, globalReveal) * smoothstep(0.96, 0.62, globalReveal);
      pixelReveal = saturate(pixelReveal + (dissolveNoise - 0.68) * 0.18 * freckleWindow);

      vec2 cellLocal = fract(p / 4.25) - 0.5;
      float sparkShape = smoothstep(0.1, 0.0, length(cellLocal));
      float edgeBand = exp(-pow((centerDist - revealRadius) / 0.055, 2.0))
                     * smoothstep(0.02, 0.16, globalReveal)
                     * smoothstep(1.0, 0.84, globalReveal);
      float frontier = sparkShape * step(0.5, cellSeed) * edgeBand;

      color += vec3(1.0) * frontier * globalWindow * 0.42;
      alpha = max(alpha, frontier * globalWindow * 0.44);
    }
  }

  gl_FragColor = vec4(color, alpha);
}
`;

function clamp(value, min, max) {
  return Math.min(Math.max(value, min), max);
}

function hashText(text) {
  let hash = 2166136261;
  const source = String(text || "");
  for (let index = 0; index < source.length; index += 1) {
    hash ^= source.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

function deterministicEffectId(rawEffect) {
  if (rawEffect.id) return String(rawEffect.id);
  const seedSource = `${rawEffect.kind || "effect"}:${rawEffect.card?.name || "?"}:${rawEffect.objectId ?? "?"}:${rawEffect.targetToken || "?"}`;
  return `${seedSource}:${hashText(seedSource).toString(36)}`;
}

function normalizeRect(rect) {
  if (!rect) return null;
  const width = Number(rect.width);
  const height = Number(rect.height);
  const left = Number(rect.left);
  const top = Number(rect.top);
  if (![width, height, left, top].every(Number.isFinite) || width <= 0 || height <= 0) {
    return null;
  }
  return {
    left,
    top,
    width,
    height,
    right: left + width,
    bottom: top + height,
  };
}

function escapeAttributeValue(value) {
  if (typeof CSS !== "undefined" && typeof CSS.escape === "function") {
    return CSS.escape(String(value));
  }
  return String(value).replace(/["\\]/g, "\\$&");
}

function isElementVisuallyAvailable(element) {
  if (!element || typeof element.getBoundingClientRect !== "function") return false;
  // Hand inspector keeps both compact + expanded shells in the DOM at the same
  // time — the inactive one carries the `is-closed` class on a clip-path host.
  if (typeof element.closest === "function" && element.closest(".is-closed")) return false;
  const rect = element.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) return false;
  if (rect.right <= 0 || rect.bottom <= 0) return false;
  if (rect.left >= window.innerWidth || rect.top >= window.innerHeight) return false;
  return true;
}

function resolveInspectorTargetElement(targetToken, targetScope = "foreground") {
  if (!targetToken || typeof document === "undefined") return null;
  const escapedToken = escapeAttributeValue(targetToken);
  // Two inspector shells (compact + expanded) can both carry the same token at
  // once; we want whichever one is actually visible. For each preferred
  // selector, gather all matches and pick the largest visible rect.
  const cropSelectors = [
    `.hover-art-stage[data-zone-transition-token="${escapedToken}"] .hover-art-foreground-crop`,
    `.hover-art-stage[data-zone-transition-token="${escapedToken}"] .hover-art-full-art-crop`,
    `.ironsmith-inspector-shell[data-zone-transition-token="${escapedToken}"] .hover-art-foreground-crop`,
    `.ironsmith-inspector-shell[data-zone-transition-token="${escapedToken}"] .hover-art-full-art-crop`,
  ];
  const shellSelectors = [
    `.ironsmith-inspector-shell[data-zone-transition-token="${escapedToken}"]`,
  ];
  const stageSelectors = [
    `.hover-art-stage[data-zone-transition-token="${escapedToken}"]`,
    `.ironsmith-inspector-shell[data-zone-transition-token="${escapedToken}"] .hover-art-stage`,
  ];
  const preferredSelectors = targetScope === "inspector" ? shellSelectors : cropSelectors;
  const fallbackSelectors = targetScope === "inspector" ? [...stageSelectors, ...cropSelectors] : [...stageSelectors, ...shellSelectors];
  let bestElement = null;
  let bestArea = 0;

  const findBestElement = (selectors) => {
    for (const selector of selectors) {
      const matches = document.querySelectorAll(selector);
      for (const candidate of matches) {
        if (!isElementVisuallyAvailable(candidate)) continue;
        const rect = candidate.getBoundingClientRect();
        const area = rect.width * rect.height;
        if (area > bestArea) {
          bestArea = area;
          bestElement = candidate;
        }
      }
      if (bestElement) return bestElement;
    }
    return null;
  };

  if (findBestElement(preferredSelectors)) return bestElement;
  bestArea = 0;
  bestElement = null;
  return findBestElement(fallbackSelectors);
}

function resolveInspectorTargetRect(targetToken, targetScope = "foreground") {
  const target = resolveInspectorTargetElement(targetToken, targetScope);
  return normalizeRect(target?.getBoundingClientRect?.());
}

function resolveInspectorImageElement(targetToken) {
  const target = resolveInspectorTargetElement(targetToken, "inspector")
    || resolveInspectorTargetElement(targetToken, "foreground");
  if (!target) return null;
  if (target.tagName === "IMG") return target;
  if (typeof target.querySelector !== "function") return null;
  return target.querySelector("img.hover-art-foreground-image")
    || target.querySelector("img");
}

function resolveLargestInspectorMaskElement(effects) {
  let bestElement = null;
  let bestArea = 0;
  for (const effect of effects) {
    if (!effect.targetToken) continue;
    const target = resolveInspectorTargetElement(effect.targetToken, effect.targetScope);
    if (!target) continue;
    const rect = target.getBoundingClientRect();
    const area = rect.width * rect.height;
    if (area > bestArea) {
      bestArea = area;
      bestElement = target;
    }
  }
  return bestElement;
}

function fallbackTargetRect(sourceRect) {
  const width = Math.min(sourceRect.width * MAX_INSPECTOR_SCALE, 148);
  const height = Math.min(sourceRect.height * MAX_INSPECTOR_SCALE, 206);
  return normalizeRect({
    left: Math.max(8, window.innerWidth - width - 18),
    top: Math.max(8, window.innerHeight - height - 18),
    width,
    height,
  });
}

function capTargetRectToSourceScale(sourceRect, targetRect) {
  const targetCenterX = targetRect.left + targetRect.width / 2;
  const targetCenterY = targetRect.top + targetRect.height / 2;
  const cappedWidth = Math.min(targetRect.width, sourceRect.width * MAX_INSPECTOR_SCALE);
  const cappedHeight = Math.min(targetRect.height, sourceRect.height * MAX_INSPECTOR_SCALE);
  return normalizeRect({
    left: targetCenterX - cappedWidth / 2,
    top: targetCenterY - cappedHeight / 2,
    width: cappedWidth,
    height: cappedHeight,
  });
}

function effectDurationMs(effect) {
  return effect?.kind === ANGELIC_DESTROY_EFFECT_KIND
    ? ANGELIC_DESTROY_EFFECT_DURATION_MS
    : EXILE_EFFECT_DURATION_MS;
}

function resolveFlightTargetRects(rawEffect, sourceRect, targetRect, clipRect = targetRect) {
  const travelsToInspector = rawEffect.travelsToInspector === true;
  const fallbackRect = targetRect || fallbackTargetRect(sourceRect);
  const resolvedTargetRect = travelsToInspector
    ? (
      rawEffect?.kind === EXILE_EFFECT_KIND && rawEffect?.targetScope === "inspector"
        ? normalizeRect({
          left: fallbackRect.left + (fallbackRect.width / 2) - 8,
          top: fallbackRect.top + (fallbackRect.height / 2) - 8,
          width: 16,
          height: 16,
        })
        : capTargetRectToSourceScale(sourceRect, fallbackRect)
    )
    : sourceRect;
  const resolvedClipRect = travelsToInspector
    ? normalizeRect(clipRect || targetRect || resolvedTargetRect)
    : sourceRect;
  return {
    travelsToInspector,
    resolvedTargetRect,
    resolvedClipRect: resolvedClipRect || resolvedTargetRect,
  };
}

function normalizeExileEffect(rawEffect, targetRect, clipRect = targetRect, options = {}) {
  if (!rawEffect || rawEffect.kind !== EXILE_EFFECT_KIND) return null;
  const rect = normalizeRect(rawEffect.rect);
  if (!rect) return null;
  const { travelsToInspector, resolvedTargetRect, resolvedClipRect } = resolveFlightTargetRects(rawEffect, rect, targetRect, clipRect);
  const startDelayMs = Math.max(0, Number(rawEffect.startDelayMs) || 0);
  const anchorAt = Number.isFinite(options.anchorAt) ? options.anchorAt : performance.now();
  const startedAt = anchorAt + startDelayMs;
  const id = deterministicEffectId(rawEffect);
  const cssAnimationDelayMs = startedAt - performance.now();

  const convergeRect = normalizeRect(rawEffect.convergeRect);
  const sourceCenterX = rect.left + rect.width / 2;
  const sourceCenterY = rect.top + rect.height / 2;
  const convergeOffsetX = convergeRect ? (convergeRect.left + convergeRect.width / 2) - sourceCenterX : 0;
  const convergeOffsetY = convergeRect ? (convergeRect.top + convergeRect.height / 2) - sourceCenterY : 0;

  return {
    id,
    kind: EXILE_EFFECT_KIND,
    rect,
    targetRect: resolvedTargetRect,
    clipRect: resolvedClipRect,
    travelsToInspector,
    includeSourceClone: rawEffect.includeSourceClone !== false,
    targetToken: rawEffect.targetToken || null,
    targetScope: rawEffect.targetScope === "inspector" ? "inspector" : "foreground",
    sourceCloneHtml: rawEffect.sourceCloneHtml || null,
    sourceImageUrl: rawEffect.sourceImageUrl || null,
    card: rawEffect.card || {},
    accentColor: rawEffect.accentColor || null,
    accentRgb: rawEffect.accentRgb || null,
    seed: (hashText(`${id}:${rawEffect.card?.name || ""}`) % 10000) / 10000,
    startDelayMs,
    startedAt,
    cssAnimationDelayMs,
    convergeOffsetX,
    convergeOffsetY,
  };
}

function normalizeAngelicDestroyEffect(rawEffect, targetRect, clipRect = targetRect, options = {}) {
  if (!rawEffect || rawEffect.kind !== ANGELIC_DESTROY_EFFECT_KIND) return null;
  const rect = normalizeRect(rawEffect.rect);
  if (!rect) return null;
  const { travelsToInspector, resolvedTargetRect, resolvedClipRect } = resolveFlightTargetRects(rawEffect, rect, targetRect, clipRect);
  const startDelayMs = Math.max(0, Number(rawEffect.startDelayMs) || 0);
  const anchorAt = Number.isFinite(options.anchorAt) ? options.anchorAt : performance.now();
  const startedAt = anchorAt + startDelayMs;
  const id = deterministicEffectId(rawEffect);

  return {
    id,
    kind: ANGELIC_DESTROY_EFFECT_KIND,
    rect,
    targetRect: resolvedTargetRect,
    clipRect: resolvedClipRect,
    travelsToInspector,
    includeSourceClone: rawEffect.includeSourceClone !== false,
    targetToken: rawEffect.targetToken || null,
    targetScope: rawEffect.targetScope === "inspector" ? "inspector" : "foreground",
    sourceCloneHtml: rawEffect.sourceCloneHtml || null,
    sourceImageUrl: rawEffect.sourceImageUrl || null,
    card: rawEffect.card || {},
    accentColor: rawEffect.accentColor || null,
    accentRgb: rawEffect.accentRgb || null,
    seed: (hashText(`${id}:${rawEffect.card?.name || ""}`) % 10000) / 10000,
    startDelayMs,
    startedAt,
    cssAnimationDelayMs: startedAt - performance.now(),
    preflight: rawEffect.preflight === true,
  };
}

function compileShader(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const info = gl.getShaderInfoLog(shader);
    gl.deleteShader(shader);
    throw new Error(info || "Shader compilation failed");
  }
  return shader;
}

function createShaderProgram(gl) {
  const vertexShader = compileShader(gl, gl.VERTEX_SHADER, VERTEX_SHADER_SOURCE);
  let fragmentShader;
  try {
    fragmentShader = compileShader(gl, gl.FRAGMENT_SHADER, FRAGMENT_SHADER_SOURCE);
  } catch (error) {
    gl.deleteShader(vertexShader);
    throw error;
  }
  const program = gl.createProgram();
  gl.attachShader(program, vertexShader);
  gl.attachShader(program, fragmentShader);
  gl.linkProgram(program);
  gl.deleteShader(vertexShader);
  gl.deleteShader(fragmentShader);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    const info = gl.getProgramInfoLog(program);
    gl.deleteProgram(program);
    throw new Error(info || "Shader link failed");
  }
  return program;
}

function rectsToFloat32Array(rects) {
  const values = new Float32Array(MAX_SHADER_EFFECTS * 4);
  for (let index = 0; index < MAX_SHADER_EFFECTS; index += 1) {
    const rect = rects[index];
    if (!rect) continue;
    values[index * 4] = rect.left || 0;
    values[index * 4 + 1] = rect.top || 0;
    values[index * 4 + 2] = rect.width || 0;
    values[index * 4 + 3] = rect.height || 0;
  }
  return values;
}

function parseRgbString(value) {
  const parts = String(value || "")
    .split(",")
    .map((part) => Number.parseFloat(part.trim()));
  if (parts.length < 3 || !parts.slice(0, 3).every(Number.isFinite)) return null;
  return parts.slice(0, 3).map((part) => clamp(part / 255, 0, 1));
}

function parseHexColor(value) {
  const match = String(value || "").trim().match(/^#?([0-9a-f]{6})$/i);
  if (!match) return null;
  const parsed = Number.parseInt(match[1], 16);
  return [
    ((parsed >> 16) & 255) / 255,
    ((parsed >> 8) & 255) / 255,
    (parsed & 255) / 255,
  ];
}

function effectAccentColor(effect) {
  return parseRgbString(effect?.accentRgb)
    || parseHexColor(effect?.accentColor)
    || [0.2, 1.0, 0.9];
}

function accentColorUniformArray(effects) {
  const values = new Float32Array(MAX_SHADER_EFFECTS * 3);
  for (let index = 0; index < MAX_SHADER_EFFECTS; index += 1) {
    const color = effectAccentColor(effects[index]);
    values[index * 3] = color[0];
    values[index * 3 + 1] = color[1];
    values[index * 3 + 2] = color[2];
  }
  return values;
}

function progressUniformArray(effects, now) {
  const values = new Float32Array(MAX_SHADER_EFFECTS);
  for (let index = 0; index < MAX_SHADER_EFFECTS; index += 1) {
    const effect = effects[index];
    values[index] = effect ? clamp((now - effect.startedAt) / effectDurationMs(effect), 0, 1) : 0;
  }
  return values;
}

function seedUniformArray(effects) {
  const values = new Float32Array(MAX_SHADER_EFFECTS);
  for (let index = 0; index < MAX_SHADER_EFFECTS; index += 1) {
    values[index] = effects[index]?.seed || 0;
  }
  return values;
}

function effectKindUniformArray(effects) {
  const values = new Float32Array(MAX_SHADER_EFFECTS);
  for (let index = 0; index < MAX_SHADER_EFFECTS; index += 1) {
    values[index] = effects[index]?.kind === ANGELIC_DESTROY_EFFECT_KIND ? 1 : 0;
  }
  return values;
}

function freshTargetRect(effect) {
  if (effect.travelsToInspector && effect.targetToken) {
    const fresh = resolveInspectorTargetRect(effect.targetToken, effect.targetScope);
    if (fresh) return capTargetRectToSourceScale(effect.rect, fresh);
  }
  return effect.targetRect || effect.rect;
}

function freshClipRect(effect) {
  if (effect.travelsToInspector && effect.targetToken) {
    const fresh = resolveInspectorTargetRect(effect.targetToken, effect.targetScope);
    if (fresh) return fresh;
  }
  return effect.clipRect || effect.targetRect || effect.rect;
}

function findInspectorImageElement(effects) {
  for (const effect of effects) {
    if (!effect.targetToken) continue;
    const img = resolveInspectorImageElement(effect.targetToken);
    if (img && img.complete && img.naturalWidth > 0) return img;
  }
  return null;
}

function ShaderCanvas({ effects }) {
  const canvasRef = useRef(null);
  const shaderEffects = useMemo(
    () => effects
      .filter((effect) => (
        effect.travelsToInspector
        && (
          effect.kind === EXILE_EFFECT_KIND
          || effect.kind === ANGELIC_DESTROY_EFFECT_KIND
        )
      ))
      .slice(0, MAX_SHADER_EFFECTS),
    [effects]
  );
  const effectsRef = useRef(shaderEffects);

  useEffect(() => {
    effectsRef.current = shaderEffects;
  }, [shaderEffects]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return undefined;
    const gl = canvas.getContext("webgl", {
      alpha: true,
      antialias: true,
      premultipliedAlpha: false,
      preserveDrawingBuffer: false,
    });
    if (!gl) {
      console.warn("[zone-move-effects] WebGL unavailable; exile shader trail disabled");
      return undefined;
    }

    let program = null;
    let frameId = 0;

    try {
      program = createShaderProgram(gl);
    } catch (error) {
      console.warn("[zone-move-effects] failed to compile exile shader:", error);
      return undefined;
    }

    const positionLocation = gl.getAttribLocation(program, "a_position");
    const resolutionLocation = gl.getUniformLocation(program, "u_resolution");
    const dprLocation = gl.getUniformLocation(program, "u_dpr");
    const timeLocation = gl.getUniformLocation(program, "u_time");
    const countLocation = gl.getUniformLocation(program, "u_count");
    const sourceRectsLocation = gl.getUniformLocation(program, "u_sourceRects[0]");
    const targetRectsLocation = gl.getUniformLocation(program, "u_targetRects[0]");
    const clipRectsLocation = gl.getUniformLocation(program, "u_clipRects[0]");
    const progressLocation = gl.getUniformLocation(program, "u_progress[0]");
    const seedLocation = gl.getUniformLocation(program, "u_seed[0]");
    const effectKindsLocation = gl.getUniformLocation(program, "u_effectKinds[0]");
    const accentColorsLocation = gl.getUniformLocation(program, "u_accentColors[0]");
    const inspectorImageLocation = gl.getUniformLocation(program, "u_inspectorImage");
    const inspectorRectLocation = gl.getUniformLocation(program, "u_inspectorRect");
    const inspectorReadyLocation = gl.getUniformLocation(program, "u_inspectorReady");
    const buffer = gl.createBuffer();

    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(
      gl.ARRAY_BUFFER,
      new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]),
      gl.STATIC_DRAW
    );
    gl.useProgram(program);
    gl.enableVertexAttribArray(positionLocation);
    gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 0, 0);
    gl.disable(gl.BLEND);

    // Inspector image texture: starts as a 1x1 transparent placeholder so
    // sampling is always defined. We CORS-load a mirror of the inspector's
    // <img.hover-art-foreground-image> (the in-DOM img isn't crossOrigin
    // tagged, so it can't be uploaded directly without tainting the canvas)
    // and copy the bitmap into the texture once it resolves.
    const inspectorTexture = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, inspectorTexture);
    gl.texImage2D(
      gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE,
      new Uint8Array([0, 0, 0, 0])
    );
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);

    let inspectorImageSrc = null;
    let pendingInspectorImage = null;

    const uploadImageToTexture = (img) => {
      try {
        gl.bindTexture(gl.TEXTURE_2D, inspectorTexture);
        gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true);
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);
        gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
        return true;
      } catch (error) {
        console.warn("[zone-move-effects] inspector image upload failed:", error);
        return false;
      }
    };

    const tryUploadInspectorImage = (imgElement) => {
      if (!imgElement) return;
      const src = imgElement.currentSrc || imgElement.src;
      if (!src || src === inspectorImageSrc) return;
      inspectorImageSrc = src;
      if (pendingInspectorImage) {
        pendingInspectorImage.onload = null;
        pendingInspectorImage.onerror = null;
        pendingInspectorImage = null;
      }

      // Visible Scryfall card images intentionally load without anonymous
      // CORS because the final cards.scryfall.io JPEG response does not send
      // ACAO headers. For WebGL sampling, try a CORS-tagged mirror only; if
      // the host rejects it, the visible DOM image still renders normally.
      if (
        imgElement.crossOrigin
        && imgElement.complete
        && imgElement.naturalWidth > 0
      ) {
        if (uploadImageToTexture(imgElement)) return;
      }

      const corsImg = new Image();
      corsImg.crossOrigin = "anonymous";
      corsImg.referrerPolicy = "no-referrer";
      corsImg.decoding = "async";
      corsImg.onload = () => {
        if (corsImg !== pendingInspectorImage) return;
        uploadImageToTexture(corsImg);
        pendingInspectorImage = null;
      };
      corsImg.onerror = () => {
        if (corsImg !== pendingInspectorImage) return;
        pendingInspectorImage = null;
      };
      pendingInspectorImage = corsImg;
      corsImg.src = src;
    };

    const resizeCanvas = () => {
      const dpr = window.devicePixelRatio || 1;
      const width = window.innerWidth;
      const height = window.innerHeight;
      canvas.width = Math.max(1, Math.floor(width * dpr));
      canvas.height = Math.max(1, Math.floor(height * dpr));
      canvas.style.width = `${width}px`;
      canvas.style.height = `${height}px`;
      gl.viewport(0, 0, canvas.width, canvas.height);
    };

    const render = (now) => {
      const activeEffects = effectsRef.current.slice(0, MAX_SHADER_EFFECTS);
      if (activeEffects.length === 0) {
        frameId = window.requestAnimationFrame(render);
        return;
      }

      const sourceRects = activeEffects.map((effect) => effect.rect);
      const targetRects = activeEffects.map(freshTargetRect);
      const clipRects = activeEffects.map(freshClipRect);

      const inspectorImgElement = findInspectorImageElement(activeEffects);
      if (inspectorImgElement) tryUploadInspectorImage(inspectorImgElement);
      const inspectorMaskElement = resolveLargestInspectorMaskElement(activeEffects);
      const inspectorMaskRect = inspectorMaskElement
        ? normalizeRect(inspectorMaskElement.getBoundingClientRect())
        : null;
      const inspectorActive = inspectorMaskRect != null;

      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.useProgram(program);
      gl.uniform2f(resolutionLocation, window.innerWidth, window.innerHeight);
      gl.uniform1f(dprLocation, window.devicePixelRatio || 1);
      gl.uniform1f(timeLocation, now / 1000);
      gl.uniform1i(countLocation, activeEffects.length);
      gl.uniform4fv(sourceRectsLocation, rectsToFloat32Array(sourceRects));
      gl.uniform4fv(targetRectsLocation, rectsToFloat32Array(targetRects));
      gl.uniform4fv(clipRectsLocation, rectsToFloat32Array(clipRects));
      gl.uniform1fv(progressLocation, progressUniformArray(activeEffects, now));
      gl.uniform1fv(seedLocation, seedUniformArray(activeEffects));
      gl.uniform1fv(effectKindsLocation, effectKindUniformArray(activeEffects));
      gl.uniform3fv(accentColorsLocation, accentColorUniformArray(activeEffects));
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, inspectorTexture);
      gl.uniform1i(inspectorImageLocation, 0);
      gl.uniform1f(inspectorReadyLocation, inspectorActive ? 1.0 : 0.0);
      if (inspectorActive) {
        gl.uniform4f(
          inspectorRectLocation,
          inspectorMaskRect.left,
          inspectorMaskRect.top,
          inspectorMaskRect.width,
          inspectorMaskRect.height,
        );
      } else {
        gl.uniform4f(inspectorRectLocation, 0, 0, 0, 0);
      }
      gl.drawArrays(gl.TRIANGLES, 0, 6);
      frameId = window.requestAnimationFrame(render);
    };

    resizeCanvas();
    window.addEventListener("resize", resizeCanvas);
    frameId = window.requestAnimationFrame(render);

    return () => {
      window.cancelAnimationFrame(frameId);
      window.removeEventListener("resize", resizeCanvas);
      if (pendingInspectorImage) {
        pendingInspectorImage.onload = null;
        pendingInspectorImage.onerror = null;
        pendingInspectorImage = null;
      }
      if (buffer) gl.deleteBuffer(buffer);
      if (inspectorTexture) gl.deleteTexture(inspectorTexture);
      if (program) gl.deleteProgram(program);
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
    };
  }, []);

  return <canvas ref={canvasRef} className="zone-move-effects-canvas zone-move-effects-canvas--shader" aria-hidden="true" />;
}

function ParticleExileCard({ effect }) {
  const name = String(effect.card?.name || "");
  const resolvedArtUrl = useScryfallImageUrl(name, "art_crop");
  const artUrl = effect.sourceImageUrl || resolvedArtUrl || null;
  const rect = effect.rect;
  const targetRect = effect.targetRect || rect;
  const sourceCenterX = rect.left + rect.width / 2;
  const sourceCenterY = rect.top + rect.height / 2;
  const targetCenterX = targetRect.left + targetRect.width / 2;
  const targetCenterY = targetRect.top + targetRect.height / 2;
  const style = useMemo(() => ({
    left: `${rect.left}px`,
    top: `${rect.top}px`,
    width: `${rect.width}px`,
    height: `${rect.height}px`,
    "--exile-target-x": `${targetCenterX - sourceCenterX}px`,
    "--exile-target-y": `${targetCenterY - sourceCenterY}px`,
    "--exile-target-scale-x": String(Math.max(0.24, targetRect.width / rect.width)),
    "--exile-target-scale-y": String(Math.max(0.24, targetRect.height / rect.height)),
    "--exile-tilt": `${hashText(effect.id) % 2 === 0 ? -4 : 4}deg`,
    "--exile-start-delay": `${effect.cssAnimationDelayMs ?? effect.startDelayMs ?? 0}ms`,
    "--exile-converge-x": `${effect.convergeOffsetX || 0}px`,
    "--exile-converge-y": `${effect.convergeOffsetY || 0}px`,
  }), [
    effect.id,
    rect.height,
    rect.left,
    rect.top,
    rect.width,
    effect.cssAnimationDelayMs,
    effect.startDelayMs,
    effect.convergeOffsetX,
    effect.convergeOffsetY,
    sourceCenterX,
    sourceCenterY,
    targetCenterX,
    targetCenterY,
    targetRect.height,
    targetRect.width,
  ]);

  if (!effect.includeSourceClone) return null;

  return (
    <div
      className={`zone-exile-effect ${effect.travelsToInspector ? "zone-exile-effect--to-inspector" : "zone-exile-effect--source-only"}`}
      style={style}
      aria-hidden="true"
    >
      <div className="zone-exile-halo" />
      <div className="zone-exile-card-proxy">
        {effect.sourceCloneHtml ? (
          <div
            className="zone-exile-source-clone"
            dangerouslySetInnerHTML={{ __html: effect.sourceCloneHtml }}
          />
        ) : artUrl ? (
          <img src={artUrl} alt="" draggable="false" referrerPolicy="no-referrer" />
        ) : null}
        <div className="zone-exile-card-frame" />
        <div className="zone-exile-card-dissolve" />
        <div className="zone-exile-card-whiteout" />
      </div>
    </div>
  );
}

function AngelicDestroyCard({ effect }) {
  const rect = effect.rect;
  const name = String(effect.card?.name || "");
  const targetRect = effect.targetRect || rect;
  const sourceCenterX = rect.left + rect.width / 2;
  const sourceCenterY = rect.top + rect.height / 2;
  const targetCenterX = targetRect.left + targetRect.width / 2;
  const targetCenterY = targetRect.top + targetRect.height / 2;
  const targetOffsetX = targetCenterX - sourceCenterX;
  const targetOffsetY = targetCenterY - sourceCenterY;
  const flightDistance = Math.hypot(targetOffsetX, targetOffsetY);
  const rise = Math.max(170, Math.min(360, rect.height * 2.25));
  const style = useMemo(() => ({
    left: `${rect.left}px`,
    top: `${rect.top}px`,
    width: `${rect.width}px`,
    height: `${rect.height}px`,
    "--destroy-target-x": `${targetOffsetX}px`,
    "--destroy-target-y": `${targetOffsetY}px`,
    "--destroy-flight-angle": `${Math.atan2(targetOffsetY, targetOffsetX)}rad`,
    "--destroy-trail-length": `${Math.max(42, Math.min(190, flightDistance * 0.24))}px`,
    "--destroy-accent-rgb": effect.accentRgb || "142, 211, 255",
    "--exile-rise": `${rise}px`,
    "--exile-tilt": `${hashText(effect.id) % 2 === 0 ? -5 : 5}deg`,
    "--exile-start-delay": `${effect.cssAnimationDelayMs ?? effect.startDelayMs ?? 0}ms`,
    "--exile-wing-flap-duration": `${460 + (hashText(effect.id) % 180)}ms`,
    "--exile-wing-flap-phase-left": `-${hashText(`${effect.id}:left`) % 280}ms`,
    "--exile-wing-flap-phase-right": `-${hashText(`${effect.id}:right`) % 280}ms`,
  }), [
    effect.cssAnimationDelayMs,
    effect.accentRgb,
    effect.id,
    effect.startDelayMs,
    rect.height,
    rect.left,
    rect.top,
    rect.width,
    rise,
    targetOffsetX,
    targetOffsetY,
    flightDistance,
  ]);

  if (!effect.includeSourceClone) return null;

  return (
    <div
      className={`zone-angelic-destroy-effect ${effect.preflight ? "zone-angelic-destroy-effect--preflight" : effect.travelsToInspector ? "zone-angelic-destroy-effect--to-inspector" : "zone-angelic-destroy-effect--source-only"}`}
      style={style}
      aria-hidden="true"
    >
      <div className="zone-exile-halo" />
      <div className="zone-destroy-tombstone-flight">
        <div className="zone-destroy-tombstone">
          <div className="zone-destroy-tombstone-name">{name}</div>
        </div>
      </div>
    </div>
  );
}

export default function ZoneMoveEffects() {
  const [effects, setEffects] = useState([]);
  const cleanupTimersRef = useRef([]);
  const pendingPollsRef = useRef(new Set());

  useEffect(() => {
    if (effects.length === 0) return undefined;
    document.body.classList.add("ironsmith-exile-animating");
    return () => {
      document.body.classList.remove("ironsmith-exile-animating");
    };
  }, [effects.length]);

  const addEffects = useCallback((nextEffects) => {
    setEffects((currentEffects) => [...currentEffects, ...nextEffects]);

    const now = performance.now();
    for (const effect of nextEffects) {
      const elapsed = now - effect.startedAt;
      const remaining = Math.max(120, effectDurationMs(effect) - elapsed + CLEANUP_TAIL_MS);
      const timerId = window.setTimeout(() => {
        setEffects((currentEffects) => currentEffects.filter((currentEffect) => currentEffect.id !== effect.id));
      }, remaining);
      cleanupTimersRef.current.push(timerId);
    }
  }, []);

  const spawnEffects = useCallback((rawEffects) => {
    const validEffects = (Array.isArray(rawEffects) ? rawEffects : [])
      .filter((effect) => effect && (
        effect.kind === EXILE_EFFECT_KIND
        || effect.kind === ANGELIC_DESTROY_EFFECT_KIND
      ));
    if (validEffects.length === 0) return;

    const requestedAt = performance.now();
    const exileEffects = validEffects.filter((effect) => effect.kind === EXILE_EFFECT_KIND);
    const angelicFlights = validEffects.filter((effect) => (
      effect.kind === ANGELIC_DESTROY_EFFECT_KIND
      && effect.travelsToInspector === true
      && effect.includeSourceClone !== false
    ));

    // Phase 1: per-card in-place dissolves spawn immediately so the cards are
    // visibly dissolving while the inspector forefront-image rect is still
    // being resolved.
    const sourceBurns = exileEffects
      .filter((effect) => effect.travelsToInspector !== true)
      .map((effect) => normalizeExileEffect(effect, null, undefined, { anchorAt: requestedAt }))
      .filter(Boolean);
    const preflightAngels = angelicFlights
      .map((effect) => normalizeAngelicDestroyEffect(
        {
          ...effect,
          id: `${deterministicEffectId(effect)}:preflight`,
          travelsToInspector: false,
          preflight: true,
        },
        null,
        undefined,
        { anchorAt: requestedAt },
      ))
      .filter(Boolean);
    if (sourceBurns.length > 0 || preflightAngels.length > 0) {
      addEffects([...sourceBurns, ...preflightAngels]);
    }

    // Flights are grouped so every card in the same dispatch shares one
    // polling loop. Exile dust remains anchored to the dispatch; angelic
    // destroy flights anchor once the inspector target is ready so their
    // per-card stagger is visible instead of being spent during target polling.
    const flightEffects = validEffects.filter((effect) => effect.travelsToInspector === true);
    if (flightEffects.length === 0) return;

    const flightsByGroup = new Map();
    for (const flight of flightEffects) {
      const key = flight.groupId || flight.targetToken || `solo:${flight.id}`;
      if (!flightsByGroup.has(key)) flightsByGroup.set(key, []);
      flightsByGroup.get(key).push(flight);
    }

    for (const flights of flightsByGroup.values()) {
      const targetToken = flights[0].targetToken;
      const targetScope = flights.some((flight) => flight.targetScope === "inspector")
        ? "inspector"
        : "foreground";
      const pollState = { cancelled: false, frameId: 0 };
      pendingPollsRef.current.add(pollState);

      let lastTargetRect = null;
      let stableTargetFrames = 0;
      let elementFirstSeenAt = null;

      const trySpawn = () => {
        if (pollState.cancelled) return;

        const targetRect = targetToken ? resolveInspectorTargetRect(targetToken, targetScope) : null;
        const now = performance.now();
        if (targetRect && elementFirstSeenAt === null) {
          elementFirstSeenAt = now;
        }
        const entrySettled = elementFirstSeenAt !== null
          && now - elementFirstSeenAt >= INSPECTOR_ENTRY_SETTLE_MS;
        const timedOut = now - requestedAt >= TARGET_WAIT_TIMEOUT_MS;
        if (targetRect && lastTargetRect) {
          const stable = (
            Math.abs(lastTargetRect.left - targetRect.left) < 0.75
            && Math.abs(lastTargetRect.top - targetRect.top) < 0.75
            && Math.abs(lastTargetRect.width - targetRect.width) < 0.75
            && Math.abs(lastTargetRect.height - targetRect.height) < 0.75
          );
          stableTargetFrames = stable ? stableTargetFrames + 1 : 0;
        }
        lastTargetRect = targetRect;

        if ((targetRect && entrySettled && stableTargetFrames >= 2) || timedOut) {
          pendingPollsRef.current.delete(pollState);
          const flightAnchorAt = performance.now();
          const normalized = flights
            .map((flight) => {
              const sourceRect = normalizeRect(flight.rect);
              if (!sourceRect) return null;
              const finalTargetRect = targetRect || fallbackTargetRect(sourceRect);
              if (!finalTargetRect) return null;
              if (flight.kind === ANGELIC_DESTROY_EFFECT_KIND) {
                return normalizeAngelicDestroyEffect(
                  flight,
                  finalTargetRect,
                  targetRect,
                  { anchorAt: flightAnchorAt },
                );
              }
              return normalizeExileEffect(
                flight,
                finalTargetRect,
                targetRect,
                { anchorAt: requestedAt },
              );
            })
            .filter(Boolean);
          for (const flight of flights) {
            if (flight.kind !== ANGELIC_DESTROY_EFFECT_KIND) continue;
            const preflightId = `${deterministicEffectId(flight)}:preflight`;
            const removeAt = flightAnchorAt + (Math.max(0, Number(flight.startDelayMs) || 0));
            const timerId = window.setTimeout(() => {
              setEffects((currentEffects) => (
                currentEffects.filter((currentEffect) => currentEffect.id !== preflightId)
              ));
            }, Math.max(0, removeAt - performance.now()));
            cleanupTimersRef.current.push(timerId);
          }
          if (normalized.length > 0) addEffects(normalized);
          return;
        }

        pollState.frameId = window.requestAnimationFrame(trySpawn);
      };

      pollState.frameId = window.requestAnimationFrame(trySpawn);
    }
  }, [addEffects]);

  useEffect(() => {
    const onZoneMoveEffects = (event) => {
      spawnEffects(event?.detail?.effects || []);
    };

    window.addEventListener("ironsmith:zone-move-effects", onZoneMoveEffects);
    return () => {
      window.removeEventListener("ironsmith:zone-move-effects", onZoneMoveEffects);
    };
  }, [spawnEffects]);

  useEffect(() => () => {
    for (const timerId of cleanupTimersRef.current) {
      window.clearTimeout(timerId);
    }
    cleanupTimersRef.current = [];
    for (const state of pendingPollsRef.current) {
      state.cancelled = true;
      if (state.frameId) window.cancelAnimationFrame(state.frameId);
    }
    pendingPollsRef.current.clear();
  }, []);

  if (effects.length === 0) return null;

  return (
    <div className="zone-move-effects-layer">
      <ShaderCanvas effects={effects} />
      {effects.map((effect) => (
        effect.kind === ANGELIC_DESTROY_EFFECT_KIND
          ? <AngelicDestroyCard key={effect.id} effect={effect} />
          : <ParticleExileCard key={effect.id} effect={effect} />
      ))}
    </div>
  );
}
