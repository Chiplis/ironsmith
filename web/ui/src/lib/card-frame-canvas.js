export function frameCanvas(width, height) {
  const canvas = typeof document === 'undefined' ? new OffscreenCanvas(width, height) : document.createElement('canvas');
  canvas.width = width; canvas.height = height;
  return canvas;
}

export async function frameCanvasUrl(canvas) {
  if (typeof canvas.toDataURL === 'function') return canvas.toDataURL();
  const blob = await canvas.convertToBlob({type: 'image/png'});
  // Worker-only synchronous conversion, after asynchronous GPU/canvas work.
  return new globalThis.FileReaderSync().readAsDataURL(blob);
}
