import { sampleCardFramePixels } from './card-frame-colors.js';
import { inpaintCardFrameGpu, cardFrameGpuStats, warmCardFrameGpu } from './card-frame-gpu.js';

// Compile the shared pipelines while fonts and image geometry are loading.
warmCardFrameGpu();

const loadedFonts = new Map();
self.onmessage = async ({data: {id, task, fonts}}) => {
  try {
    await Promise.all(fonts.map(descriptor => {
      const key = JSON.stringify(descriptor);
      if (!loadedFonts.has(key)) loadedFonts.set(key, (async () => {
        const face = new FontFace(descriptor.family, descriptor.src, {weight: descriptor.weight, style: descriptor.style});
        await face.load(); self.fonts.add(face);
      })());
      return loadedFonts.get(key);
    }));
    const before = cardFrameGpuStats().gpuJobs;
    const result = await sampleCardFramePixels(task, inpaintCardFrameGpu);
    result['--frame-processing-backend'] = cardFrameGpuStats().gpuJobs > before ? 'webgpu' : 'worker-cpu';
    self.postMessage({id, result});
  } catch (error) {
    self.postMessage({id, error: String(error?.message || error)});
  }
};
