import { useSyncExternalStore } from 'react';

const KEY = 'ironsmith.battlefieldAppearance';
const defaults = Object.freeze({ compactCards: true, compactLands: true });
let snapshot;
const listeners = new Set();
function read() {
  if (snapshot) return snapshot;
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) || '{}');
    snapshot = Object.freeze(Object.fromEntries(Object.entries(defaults).map(([key, value]) => [key, typeof saved[key] === 'boolean' ? saved[key] : value])));
  } catch { snapshot = defaults; }
  return snapshot;
}
function subscribe(listener) {
  listeners.add(listener);
  const sync = event => { if (event.key === KEY || event.key === null) { snapshot = undefined; listener(); } };
  window.addEventListener('storage', sync);
  return () => { listeners.delete(listener); window.removeEventListener('storage', sync); };
}
function update(key, value) {
  if (!(key in defaults)) return;
  snapshot = Object.freeze({ ...read(), [key]: Boolean(value) });
  try { localStorage.setItem(KEY, JSON.stringify(snapshot)); } catch { /* Settings remain usable with storage disabled. */ }
  listeners.forEach(listener => listener());
}
export default function useBattlefieldAppearance() {
  return [useSyncExternalStore(subscribe, read, () => defaults), update];
}
