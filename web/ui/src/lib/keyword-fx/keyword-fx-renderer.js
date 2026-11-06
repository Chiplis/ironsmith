import { prepareShaderProgram } from "../shader-program.js";

// One WebGL2 context paints every battlefield card's keyword aura. Each aura
// layer is a tile of this canvas; the scheduler copies tiles into small 2D
// canvases that live inside the cards, so the cards' own transforms, tapping,
// hover scale and stacking order all apply to the effect for free.

import { FX } from "./flags.js";

export { FX, BACK_FX } from "./flags.js";

const vertex = `#version 300 es
layout(location=0) in vec2 corner;
out vec2 v_uv;
void main() {
  v_uv = corner * 0.5 + 0.5;
  gl_Position = vec4(corner, 0.0, 1.0);
}`;

const fragment = `#version 300 es
precision highp float;
precision highp int;
in vec2 v_uv;
out vec4 outColor;
uniform vec2 u_size;      // tile size, css px
uniform vec2 u_card;      // card size, css px
uniform float u_time;     // seconds
uniform float u_seed;
uniform float u_lift;     // current flying lift, css px
uniform float u_drift;    // current flying sideways drift, css px
uniform int u_flags;
uniform int u_layer;      // 0 = behind the card, 1 = over it
uniform vec3 u_protection;

const float TAU = 6.2831853;

float hash(vec2 p) { p = fract(p * vec2(123.34, 456.21)); p += dot(p, p + 45.32); return fract(p.x * p.y); }
float noise(vec2 p) {
  vec2 i = floor(p), f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2(1, 0)), u.x), mix(hash(i + vec2(0, 1)), hash(i + vec2(1, 1)), u.x), u.y);
}
float fbm(vec2 p) {
  float v = 0.0, a = 0.5;
  mat2 r = mat2(0.8, 0.6, -0.6, 0.8);
  for (int i = 0; i < 5; i++) { v += a * noise(p); p = r * p * 2.03 + 11.7; a *= 0.5; }
  return v;
}
float roundRect(vec2 q, vec2 halfSize, float radius) {
  vec2 d = abs(q) - halfSize + radius;
  return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0) - radius;
}
// Distance from a hex cell's edge (0 on the edge, 0.5 at the centre); id is per cell.
vec3 hexCell(vec2 p) {
  const vec2 r = vec2(1.0, 1.7320508);
  vec2 h = r * 0.5;
  vec2 a = mod(p, r) - h;
  vec2 b = mod(p - h, r) - h;
  vec2 g = dot(a, a) < dot(b, b) ? a : b;
  vec2 ap = abs(g);
  float hd = max(dot(ap, normalize(r)), ap.x);
  return vec3(0.5 - hd, p - g);
}
void over(inout vec4 acc, vec3 rgb, float alpha) {
  alpha = clamp(alpha, 0.0, 1.0);
  acc.rgb += rgb * alpha * (1.0 - acc.a);
  acc.a += alpha * (1.0 - acc.a);
}
void add(inout vec4 acc, vec3 rgb, float alpha) {
  alpha = clamp(alpha, 0.0, 1.0);
  acc.rgb += rgb * alpha;
  acc.a = clamp(acc.a + alpha * 0.6, 0.0, 1.0);
}

void main() {
  vec2 px = vec2(v_uv.x, 1.0 - v_uv.y) * u_size;
  vec2 q = px - u_size * 0.5;                 // css px from the card centre
  vec2 halfSize = u_card * 0.5;
  float unit = min(halfSize.x, halfSize.y);
  float radius = unit * 0.09;
  float d = roundRect(q, halfSize, radius);       // <0 inside the card
  float angle = atan(q.y, q.x);
  float t = u_time + u_seed * 37.0;
  // A position that runs once around the silhouette, in css px.
  float around = (angle / TAU + 0.5) * 2.0 * (u_card.x + u_card.y);
  vec4 acc = vec4(0.0);

  if (u_layer == 0) {
    if ((u_flags & ${FX.flying}) != 0) {
      // The card floats; its shadow stays on the table below it, so it
      // slides against the card's drift.
      float drop = u_lift + 5.0;
      float blur = 7.0 + u_lift * 0.9;
      float s = roundRect(q - vec2(u_lift * 0.35 - u_drift, drop), halfSize * 0.97, radius);
      float shadow = 1.0 - smoothstep(-blur, blur, s);
      over(acc, vec3(0.0), shadow * (0.62 - u_lift * 0.022));
    }
    if ((u_flags & ${FX.attacking}) != 0) {
      float outside = max(d, 0.0);
      float flicker = 0.75 + 0.25 * noise(vec2(around * 0.05, t * 3.0));
      add(acc, vec3(1.0, 0.28, 0.08), exp(-outside / 9.0) * step(0.0, d) * 0.7 * flicker);
    }
    if ((u_flags & ${FX.blocking}) != 0) {
      float outside = max(d, 0.0);
      float pulse = 0.7 + 0.3 * sin(t * 2.4);
      add(acc, vec3(0.35, 0.62, 1.0), exp(-outside / 8.0) * step(0.0, d) * 0.6 * pulse);
    }
    outColor = acc;
    return;
  }

  vec2 cardUv = q / unit;                     // ~[-1,1] on the short axis

  if ((u_flags & ${FX.shroud}) != 0) {
    // Shroud: a veil of fog curls slowly up through the card, hiding it from targeting.
    vec2 p = cardUv * 1.25;
    vec2 warp = vec2(fbm(p + vec2(0.0, t * 0.12)), fbm(p + vec2(5.2, 1.3) - vec2(t * 0.08, 0.0)));
    float swirl = fbm(p + 2.2 * warp + vec2(t * 0.05, t * 0.22));
    float wisps = smoothstep(0.38, 0.78, swirl);
    float body = 1.0 - smoothstep(-unit * 0.05, unit * 0.16, d);
    float lower = 0.6 + 0.4 * smoothstep(-0.6, 0.9, cardUv.y / max(1.0, halfSize.y / unit));
    // A faint dimming veil under the wisps.
    over(acc, vec3(0.05, 0.06, 0.10), body * 0.16);
    vec3 tone = mix(vec3(0.66, 0.70, 0.84), vec3(0.96, 0.97, 1.0), smoothstep(0.55, 0.92, swirl));
    over(acc, tone, wisps * body * lower * 0.62);
  }

  if ((u_flags & ${FX.hexproof}) != 0) {
    vec3 tone = vec3(0.36, 0.93, 0.62);
    float scale = 3.6;
    vec3 cell = hexCell(cardUv * scale + vec2(0.0, t * 0.03));
    float line = 1.0 - smoothstep(0.0, 0.05, cell.x);
    float rim = smoothstep(-unit * 0.75, 0.0, d);
    // A brighter crest sweeps around the shield.
    float crest = pow(0.5 + 0.5 * cos(angle - t * 1.4), 8.0);
    float cellPulse = 0.5 + 0.5 * sin(t * 2.0 + hash(cell.yz) * TAU);
    float inside = 1.0 - smoothstep(0.0, 2.5, d);
    float lattice = line * inside * (0.06 + rim * (0.22 + 0.6 * crest) * (0.6 + 0.4 * cellPulse));
    float fill = (1.0 - smoothstep(0.0, 0.28, cell.x)) * inside * rim * crest * 0.18;
    float edge = exp(-abs(d) / 2.2) * (0.3 + 0.55 * crest);
    add(acc, tone, lattice + fill + edge);
  }

  if ((u_flags & ${FX.ward}) != 0) {
    // A rotating runic circle just outside the card: a soft halo carrying
    // drifting glyph marks, and a thin counter-rotating inner band.
    float slot = around / 14.0 - t * 0.25;
    float cellX = fract(slot) - 0.5;
    float glyphSeed = hash(vec2(floor(slot), 2.0));
    float glyph = smoothstep(0.30, 0.18, abs(cellX)) * step(0.25, glyphSeed);
    // Each glyph is a short bar with a tick, chosen per slot.
    float tick = step(abs(cellX - (glyphSeed - 0.5) * 0.3), 0.06);
    float halo = exp(-pow((d - 6.5) / 3.2, 2.0));
    float runeBand = exp(-pow((d - 6.5) / 1.3, 2.0)) * (glyph * 0.85 + tick * exp(-pow((d - 4.2) / 0.8, 2.0)) * 0.9);
    float inner = exp(-pow((d - 2.2) / 0.6, 2.0)) * (0.35 + 0.35 * step(0.55, fract(around / 7.0 + t * 0.4)));
    float breathe = 0.78 + 0.22 * sin(t * 1.7);
    add(acc, vec3(0.40, 0.85, 0.95), (halo * 0.22 + runeBand + inner) * breathe);
  }

  if ((u_flags & ${FX.indestructible}) != 0) {
    // Burnished gold rim and a metallic glint that rakes across the card.
    float band = exp(-pow((d + 1.6) / 1.9, 2.0));
    float metal = 0.65 + 0.35 * sin(around * 0.09 + t * 0.8) * sin(around * 0.023 - t * 0.4);
    add(acc, vec3(1.0, 0.82, 0.42), band * metal * 0.75);
    float diag = (q.x - q.y) / (halfSize.x + halfSize.y);
    float sweep = mod(t / 3.4, 1.0) * 3.4 - 1.7;
    float glint = exp(-pow((diag - sweep) * 7.0, 2.0)) + 0.5 * exp(-pow((diag - sweep + 0.12) * 16.0, 2.0));
    float inside = 1.0 - smoothstep(-1.0, 0.5, d);
    add(acc, vec3(1.0, 0.93, 0.72), glint * inside * 0.32);
    // Sparks twinkle where the glint crosses the rim.
    float slot = floor(around / 9.0);
    float twinkle = pow(max(0.0, sin(t * 3.0 + hash(vec2(slot, 4.0)) * TAU)), 24.0);
    float sparkle = exp(-length(vec2(mod(around, 9.0) - 4.5, d + 1.5)) / 1.3) * twinkle;
    add(acc, vec3(1.0, 0.97, 0.85), sparkle * 1.2);
  }

  if ((u_flags & ${FX.protection}) != 0) {
    // An iridescent bubble enclosing the whole card.
    float shell = roundRect(q, halfSize + vec2(8.0), radius + unit * 0.45);
    float surface = exp(-pow(shell / 2.2, 2.0));
    float body = smoothstep(-unit * 0.55, 0.0, shell) * step(shell, 0.0);
    float shimmer = 0.6 + 0.4 * sin(angle * 3.0 + t * 1.2);
    vec3 hue = mix(u_protection, vec3(0.80, 0.92, 1.0), 0.3 + 0.3 * sin(angle * 2.0 - t * 0.9));
    // A specular arc on the upper-left of the bubble.
    float arc = smoothstep(0.55, 1.0, cos(angle + 2.3 + 0.25 * sin(t * 0.6))) * exp(-pow((shell + 3.0) / 1.6, 2.0));
    add(acc, hue, surface * shimmer * 0.55 + body * 0.14 + arc * 0.7);
  }

  if ((u_flags & ${FX.attacking}) != 0) {
    // Flames licking off the silhouette.
    float outside = max(d, 0.0);
    float flame = fbm(vec2(around * 0.09, outside * 0.13 - t * 1.9));
    float tongue = smoothstep(0.22, 0.7, flame - outside / 20.0) * step(-1.5, d);
    vec3 fire = mix(vec3(1.0, 0.25, 0.05), vec3(1.0, 0.85, 0.4), smoothstep(0.45, 0.95, flame - outside / 22.0));
    add(acc, fire, tongue * 1.0 + exp(-abs(d) / 1.6) * 0.45);
  }

  if ((u_flags & ${FX.blocking}) != 0) {
    float pulse = 0.65 + 0.35 * sin(t * 2.4);
    add(acc, vec3(0.55, 0.75, 1.0), exp(-abs(d) / 1.8) * 0.55 * pulse);
  }

  outColor = acc;
}`;

export class KeywordFxRenderer {
  constructor(canvas) {
    const gl = canvas.getContext("webgl2", { alpha: true, premultipliedAlpha: true, antialias: false, depth: false, stencil: false, preserveDrawingBuffer: false });
    if (!gl) throw new Error("WebGL2 unavailable");
    this.gl = gl;
    this.job = prepareShaderProgram(gl, vertex, fragment);
    this.program = this.job.program;
    this.ready = this.job.ready.then(ok => {
      if (!ok || this.disposed) return false;
      const names = ["u_size", "u_card", "u_time", "u_seed", "u_lift", "u_drift", "u_flags", "u_layer", "u_protection"];
      this.uniforms = Object.fromEntries(names.map(name => [name, gl.getUniformLocation(this.program, name)]));
      this.initialized = true;
      return true;
    });
    this.vao = gl.createVertexArray();
    gl.bindVertexArray(this.vao);
    this.quad = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, this.quad);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    gl.disable(gl.BLEND);
    gl.enable(gl.SCISSOR_TEST);
  }

  ensureSize(width, height) {
    const { canvas } = this.gl;
    if (canvas.width < width || canvas.height < height) {
      canvas.width = Math.max(canvas.width, width);
      canvas.height = Math.max(canvas.height, height);
    }
  }

  // tiles: [{x, y, w, h (device px, top-left origin), size, card, flags, layer, seed, lift, drift, protection}]
  render(tiles, time) {
    if (!this.initialized || this.disposed) return false;
    const { gl, uniforms: u } = this;
    const height = gl.canvas.height;
    gl.useProgram(this.program);
    gl.bindVertexArray(this.vao);
    gl.uniform1f(u.u_time, time);
    for (const tile of tiles) {
      const y = height - tile.y - tile.h;
      gl.viewport(tile.x, y, tile.w, tile.h);
      gl.scissor(tile.x, y, tile.w, tile.h);
      gl.uniform2f(u.u_size, tile.size[0], tile.size[1]);
      gl.uniform2f(u.u_card, tile.card[0], tile.card[1]);
      gl.uniform1f(u.u_seed, tile.seed);
      gl.uniform1f(u.u_lift, tile.lift);
      gl.uniform1f(u.u_drift, tile.drift);
      gl.uniform1i(u.u_flags, tile.flags);
      gl.uniform1i(u.u_layer, tile.layer);
      gl.uniform3fv(u.u_protection, tile.protection);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
    }
    return true;
  }

  dispose() {
    if (this.disposed) return;
    this.disposed = true;
    const { gl } = this;
    gl.deleteBuffer(this.quad);
    gl.deleteVertexArray(this.vao);
    this.job.dispose();
  }
}
