import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { scryfallImageUrl } from "@/lib/scryfall";

const EXILE_EFFECT_DURATION_MS = 2400;
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

vec3 effectColor(vec2 p, vec4 src, vec4 dst, vec4 clip, float progress, float seed) {
  vec2 srcC = src.xy + src.zw * 0.5;
  vec2 dstC = dst.xy + dst.zw * 0.5;
  float travel = easeInOut(saturate((progress - 0.22) / 0.46));
  vec2 head = mix(srcC, dstC, travel);
  float pathLength = max(length(dstC - srcC), 1.0);
  float pathT = saturate(length(head - srcC) / pathLength);

  // Late-phase dissolve: as the comet nears the target, fade out the tight
  // head/wings entirely so multiple flights converging on the same rect
  // don't accumulate into a bright "TV turn-off" dot at the landing point.
  float dissolveBlend = smoothstep(0.62, 0.94, progress);
  float cometAttn = 1.0 - dissolveBlend * 0.98;

  float wingPhase = sin((progress * 18.0) + seed);
  float wingSpread = mix(src.z * 0.36, dst.z * 0.18, travel);
  float wingY = sin((p.x - head.x) / max(wingSpread, 1.0) * 3.14159 + wingPhase) * 18.0;
  float wingDist = abs((p.y - head.y) - wingY) + abs(abs(p.x - head.x) - wingSpread) * 0.28;
  float wingGlow = exp(-wingDist * 0.035) * smoothstep(0.22, 0.34, progress) * smoothstep(0.72, 0.44, progress) * cometAttn;

  float headDist = length(p - head);
  float sourceIgnition = rectMask(p, src, 14.0) * sin(saturate(progress / 0.22) * 3.14159);
  float headGlow = exp(-headDist * 0.022) * smoothstep(0.22, 0.34, progress) * smoothstep(0.74, 0.48, progress) * cometAttn;
  float core = exp(-headDist * 0.04) * smoothstep(0.2, 0.32, progress) * cometAttn;

  float targetFillProgress = saturate((progress - 0.52) / 0.36);
  float clipMask = rectMask(p, clip, 5.0);
  float targetMask = rectMask(p, dst, 14.0) * clipMask;
  float targetRing = ringRect(p, dst, 8.0 + 12.0 * sin(targetFillProgress * 3.14159), 12.0) * clipMask;
  float whiteFill = targetMask * sin(targetFillProgress * 3.14159);
  float targetBloom = targetMask * smoothstep(0.42, 0.72, progress) * smoothstep(1.0, 0.68, progress);

  vec2 sparkleSpace = (p - head) + seed * 97.0;
  vec2 cell = floor(sparkleSpace / 18.0);
  vec2 local = fract(sparkleSpace / 18.0) - 0.5;
  float sparkleSeed = hash(cell + seed);
  float sparkleTime = fract(sparkleSeed + progress * 1.8);
  float sparkle = smoothstep(0.036, 0.0, length(local)) * sin(sparkleTime * 3.14159);
  sparkle *= smoothstep(0.18, 0.46, progress) * smoothstep(0.72, 0.48, progress) * step(0.72, sparkleSeed);
  sparkle *= smoothstep(190.0, 24.0, headDist);

  // Particle cloud: dense scatter inside the container during dissolve. Each
  // pixel hashes to a per-cell pulse so the cloud reads as individual particles
  // rather than a uniform glow.
  vec2 cloudSpace = (p - dstC) + seed * 73.0;
  vec2 cloudCell = floor(cloudSpace / 7.0);
  vec2 cloudLocal = fract(cloudSpace / 7.0) - 0.5;
  float cloudSeed = hash(cloudCell * 1.31 + seed * 4.0);
  float cloudPulse = fract(cloudSeed + progress * 2.4);
  float cloudParticle = smoothstep(0.045, 0.0, length(cloudLocal) * (1.2 + cloudSeed * 0.4))
                      * sin(cloudPulse * 3.14159);
  cloudParticle *= step(0.5, cloudSeed) * dissolveBlend * clipMask;

  vec3 gold = vec3(1.0, 0.78, 0.28);
  vec3 blue = vec3(0.48, 0.78, 1.0);
  vec3 white = vec3(1.0);
  vec3 color = vec3(0.0);
  color += mix(white, gold, 0.24) * sourceIgnition * 0.95;
  color += mix(gold, blue, pathT) * headGlow * 0.34;
  float landedClip = mix(1.0, clipMask, smoothstep(0.66, 0.78, progress));
  color += white * core * 0.92 * landedClip;
  color += white * whiteFill * 0.45;
  // Suppress the gold target ring inside the inspector area — main()'s
  // image-reconstruction pass owns the visual there. Outside the inspector
  // (no texture loaded), keep the ring so the comet still has a landing
  // flourish.
  color += gold * targetRing * targetFillProgress * 0.55 * (1.0 - u_inspectorReady * 0.9);
  color += mix(white, blue, 0.35) * wingGlow * 0.45 * landedClip;
  color += mix(gold, white, 0.55) * sparkle * 0.9;
  color += mix(blue, white, 0.3) * targetBloom * 0.22;
  // Per-effect particle dissolve: provides a fallback particle storm when
  // the inspector image hasn't loaded yet (CORS load pending or failed).
  // When the texture is ready, main() runs the image-reconstruction pass
  // instead, so we damp this contribution accordingly. Even the fallback is
  // now subtle so it doesn't read as a hot white blob.
  float cloudFallback = mix(0.55, 0.18, u_inspectorReady);
  color += mix(gold, white, 0.55) * cloudParticle * cloudFallback;
  color += mix(white, blue, 0.4) * targetMask * dissolveBlend * 0.12;
  return color;
}

void main() {
  vec2 pixel = gl_FragCoord.xy / max(u_dpr, 0.0001);
  vec2 p = vec2(pixel.x, u_resolution.y - pixel.y);
  vec3 color = vec3(0.0);
  // Reveal drives "how much of the image has been sampled into the canvas".
  // Window peaks during dissolve and fades back to 0 by progress = 1 so the
  // shader stops painting and the natural <img> underneath takes over.
  float globalReveal = 0.0;
  float globalWindow = 0.0;

  for (int i = 0; i < MAX_EFFECTS; i++) {
    if (i >= u_count) break;
    color += effectColor(p, u_sourceRects[i], u_targetRects[i], u_clipRects[i], u_progress[i], u_seed[i]);
    float pi = u_progress[i];
    globalReveal = max(globalReveal, smoothstep(0.62, 0.92, pi));
    // Keep globalWindow at 1.0 once reached, so the inspector area stays
    // cleanly replaced by reconColor (black-passthrough once revealed) rather
    // than fading back through comet residuals — those residuals are tiny
    // (~5-7% brightness from core/targetRing) but become visible during a
    // ramp-down because they screen-blend over the image. The cleanup timer
    // unmounts the canvas shortly after progress = 1, which is the actual
    // hand-off to the natural <img>.
    globalWindow = max(globalWindow, smoothstep(0.62, 0.78, pi));
  }

  // Center-out white-to-passthrough mask. We don't sample the inspector image
  // at all — instead we paint white where the reveal hasn't reached yet, and
  // black inside the reveal radius. With mix-blend-mode: screen, black is a
  // no-op so the underlying <img> shows through unmodified. This avoids the
  // double-exposure / filter appearance that came from screen-blending
  // sampled imageColor over the same pixels in the natural <img>.
  if (u_inspectorReady > 0.5
      && u_inspectorRect.z > 0.0
      && u_inspectorRect.w > 0.0
      && globalWindow > 0.0) {
    vec2 uv = (p - u_inspectorRect.xy) / u_inspectorRect.zw;
    if (uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0) {
      vec2 cell = floor(p / 4.5);
      float cellSeed = hash(cell + 11.0);

      float centerDist = length(uv - 0.5);
      float jitter = (cellSeed - 0.5) * 0.06;
      float revealRadius = globalReveal * 0.92;
      float pixelReveal = saturate(1.0 - smoothstep(
        revealRadius - 0.07 + jitter,
        revealRadius + 0.04 + jitter,
        centerDist
      ));

      vec2 cellLocal = fract(p / 4.5) - 0.5;
      float sparkShape = smoothstep(0.08, 0.0, length(cellLocal));
      float frontier = sparkShape * step(0.55, cellSeed)
                     * sin(saturate(pixelReveal) * 3.14159);

      // (1 - pixelReveal) is white outside the reveal, black inside. Add a
      // subtle white frontier sparkle along the leading edge.
      vec3 reconColor = vec3(1.0 - pixelReveal);
      reconColor += vec3(1.0) * frontier * 0.32;

      color = mix(color, reconColor, globalWindow);
    }
  }

  float alpha = saturate(max(max(color.r, color.g), color.b));
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
  const seedSource = `exile:${rawEffect.card?.name || "?"}:${rawEffect.objectId ?? "?"}:${rawEffect.targetToken || "?"}`;
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

function resolveInspectorTargetElement(targetToken) {
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
  let bestElement = null;
  let bestArea = 0;
  for (const selector of cropSelectors) {
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

  const stageMatches = document.querySelectorAll(
    `.hover-art-stage[data-zone-transition-token="${escapedToken}"], ` +
    `.ironsmith-inspector-shell[data-zone-transition-token="${escapedToken}"] .hover-art-stage`
  );
  for (const candidate of stageMatches) {
    if (!isElementVisuallyAvailable(candidate)) continue;
    const rect = candidate.getBoundingClientRect();
    const area = rect.width * rect.height;
    if (area > bestArea) {
      bestArea = area;
      bestElement = candidate;
    }
  }
  return bestElement;
}

function resolveInspectorTargetRect(targetToken) {
  const target = resolveInspectorTargetElement(targetToken);
  return normalizeRect(target?.getBoundingClientRect?.());
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

function normalizeEffect(rawEffect, targetRect, clipRect = targetRect, options = {}) {
  if (!rawEffect || rawEffect.kind !== "angelic-exile") return null;
  const rect = normalizeRect(rawEffect.rect);
  if (!rect) return null;
  const travelsToInspector = rawEffect.travelsToInspector === true;
  const resolvedTargetRect = travelsToInspector
    ? capTargetRectToSourceScale(rect, targetRect || fallbackTargetRect(rect))
    : rect;
  const resolvedClipRect = travelsToInspector
    ? normalizeRect(clipRect || targetRect || resolvedTargetRect)
    : rect;
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

  // Per-effect wing flap rhythm: deterministic from the id hash so each angel
  // beats its wings at its own pace and the right wing is slightly out of
  // phase with the left.
  const flapHash = hashText(`flap:${id}`);
  const wingFlapDurationMs = 320 + (flapHash % 360);
  const wingFlapPhaseMs = -((flapHash >>> 8) % wingFlapDurationMs);
  const wingFlapPhaseRightMs = wingFlapPhaseMs - Math.round(wingFlapDurationMs * 0.18);

  return {
    id,
    kind: "angelic-exile",
    rect,
    targetRect: resolvedTargetRect,
    clipRect: resolvedClipRect || resolvedTargetRect,
    travelsToInspector,
    includeSourceClone: rawEffect.includeSourceClone !== false,
    targetToken: rawEffect.targetToken || null,
    sourceCloneHtml: rawEffect.sourceCloneHtml || null,
    sourceImageUrl: rawEffect.sourceImageUrl || null,
    card: rawEffect.card || {},
    seed: (hashText(`${id}:${rawEffect.card?.name || ""}`) % 10000) / 10000,
    startDelayMs,
    startedAt,
    cssAnimationDelayMs,
    convergeOffsetX,
    convergeOffsetY,
    wingFlapDurationMs,
    wingFlapPhaseMs,
    wingFlapPhaseRightMs,
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

function progressUniformArray(effects, now) {
  const values = new Float32Array(MAX_SHADER_EFFECTS);
  for (let index = 0; index < MAX_SHADER_EFFECTS; index += 1) {
    const effect = effects[index];
    values[index] = effect ? clamp((now - effect.startedAt) / EXILE_EFFECT_DURATION_MS, 0, 1) : 0;
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

function freshTargetRect(effect) {
  if (effect.travelsToInspector && effect.targetToken) {
    const fresh = resolveInspectorTargetRect(effect.targetToken);
    if (fresh) return capTargetRectToSourceScale(effect.rect, fresh);
  }
  return effect.targetRect || effect.rect;
}

function freshClipRect(effect) {
  if (effect.travelsToInspector && effect.targetToken) {
    const fresh = resolveInspectorTargetRect(effect.targetToken);
    if (fresh) return fresh;
  }
  return effect.clipRect || effect.targetRect || effect.rect;
}

function findInspectorImageElement(effects) {
  for (const effect of effects) {
    if (!effect.targetToken) continue;
    const targetEl = resolveInspectorTargetElement(effect.targetToken);
    if (!targetEl) continue;
    let img = null;
    if (targetEl.tagName === "IMG") {
      img = targetEl;
    } else if (typeof targetEl.querySelector === "function") {
      img = targetEl.querySelector("img.hover-art-foreground-image")
        || targetEl.querySelector("img");
    }
    if (img && img.complete && img.naturalWidth > 0) return img;
  }
  return null;
}

function ShaderCanvas({ effects }) {
  const canvasRef = useRef(null);
  const shaderEffects = useMemo(
    () => effects.filter((effect) => effect.travelsToInspector).slice(0, MAX_SHADER_EFFECTS),
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

    let inspectorImageReady = false;
    let inspectorImageSrc = null;
    let pendingInspectorImage = null;

    const uploadImageToTexture = (img) => {
      try {
        gl.bindTexture(gl.TEXTURE_2D, inspectorTexture);
        gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true);
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);
        gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
        inspectorImageReady = true;
        return true;
      } catch (error) {
        console.warn("[zone-move-effects] inspector image upload failed:", error);
        inspectorImageReady = false;
        return false;
      }
    };

    const tryUploadInspectorImage = (imgElement) => {
      if (!imgElement) return;
      const src = imgElement.currentSrc || imgElement.src;
      if (!src || src === inspectorImageSrc) return;
      inspectorImageSrc = src;
      inspectorImageReady = false;
      if (pendingInspectorImage) {
        pendingInspectorImage.onload = null;
        pendingInspectorImage.onerror = null;
        pendingInspectorImage = null;
      }

      // The in-DOM <img> already has crossOrigin="anonymous", so when the
      // browser has finished loading it we can sample directly without an
      // extra fetch. If it's still loading (or never reached crossOrigin
      // mode for some reason), kick off a CORS-tagged mirror as a fallback.
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
        inspectorImageReady = false;
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
      const inspectorImgRect = inspectorImgElement
        ? normalizeRect(inspectorImgElement.getBoundingClientRect())
        : null;
      const inspectorActive = inspectorImageReady && inspectorImgRect != null;

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
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, inspectorTexture);
      gl.uniform1i(inspectorImageLocation, 0);
      gl.uniform1f(inspectorReadyLocation, inspectorActive ? 1.0 : 0.0);
      if (inspectorActive) {
        gl.uniform4f(
          inspectorRectLocation,
          inspectorImgRect.left,
          inspectorImgRect.top,
          inspectorImgRect.width,
          inspectorImgRect.height,
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
      const lose = gl.getExtension("WEBGL_lose_context");
      if (lose) lose.loseContext();
    };
  }, []);

  return <canvas ref={canvasRef} className="zone-move-effects-canvas zone-move-effects-canvas--shader" aria-hidden="true" />;
}

function AngelicExileCard({ effect }) {
  const name = String(effect.card?.name || "");
  const artUrl = effect.sourceImageUrl || (name ? scryfallImageUrl(name, "art_crop") : null);
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
    "--exile-wing-flap-duration": `${effect.wingFlapDurationMs ?? 520}ms`,
    "--exile-wing-flap-phase-left": `${effect.wingFlapPhaseMs ?? 0}ms`,
    "--exile-wing-flap-phase-right": `${effect.wingFlapPhaseRightMs ?? 0}ms`,
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
    effect.wingFlapDurationMs,
    effect.wingFlapPhaseMs,
    effect.wingFlapPhaseRightMs,
    sourceCenterX,
    sourceCenterY,
    targetCenterX,
    targetCenterY,
    targetRect.height,
    targetRect.width,
  ]);

  return (
    <div
      className={`zone-exile-effect ${effect.travelsToInspector ? "zone-exile-effect--to-inspector" : "zone-exile-effect--source-only"}`}
      style={style}
      aria-hidden="true"
    >
      <div className="zone-exile-halo" />
      {effect.travelsToInspector ? (
        <div className="zone-exile-wings">
          <div className="zone-exile-wing zone-exile-wing--left">
            <span />
            <span />
            <span />
          </div>
          <div className="zone-exile-wing zone-exile-wing--right">
            <span />
            <span />
            <span />
          </div>
        </div>
      ) : null}
      {effect.includeSourceClone ? (
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
          <div className="zone-exile-card-whiteout" />
        </div>
      ) : null}
      {effect.travelsToInspector ? <div className="zone-exile-final-spark" /> : null}
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
      const remaining = Math.max(120, EXILE_EFFECT_DURATION_MS - elapsed + CLEANUP_TAIL_MS);
      const timerId = window.setTimeout(() => {
        setEffects((currentEffects) => currentEffects.filter((currentEffect) => currentEffect.id !== effect.id));
      }, remaining);
      cleanupTimersRef.current.push(timerId);
    }
  }, []);

  const spawnEffects = useCallback((rawEffects) => {
    const validEffects = (Array.isArray(rawEffects) ? rawEffects : [])
      .filter((effect) => effect && effect.kind === "angelic-exile");
    if (validEffects.length === 0) return;

    const requestedAt = performance.now();

    // Phase 1: per-card in-place dissolves spawn immediately so the cards are
    // visibly dissolving while the inspector forefront-image rect is still
    // being resolved.
    const sourceBurns = validEffects
      .filter((effect) => effect.travelsToInspector !== true)
      .map((effect) => normalizeEffect(effect, null, undefined, { anchorAt: requestedAt }))
      .filter(Boolean);
    if (sourceBurns.length > 0) addEffects(sourceBurns);

    // Phase 2: flights are grouped so every card in the same dispatch shares
    // one polling loop and one anchor time, guaranteeing they all leave their
    // origins simultaneously and therefore arrive at the shared target
    // simultaneously (longer paths = visibly faster).
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
      const pollState = { cancelled: false, frameId: 0 };
      pendingPollsRef.current.add(pollState);

      let lastTargetRect = null;
      let stableTargetFrames = 0;
      let elementFirstSeenAt = null;

      const trySpawn = () => {
        if (pollState.cancelled) return;

        const targetRect = targetToken ? resolveInspectorTargetRect(targetToken) : null;
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
          const normalized = flights
            .map((flight) => {
              const sourceRect = normalizeRect(flight.rect);
              if (!sourceRect) return null;
              const finalTargetRect = targetRect || fallbackTargetRect(sourceRect);
              if (!finalTargetRect) return null;
              return normalizeEffect(
                flight,
                finalTargetRect,
                targetRect,
                { anchorAt: requestedAt },
              );
            })
            .filter(Boolean);
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
        <AngelicExileCard key={effect.id} effect={effect} />
      ))}
    </div>
  );
}
