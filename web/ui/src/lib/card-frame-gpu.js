import shader from './card-frame-inpaint.wgsl?raw';
import { inpaintGlyphMask, paperField } from './card-frame-font-mask.js';

let enginePromise;
let disabled = false;
const metrics = {backend: 'cpu', gpuJobs: 0, cpuJobs: 0, gpuMs: 0, reason: ''};
export const cardFrameGpuStats = () => ({...metrics});

export function warmCardFrameGpu() {
  if (disabled) return;
  enginePromise ||= createEngine();
  enginePromise.catch(error => {
    disabled = true; metrics.backend = 'cpu'; metrics.reason = String(error?.message || error);
  });
}

async function createEngine() {
  const adapter = await globalThis.navigator?.gpu?.requestAdapter({powerPreference: 'low-power'});
  if (!adapter || adapter.info?.isFallbackAdapter) throw new Error('Hardware WebGPU adapter unavailable');
  const device = await adapter.requestDevice();
  device.lost.then(() => {disabled = true; metrics.reason = 'device-lost'; metrics.backend = 'cpu';});
  const module = device.createShaderModule({code: shader});
  const compilation = await module.getCompilationInfo();
  const errors = compilation.messages.filter(message => message.type === 'error');
  if (errors.length) {device.destroy(); throw new Error(errors.map(error => `${error.lineNum}: ${error.message}`).join('\n'));}
  const layout = device.createBindGroupLayout({entries: [
    {binding: 0, visibility: GPUShaderStage.COMPUTE, buffer: {type: 'read-only-storage'}},
    {binding: 1, visibility: GPUShaderStage.COMPUTE, buffer: {type: 'storage'}},
    {binding: 2, visibility: GPUShaderStage.COMPUTE, buffer: {type: 'read-only-storage'}},
    {binding: 3, visibility: GPUShaderStage.COMPUTE, buffer: {type: 'read-only-storage'}},
    {binding: 4, visibility: GPUShaderStage.COMPUTE, buffer: {type: 'uniform'}},
    {binding: 5, visibility: GPUShaderStage.COMPUTE, buffer: {type: 'read-only-storage'}},
  ]});
  const pipelineLayout = device.createPipelineLayout({bindGroupLayouts: [layout]});
  const pipelines = await Promise.all(['propagate', 'relax', 'grain'].map(entryPoint => device.createComputePipelineAsync({layout: pipelineLayout, compute: {module, entryPoint}})));
  metrics.backend = 'webgpu';
  return {device, layout, pipelines};
}

const donorOffsets = new Int32Array(8 * 48 * 2);
for (let seed = 0; seed < 8; seed++) for (let radius = 4; radius <= 14; radius += 2) for (let i = 0; i < 8; i++) {
  const at = (seed * 48 + (radius - 4) / 2 * 8 + i) * 2, angle = (i + seed) * Math.PI / 4;
  donorOffsets[at] = Math.round(Math.cos(angle) * radius); donorOffsets[at + 1] = Math.round(Math.sin(angle) * radius);
}

async function compute(scan, mask) {
  enginePromise ||= createEngine();
  const {device, layout, pipelines} = await enginePromise;
  const count = scan.width * scan.height;
  if (count * 8 > device.limits.maxStorageBufferBindingSize) throw new Error('Frame exceeds GPU buffer limit');
  const started = performance.now(), allocated = [];
  const buffer = (size, usage, data) => {
    const result = device.createBuffer({size, usage}); allocated.push(result);
    if (data) device.queue.writeBuffer(result, 0, data);
    return result;
  };
  device.pushErrorScope('validation');
  let scopeOpen = true;
  try {
    const pixels = new Uint32Array(scan.data.buffer, scan.data.byteOffset, count);
    const initial = new Uint32Array(count * 2), masks = Uint32Array.from(mask);
    const paperAt = paperField(scan);
    for (let p = 0; p < count; p++) {
      initial[p * 2] = pixels[p];
      const at = p * 4, light = (scan.data[at] + scan.data[at + 1] + scan.data[at + 2]) / 3;
      initial[p * 2 + 1] = !mask[p] && Math.abs(light - paperAt(p % scan.width, Math.floor(p / scan.width))) < 55 ? 1 : 0;
    }
    const rw = GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST | GPUBufferUsage.COPY_SRC;
    const a = buffer(count * 8, rw, initial), b = buffer(count * 8, rw);
    const original = buffer(count * 4, rw, pixels), maskBuffer = buffer(count * 4, rw, masks);
    const dimensions = buffer(16, GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST, new Uint32Array([scan.width, scan.height, count, 0]));
    const offsets = buffer(donorOffsets.byteLength, rw, donorOffsets);
    const readback = buffer(count * 8, GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ);
    const groups = [[a, b], [b, a]].map(pair => device.createBindGroup({layout, entries: [...pair, original, maskBuffer, dimensions, offsets].map((resource, binding) => ({binding, resource: {buffer: resource}}))}));
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginComputePass();
    let current = 0;
    // Each dispatch reads a complete prior pass. No CPU readback between
    // propagation, Jacobi relaxation, and texture-grain restoration.
    for (const [pipeline, iterations] of [[pipelines[0], 40], [pipelines[1], 24], [pipelines[2], 1]]) {
      pass.setPipeline(pipeline);
      for (let i = 0; i < iterations; i++) {pass.setBindGroup(0, groups[current]); pass.dispatchWorkgroups(Math.ceil(count / 128)); current = 1 - current;}
    }
    pass.end();
    encoder.copyBufferToBuffer(current ? b : a, 0, readback, 0, count * 8);
    device.queue.submit([encoder.finish()]);
    const error = await device.popErrorScope(); scopeOpen = false;
    if (error) throw new Error(error.message);
    await readback.mapAsync(GPUMapMode.READ);
    const mapped = new Uint32Array(readback.getMappedRange()), data = new Uint8ClampedArray(scan.data.length), packed = new Uint32Array(data.buffer);
    for (let p = 0; p < count; p++) packed[p] = mapped[p * 2];
    readback.unmap(); metrics.gpuJobs++; metrics.gpuMs += performance.now() - started;
    return {data, width: scan.width, height: scan.height, mask};
  } finally {
    if (scopeOpen) await device.popErrorScope();
    for (const resource of allocated) resource.destroy();
  }
}

export async function inpaintCardFrameGpu(scan, mask) {
  // Small badges cost less to fill locally than to upload and dispatch.
  if (!disabled && scan.width * scan.height >= 2048 && mask.some(Boolean)) {
    try {return await compute(scan, mask);} catch (error) {
      disabled = true; metrics.backend = 'cpu'; metrics.reason = String(error?.message || error);
    }
  }
  metrics.cpuJobs++;
  return inpaintGlyphMask(scan, mask);
}
