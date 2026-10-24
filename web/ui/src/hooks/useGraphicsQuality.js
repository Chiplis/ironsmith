import { useSyncExternalStore } from 'react';
import { DEFAULT_GRAPHICS, normalizeGraphics } from '../lib/graphics-quality.js';
const KEY = 'ironsmith.graphicsQuality';
const listeners = new Set();
let current;
function read() {
  if (current !== undefined) return current;
  try { current = normalizeGraphics(localStorage.getItem(KEY)); }
  catch { current = DEFAULT_GRAPHICS; }
  return current;
}
function subscribe(listener) {
  listeners.add(listener);
  const sync = event => {
    if (event.key === KEY || event.key === null) { current = undefined; listener(); }
  };
  window.addEventListener('storage', sync);
  return () => { listeners.delete(listener); window.removeEventListener('storage', sync); };
}
export function setGraphicsQuality(value) {
  current = normalizeGraphics(value);
  try { localStorage.setItem(KEY, current); } catch { /* Keep the session setting. */ }
  listeners.forEach(listener => listener());
}
export default function useGraphicsQuality() {
  return [useSyncExternalStore(subscribe, read, () => DEFAULT_GRAPHICS), setGraphicsQuality];
}
