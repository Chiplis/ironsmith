// One WebGL2 surface for stack light. Particle records are uploaded only on
// emission; the vertex shader evaluates drag, gravity, spin and lifetime.
const CAPACITY = 900;
const STRIDE = 20;
const vertex = `#version 300 es
precision highp float;
layout(location=0) in vec2 corner;
layout(location=1) in vec4 originVelocity;
layout(location=2) in vec4 timing; // birth, life, size, drag
layout(location=3) in vec4 dynamics; // gravity, rotation, spin, kind
layout(location=4) in vec4 color;
layout(location=5) in vec4 extras; // flicker, seed, shrink, unused
uniform vec2 resolution;
uniform float time;
out vec2 uv;
out vec4 tint;
flat out float kind;
void main() {
  float age = time - timing.x;
  float life = clamp(1.0 - age / timing.y, 0.0, 1.0);
  float k = max(age, 0.0) / 16.0;
  float drag = timing.w;
  float decay = pow(drag, k);
  float travel = drag * (1.0 - decay) / max(1.0 - drag, 0.00001);
  vec2 position = originVelocity.xy + originVelocity.zw * travel;
  position.y += dynamics.x / max(1.0 - drag, 0.00001) * (k - travel);
  float angle = dynamics.y + dynamics.z * k;
  float size = timing.z * mix(1.0, 0.35 + 0.65 * life, extras.z);
  kind = dynamics.w;
  float radius = size * (kind == 1.0 ? 2.4 : kind == 2.0 ? 1.6 : 3.0);
  vec2 offset = mat2(cos(angle), sin(angle), -sin(angle), cos(angle)) * corner * radius;
  vec2 clip = (position + offset) / resolution * 2.0 - 1.0;
  gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
  uv = corner;
  tint = color;
  tint.a *= life * mix(1.0, 0.55 + 0.45 * sin(age * 0.05 + extras.y), extras.x);
  if (age < 0.0 || age >= timing.y) {
    tint.a = 0.0;
    gl_Position = vec4(2.0, 2.0, 0.0, 1.0);
  }
}`;
const fragment = `#version 300 es
precision highp float;
in vec2 uv;
in vec4 tint;
flat in float kind;
out vec4 result;
void main() {
  float coverage;
  vec3 rgb = tint.rgb;
  if (kind == 1.0) {
    float d = min(max(abs(uv.x)-0.055, abs(uv.y)-1.0), max(abs(uv.y)-0.055, abs(uv.x)-1.0));
    coverage = 1.0 - smoothstep(-0.025, 0.025, d);
  } else if (kind == 2.0) {
    // A rotating triangular glass shard.
    float d = max(abs(uv.x + uv.y * 0.08) - (uv.y + 1.0) * 0.32, uv.y - 0.625);
    coverage = 1.0 - smoothstep(-0.02, 0.02, d);
  } else {
    float r = length(uv);
    coverage = r < 0.25 ? mix(1.0, 0.9, r / 0.25) : max(0.0, 0.9 * (1.0-r) / 0.75);
    rgb = mix(rgb, vec3(1.0), 1.0-smoothstep(0.0, 0.25, r));
  }
  float alpha = tint.a * coverage;
  result = vec4(rgb * alpha, alpha);
}`;
const shapeVertex = `#version 300 es
precision highp float;
layout(location=0) in vec2 position;
layout(location=1) in vec2 local;
layout(location=2) in vec4 color;
layout(location=3) in vec4 shape;
uniform vec2 resolution;
out vec2 uv;
out vec4 tint;
flat out vec4 spec;
void main() {
  vec2 clip = position / resolution * 2.0 - 1.0;
  gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
  uv = local; tint = color; spec = shape;
}`;
const shapeFragment = `#version 300 es
precision highp float;
in vec2 uv;
in vec4 tint;
flat in vec4 spec;
out vec4 result;
void main() {
  float coverage = 1.0;
  vec3 rgb = tint.rgb;
  if (spec.x == 1.0) {
    float r = length(uv);
    coverage = r < 0.35 ? mix(1.0, 0.8, r / 0.35) : max(0.0, 0.8 * (1.0-r) / 0.65);
    rgb = mix(rgb, vec3(1.0), 1.0-smoothstep(0.18, 0.35, r));
  } else if (spec.x == 2.0) {
    float d = abs(length(uv)-spec.y)-spec.z*0.5;
    coverage = 1.0-smoothstep(-0.75, 0.75, d);
  } else if (spec.x == 3.0) {
    vec2 q = abs(uv) - spec.yz + vec2(spec.w);
    float d = length(max(q,0.0))+min(max(q.x,q.y),0.0)-spec.w;
    coverage = 1.0-smoothstep(-0.75,0.75,abs(d)-1.25);
  }
  float alpha = tint.a*coverage;
  result = vec4(rgb*alpha,alpha);
}`;
function program(gl, vs, fs) {
  const shaders = [];
  const result = gl.createProgram();
  try {
    for (const [type, source] of [[gl.VERTEX_SHADER, vs], [gl.FRAGMENT_SHADER, fs]]) {
      const shader = gl.createShader(type);
      shaders.push(shader);
      gl.shaderSource(shader, source); gl.compileShader(shader);
      if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader));
      gl.attachShader(result, shader);
    }
    gl.linkProgram(result);
    if (!gl.getProgramParameter(result, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(result));
    return result;
  } catch (error) { gl.deleteProgram(result); throw error; }
  finally { for (const shader of shaders) gl.deleteShader(shader); }
}
const colors = new Map();
function color(rgb, alpha) {
  if (!colors.has(rgb)) colors.set(rgb, rgb.split(',').map(value => Number(value.trim()) / 255));
  return [...colors.get(rgb), Math.max(0, Math.min(1, alpha))];
}

export class StackFxRenderer {
  constructor(canvas) {
    const gl = canvas.getContext('webgl2', { alpha: true, premultipliedAlpha: true, antialias: false, depth: false, stencil: false });
    if (!gl) throw new Error('WebGL2 unavailable');
    this.gl = gl;
    this.particleProgram = program(gl, vertex, fragment);
    this.shapeProgram = program(gl, shapeVertex, shapeFragment);
    this.particleResolution = gl.getUniformLocation(this.particleProgram, 'resolution');
    this.particleTime = gl.getUniformLocation(this.particleProgram, 'time');
    this.shapeResolution = gl.getUniformLocation(this.shapeProgram, 'resolution');
    this.particleVao = gl.createVertexArray(); gl.bindVertexArray(this.particleVao);
    this.corners = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, this.corners);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1,-1,1,-1,-1,1,-1,1,1,-1,1,1]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0,2,gl.FLOAT,false,0,0);
    this.particles = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, this.particles);
    gl.bufferData(gl.ARRAY_BUFFER, CAPACITY*STRIDE*4, gl.DYNAMIC_DRAW);
    for (let i=1;i<=5;i++) {
      gl.enableVertexAttribArray(i); gl.vertexAttribPointer(i,4,gl.FLOAT,false,STRIDE*4,(i-1)*16); gl.vertexAttribDivisor(i,1);
    }
    this.shapeVao = gl.createVertexArray(); gl.bindVertexArray(this.shapeVao);
    this.shapeBuffer = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER,this.shapeBuffer);
    for (const [i,size,offset] of [[0,2,0],[1,2,8],[2,4,16],[3,4,32]]) {
      gl.enableVertexAttribArray(i); gl.vertexAttribPointer(i,size,gl.FLOAT,false,48,offset);
    }
    this.shapes=[]; this.next=0; this.count=0; this.until=0;
    this.particleData = new Float32Array(CAPACITY * STRIDE);
    this.pendingStart = 0; this.pendingCount = 0;
    this.epoch = performance.now();
    gl.enable(gl.BLEND); gl.blendFunc(gl.ONE,gl.ONE);
  }
  resize(width,height,dpr) {
    const {gl}=this; this.width=width; this.height=height;
    gl.canvas.width=Math.max(1,Math.round(width*dpr)); gl.canvas.height=Math.max(1,Math.round(height*dpr));
    gl.viewport(0,0,gl.canvas.width,gl.canvas.height);
  }
  emit(p) {
    const now=performance.now();
    // A completed burst need not keep drawing the previous high-water count.
    if (now >= this.until) {
      this.next=0; this.count=0; this.pendingCount=0;
    }
    if (!this.pendingCount) this.pendingStart=this.next;
    this.particleData.set([
      p.x,p.y,p.vx,p.vy, now-this.epoch,p.life,p.size,p.drag??0.94,
      p.gravity??0,p.rotation??0,p.spin??0,p.kind==='star'?1:p.kind==='shard'?2:0,
      ...color(p.rgb,p.alpha),p.flicker?1:0,p.seed??0,p.shrink===false?0:1,0,
    ], this.next*STRIDE);
    this.pendingCount++;
    this.next=(this.next+1)%CAPACITY; this.count=Math.min(CAPACITY,this.count+1);
    this.until=Math.max(this.until,now+p.life);
  }
  begin() { this.shapes.length=0; }
  quad(points,locals,rgb,alpha,spec=[0,0,0,0]) {
    const tint=color(rgb,alpha);
    for (const i of [0,1,2,2,1,3]) this.shapes.push(...points[i],...locals[i],...tint,...spec);
  }
  box(x,y,w,h,rgb,alpha,spec=[0,0,0,0],localScale=1) {
    this.quad([[x-w,y-h],[x+w,y-h],[x-w,y+h],[x+w,y+h]],
      [[-w/localScale,-h/localScale],[w/localScale,-h/localScale],[-w/localScale,h/localScale],[w/localScale,h/localScale]],rgb,alpha,spec);
  }
  line(a,b,width,rgb,alpha,endAlpha=alpha) {
    const length=Math.hypot(b.x-a.x,b.y-a.y)||1;
    const x=-(b.y-a.y)/length*width/2, y=(b.x-a.x)/length*width/2;
    const points=[[a.x+x,a.y+y],[b.x+x,b.y+y],[a.x-x,a.y-y],[b.x-x,b.y-y]];
    for (const i of [0,1,2,2,1,3]) this.shapes.push(...points[i],0,0,...color(rgb,i%2?endAlpha:alpha),0,0,0,0);
  }
  ring(x,y,radius,width,rgb,alpha) { const extent=radius+width+1; this.box(x,y,extent,extent,rgb,alpha,[2,radius,width,0]); }
  glow(x,y,radius,rgb,alpha=1) { this.box(x,y,radius,radius,rgb,alpha,[1,0,0,0],radius); }
  rect(x,y,w,h,rgb,alpha,outline=false) { this.box(x+w/2,y+h/2,w/2+2,h/2+2,rgb,alpha,outline?[3,w/2,h/2,10]:[0,0,0,0]); }
  render(now) {
    const {gl}=this; gl.clearColor(0,0,0,0); gl.clear(gl.COLOR_BUFFER_BIT);
    // Flush emissions together immediately before drawing. Per-spark uploads
    // repeatedly touch a buffer the preceding frame may still be reading.
    if (this.pendingCount) {
      gl.bindBuffer(gl.ARRAY_BUFFER,this.particles);
      if (this.pendingCount >= CAPACITY) {
        gl.bufferSubData(gl.ARRAY_BUFFER,0,this.particleData);
      } else {
        const end=Math.min(CAPACITY,this.pendingStart+this.pendingCount);
        gl.bufferSubData(gl.ARRAY_BUFFER,this.pendingStart*STRIDE*4,
          this.particleData.subarray(this.pendingStart*STRIDE,end*STRIDE));
        const wrapped=this.pendingStart+this.pendingCount-CAPACITY;
        if (wrapped > 0) gl.bufferSubData(gl.ARRAY_BUFFER,0,this.particleData.subarray(0,wrapped*STRIDE));
      }
      this.pendingCount=0;
    }
    if (this.shapes.length) {
      gl.useProgram(this.shapeProgram); gl.bindVertexArray(this.shapeVao); gl.bindBuffer(gl.ARRAY_BUFFER,this.shapeBuffer);
      gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(this.shapes),gl.DYNAMIC_DRAW);
      gl.uniform2f(this.shapeResolution,this.width,this.height);
      gl.drawArrays(gl.TRIANGLES,0,this.shapes.length/12);
    }
    if (now<this.until) {
      gl.useProgram(this.particleProgram); gl.bindVertexArray(this.particleVao);
      gl.uniform2f(this.particleResolution,this.width,this.height); gl.uniform1f(this.particleTime,now-this.epoch);
      gl.drawArraysInstanced(gl.TRIANGLES,0,6,this.count);
    }
  }
  dispose() {
    const {gl}=this;
    for (const buffer of [this.corners,this.particles,this.shapeBuffer]) gl.deleteBuffer(buffer);
    for (const vao of [this.particleVao,this.shapeVao]) gl.deleteVertexArray(vao);
    gl.deleteProgram(this.particleProgram); gl.deleteProgram(this.shapeProgram);
  }
}
