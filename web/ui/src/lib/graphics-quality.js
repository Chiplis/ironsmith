export const GRAPHICS_LEVELS = ['off', 'low', 'high'];
export const DEFAULT_GRAPHICS = 'high';
export function normalizeGraphics(value) {
  return GRAPHICS_LEVELS.includes(value) ? value : DEFAULT_GRAPHICS;
}
// Decorative buffers have a pixel budget; card faces never use this scale.
export function effectPixelRatio(width, height, dpr = 1, quality = 'high') {
  const cap = quality === 'high' ? 2 : 1;
  const budget = quality === 'high' ? 4_000_000 : 1_000_000;
  return Math.min(Math.max(1, dpr), cap, Math.sqrt(budget / Math.max(1, width * height)));
}
