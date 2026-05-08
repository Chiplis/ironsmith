import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { scryfallImageUrl } from "@/lib/scryfall";

const EXILE_EFFECT_DURATION_MS = 2400;
const MAX_SHADER_EFFECTS = 4;
const TARGET_WAIT_TIMEOUT_MS = 420;
const MAX_INSPECTOR_SCALE = 2.8;

const VERTEX_SHADER_SOURCE = `
attribute vec2 a_position;
void main() {
  gl_Position = vec4(a_position, 0.0, 1.0);
}
`;

const FRAGMENT_SHADER_SOURCE = `
precision highp float;

uniform vec2 u_resolution;
uniform float u_time;
uniform int u_count;
uniform vec4 u_sourceRects[4];
uniform vec4 u_targetRects[4];
uniform float u_progress[4];
uniform float u_seed[4];

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

vec3 effectColor(vec2 p, vec4 src, vec4 dst, float progress, float seed) {
  vec2 srcC = src.xy + src.zw * 0.5;
  vec2 dstC = dst.xy + dst.zw * 0.5;
  float travel = easeInOut(saturate((progress - 0.22) / 0.46));
  vec2 head = mix(srcC, dstC, travel);
  float pathLength = max(length(dstC - srcC), 1.0);
  float pathT = saturate(length(head - srcC) / pathLength);
  float wingPhase = sin((progress * 18.0) + seed);
  float wingSpread = mix(src.z * 0.36, dst.z * 0.18, travel);
  float wingY = sin((p.x - head.x) / max(wingSpread, 1.0) * 3.14159 + wingPhase) * 18.0;
  float wingDist = abs((p.y - head.y) - wingY) + abs(abs(p.x - head.x) - wingSpread) * 0.28;
  float wingGlow = exp(-wingDist * 0.035) * smoothstep(0.22, 0.34, progress) * smoothstep(0.72, 0.44, progress);

  float headDist = length(p - head);
  float sourceIgnition = rectMask(p, src, 14.0) * sin(saturate(progress / 0.22) * 3.14159);
  float headGlow = exp(-headDist * 0.022) * smoothstep(0.22, 0.34, progress) * smoothstep(0.74, 0.48, progress);
  float core = exp(-headDist * 0.04) * smoothstep(0.2, 0.32, progress);

  float targetFillProgress = saturate((progress - 0.52) / 0.36);
  float targetMask = rectMask(p, dst, 18.0);
  float targetRing = ringRect(p, dst, 18.0 + 20.0 * sin(targetFillProgress * 3.14159), 22.0);
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

  vec3 gold = vec3(1.0, 0.78, 0.28);
  vec3 blue = vec3(0.48, 0.78, 1.0);
  vec3 white = vec3(1.0);
  vec3 color = vec3(0.0);
  color += mix(white, gold, 0.24) * sourceIgnition * 0.95;
  color += mix(gold, blue, pathT) * headGlow * 0.34;
  color += white * core * 0.92;
  color += white * whiteFill * 1.35;
  color += gold * targetRing * targetFillProgress * 0.7;
  color += mix(white, blue, 0.35) * wingGlow * 0.45;
  color += mix(gold, white, 0.55) * sparkle * 0.9;
  color += mix(blue, white, 0.3) * targetBloom * 0.36;
  return color;
}

void main() {
  vec2 p = vec2(gl_FragCoord.x, u_resolution.y - gl_FragCoord.y);
  vec3 color = vec3(0.0);

  for (int i = 0; i < 4; i++) {
    if (i >= u_count) break;
    color += effectColor(p, u_sourceRects[i], u_targetRects[i], u_progress[i], u_seed[i]);
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

function resolveInspectorTargetRect(targetToken) {
  if (!targetToken || typeof document === "undefined") return null;
  const escapedToken = escapeAttributeValue(targetToken);
  const target = document.querySelector(
    `.hover-art-stage[data-zone-transition-token="${escapedToken}"] .hover-art-foreground-crop, ` +
    `.hover-art-stage[data-zone-transition-token="${escapedToken}"] .hover-art-full-art-crop, ` +
    `.ironsmith-inspector-shell[data-zone-transition-token="${escapedToken}"] .hover-art-foreground-crop, ` +
    `.ironsmith-inspector-shell[data-zone-transition-token="${escapedToken}"] .hover-art-full-art-crop, ` +
    `.hover-art-stage[data-zone-transition-token="${escapedToken}"], ` +
    `.ironsmith-inspector-shell[data-zone-transition-token="${escapedToken}"] .hover-art-stage`
  );
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

function normalizeEffect(rawEffect, targetRect) {
  if (!rawEffect || rawEffect.kind !== "angelic-exile") return null;
  const rect = normalizeRect(rawEffect.rect);
  if (!rect) return null;
  const travelsToInspector = rawEffect.travelsToInspector === true;
  const resolvedTargetRect = travelsToInspector
    ? capTargetRectToSourceScale(rect, targetRect || fallbackTargetRect(rect))
    : rect;

  return {
    id: rawEffect.id || `exile:${Date.now()}:${Math.random()}`,
    kind: "angelic-exile",
    rect,
    targetRect: resolvedTargetRect,
    travelsToInspector,
    includeSourceClone: rawEffect.includeSourceClone !== false,
    targetToken: rawEffect.targetToken || null,
    sourceCloneHtml: rawEffect.sourceCloneHtml || null,
    sourceImageUrl: rawEffect.sourceImageUrl || null,
    card: rawEffect.card || {},
    seed: (hashText(`${rawEffect.id}:${rawEffect.card?.name || ""}`) % 10000) / 10000,
    startedAt: performance.now(),
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
  const fragmentShader = compileShader(gl, gl.FRAGMENT_SHADER, FRAGMENT_SHADER_SOURCE);
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

function rectUniformArray(effects, pickRect) {
  const values = new Float32Array(MAX_SHADER_EFFECTS * 4);
  for (let index = 0; index < MAX_SHADER_EFFECTS; index += 1) {
    const rect = effects[index] ? pickRect(effects[index]) : null;
    values[index * 4] = rect?.left || 0;
    values[index * 4 + 1] = rect?.top || 0;
    values[index * 4 + 2] = rect?.width || 0;
    values[index * 4 + 3] = rect?.height || 0;
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
    if (!gl) return undefined;

    let program = null;
    let frameId = 0;

    try {
      program = createShaderProgram(gl);
    } catch {
      return undefined;
    }

    const positionLocation = gl.getAttribLocation(program, "a_position");
    const resolutionLocation = gl.getUniformLocation(program, "u_resolution");
    const timeLocation = gl.getUniformLocation(program, "u_time");
    const countLocation = gl.getUniformLocation(program, "u_count");
    const sourceRectsLocation = gl.getUniformLocation(program, "u_sourceRects[0]");
    const targetRectsLocation = gl.getUniformLocation(program, "u_targetRects[0]");
    const progressLocation = gl.getUniformLocation(program, "u_progress[0]");
    const seedLocation = gl.getUniformLocation(program, "u_seed[0]");
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
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE);

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
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.useProgram(program);
      gl.uniform2f(resolutionLocation, window.innerWidth, window.innerHeight);
      gl.uniform1f(timeLocation, now / 1000);
      gl.uniform1i(countLocation, activeEffects.length);
      gl.uniform4fv(sourceRectsLocation, rectUniformArray(activeEffects, (effect) => effect.rect));
      gl.uniform4fv(targetRectsLocation, rectUniformArray(activeEffects, (effect) => effect.targetRect || effect.rect));
      gl.uniform1fv(progressLocation, progressUniformArray(activeEffects, now));
      gl.uniform1fv(seedLocation, seedUniformArray(activeEffects));
      gl.drawArrays(gl.TRIANGLES, 0, 6);
      frameId = window.requestAnimationFrame(render);
    };

    resizeCanvas();
    window.addEventListener("resize", resizeCanvas);
    frameId = window.requestAnimationFrame(render);

    return () => {
      window.cancelAnimationFrame(frameId);
      window.removeEventListener("resize", resizeCanvas);
      if (buffer) gl.deleteBuffer(buffer);
      if (program) gl.deleteProgram(program);
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
  }), [
    effect.id,
    rect.height,
    rect.left,
    rect.top,
    rect.width,
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
  const pendingFrameRefs = useRef([]);

  const addEffects = useCallback((nextEffects) => {
    setEffects((currentEffects) => [...currentEffects, ...nextEffects]);

    for (const effect of nextEffects) {
      const timerId = window.setTimeout(() => {
        setEffects((currentEffects) => currentEffects.filter((currentEffect) => currentEffect.id !== effect.id));
      }, EXILE_EFFECT_DURATION_MS + 220);
      cleanupTimersRef.current.push(timerId);
    }
  }, []);

  const spawnEffects = useCallback((rawEffects) => {
    for (const rawEffect of Array.isArray(rawEffects) ? rawEffects : []) {
      if (!rawEffect || rawEffect.kind !== "angelic-exile") continue;

      if (rawEffect.travelsToInspector !== true) {
        const normalized = normalizeEffect(rawEffect, null);
        if (normalized) addEffects([normalized]);
        continue;
      }

      const requestedAt = performance.now();
      let cancelled = false;
      let frameId = 0;
      let lastTargetRect = null;
      let stableTargetFrames = 0;

      const trySpawn = () => {
        if (cancelled) return;

        const sourceRect = normalizeRect(rawEffect.rect);
        const targetRect = resolveInspectorTargetRect(rawEffect.targetToken);
        const timedOut = performance.now() - requestedAt >= TARGET_WAIT_TIMEOUT_MS;
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

        if (!sourceRect || (targetRect && stableTargetFrames >= 2) || timedOut) {
          const normalized = normalizeEffect(rawEffect, targetRect || (sourceRect ? fallbackTargetRect(sourceRect) : null));
          if (normalized) addEffects([normalized]);
          return;
        }

        frameId = window.requestAnimationFrame(trySpawn);
        pendingFrameRefs.current.push(frameId);
      };

      frameId = window.requestAnimationFrame(trySpawn);
      pendingFrameRefs.current.push(frameId);
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
    for (const frameId of pendingFrameRefs.current) {
      window.cancelAnimationFrame(frameId);
    }
    pendingFrameRefs.current = [];
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
