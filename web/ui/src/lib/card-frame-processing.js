import { sampleCardFrameColors as sampleColors, sampleCardFramePixels } from './card-frame-colors.js';

let worker;
let disabled = false;
let nextId = 0;
let active;
let queue = Promise.resolve();
let fontDescriptors;

function fontsFor(typography) {
  if (!fontDescriptors) {
    fontDescriptors = [];
    const visit = (rules, base) => {
      for (const rule of rules) {
        if (rule.type === 5) {
          const family = rule.style.getPropertyValue('font-family').replace(/["']/g, '').trim();
          const src = rule.style.getPropertyValue('src').replace(/url\(["']?([^"')]+)["']?\)/g, (_, url) => `url("${new URL(url, base).href}")`);
          fontDescriptors.push({family, src, weight: rule.style.getPropertyValue('font-weight') || 'normal', style: rule.style.getPropertyValue('font-style') || 'normal'});
        } else if (rule.cssRules) visit(rule.cssRules, base);
      }
    };
    for (const sheet of document.styleSheets) {
      try {visit(sheet.cssRules, sheet.href || document.baseURI);} catch { /* Unrelated cross-origin stylesheets. */ }
    }
  }
  const families = new Set(['title', 'type', 'rules', 'stats'].flatMap(key => String(typography?.[key] || '').split(',').map(name => name.replace(/["']/g, '').trim())));
  return fontDescriptors.filter(font => families.has(font.family));
}

function stopWorker(error = new Error('Frame worker unavailable')) {
  disabled = true; worker?.terminate(); worker = null;
  if (active) {clearTimeout(active.timer); active.reject(error); active = null;}
}

function request(task) {
  if (!worker) {
    worker = new Worker(new URL('./card-frame-processing.worker.js', import.meta.url), {type: 'module', name: 'card-frame-processing'});
    worker.onerror = () => stopWorker();
    worker.onmessageerror = () => stopWorker();
    worker.onmessage = ({data}) => {
      if (data.id !== active?.id) return;
      const job = active; active = null; clearTimeout(job.timer);
      if (data.error) job.reject(new Error(data.error)); else job.resolve(data.result);
    };
  }
  const fonts = fontsFor(task.typography);
  return new Promise((resolve, reject) => {
    const id = ++nextId;
    active = {id, resolve, reject, timer: setTimeout(() => stopWorker(new Error('Frame processing timed out')), 30000)};
    // Retain originals for recovery; transfer only the scan copies for the
    // one active job instead of cloning every queued card at once.
    const transfers = [];
    const scan = value => {
      if (!value) return null;
      const data = value.data.slice(); transfers.push(data.buffer);
      return {...value, width: value.width, height: value.height, data};
    };
    try {
      worker.postMessage({id, task: {...task, fullScan: scan(task.fullScan), artScan: scan(task.artScan), symbolScan: scan(task.symbolScan), icons: task.icons.map(scan)}, fonts}, transfers);
    } catch (error) {clearTimeout(active.timer); active = null; reject(error);}
  });
}

async function execute(task) {
  if (!disabled && typeof Worker !== 'undefined' && typeof OffscreenCanvas !== 'undefined') {
    try {return await request(task);} catch {stopWorker();}
  }
  await new Promise(resolve => setTimeout(resolve, 0));
  const result = await sampleCardFramePixels(task);
  result['--frame-processing-backend'] = 'main-cpu';
  return result;
}

export function sampleCardFrameColors(url, options) {
  return sampleColors(url, {...options, execute: task => {
    // One shared worker/adapter for the whole table; never one per card.
    const result = queue.then(() => execute(task));
    queue = result.catch(() => {});
    return result;
  }});
}
