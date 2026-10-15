import { useSyncExternalStore } from "react";

const KEY = "ironsmith.showShortcutsOnStart";
export const OPEN_SHORTCUTS_EVENT = "ironsmith:open-keyboard-shortcuts";

const listeners = new Set();
let snapshot;

function read() {
  if (snapshot !== undefined) return snapshot;
  try { snapshot = localStorage.getItem(KEY) !== "false"; }
  catch { snapshot = true; }
  return snapshot;
}

function subscribe(listener) {
  listeners.add(listener);
  const sync = event => { if (event.key === KEY || event.key === null) { snapshot = undefined; listener(); } };
  window.addEventListener("storage", sync);
  return () => { listeners.delete(listener); window.removeEventListener("storage", sync); };
}

export function setShowShortcutsOnStart(value) {
  snapshot = Boolean(value);
  try { localStorage.setItem(KEY, String(snapshot)); } catch { /* Preference lasts for this session. */ }
  listeners.forEach(listener => listener());
}

export function useShowShortcutsOnStart() {
  return [useSyncExternalStore(subscribe, read, () => true), setShowShortcutsOnStart];
}

export function openKeyboardShortcuts() {
  window.dispatchEvent(new Event(OPEN_SHORTCUTS_EVENT));
}
