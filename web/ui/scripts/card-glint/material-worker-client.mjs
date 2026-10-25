import { buildMaterial } from '@ironsmith/card-glint-material';

let worker, unavailable = false, sequence = 0, idleTimer;
const pending = new Map();

function stop(error) {
  clearTimeout(idleTimer);
  worker?.terminate();
  worker = undefined;
  for (const job of pending.values()) { clearTimeout(job.timer); job.reject(error); }
  pending.clear();
}

function materialWorker() {
  if (unavailable || typeof Worker === 'undefined') return;
  if (!worker) {
    try {
      worker = new Worker(new URL('./material-worker.mjs', import.meta.url), { type: 'module' });
      worker.onmessage = ({ data }) => {
        const job = pending.get(data.id);
        if (!job) { data.bitmap?.close(); return; }
        pending.delete(data.id); clearTimeout(job.timer);
        if (data.error) job.reject(new Error(data.error));
        else job.resolve(data.bitmap);
        if (!pending.size) idleTimer = setTimeout(() => stop(new Error('Material worker idle')), 30000);
      };
      worker.onerror = event => {
        event.preventDefault(); unavailable = true;
        stop(new Error('Material worker unavailable'));
      };
      worker.onmessageerror = () => { unavailable = true; stop(new Error('Invalid material worker response')); };
    } catch { unavailable = true; }
  }
  clearTimeout(idleTimer);
  return worker;
}

// Transfer clones, never the AssetStore's bitmaps: other faces and cached
// material leases continue to own those originals, including on worker failure.
export async function buildMaterialOffThread(source, adjustment, images, tone) {
  const target = materialWorker();
  if (!target) return buildMaterial(source, adjustment, images, tone);
  const copies = [];
  try {
    const urls = [...new Set([
      adjustment.blackPoint?.textureMaskSource, adjustment.blend?.maskSource,
      adjustment.blend?.outerMaskSource, adjustment.outerStockMaskSource,
    ].filter(Boolean))];
    const bitmap = await createImageBitmap(source); copies.push(bitmap);
    const masks = new Map();
    for (const url of urls) {
      const image = images.get(url);
      if (!image) throw new Error(`Frame material mask unavailable: ${url}`);
      const copy = await createImageBitmap(image); copies.push(copy); masks.set(url, copy);
    }
    const id = ++sequence;
    return await new Promise((resolve, reject) => {
      const timer = setTimeout(() => { unavailable = true; stop(new Error('Material worker timed out')); }, 30000);
      pending.set(id, { resolve, reject, timer });
      try { target.postMessage({ id, source: bitmap, adjustment, images: masks, tone }, copies); }
      catch (error) { pending.delete(id); clearTimeout(timer); reject(error); }
    });
  } catch {
    // The exact upstream implementation also supports browsers that block or
    // lack module workers. Its input ownership and output pixels are unchanged.
    return buildMaterial(source, adjustment, images, tone);
  } finally {
    for (const copy of copies) copy.close();
  }
}
