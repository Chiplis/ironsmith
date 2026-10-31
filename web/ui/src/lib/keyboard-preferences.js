export const SHORTCUT_PRESET_KEY = 'ironsmith.priorityShortcutPreset';
export function readShortcutPreset() {
  try { return globalThis.localStorage?.getItem(SHORTCUT_PRESET_KEY) || 'mtgo'; } catch { return 'mtgo'; }
}
const PRESET_EVENT = 'ironsmith:keyboard-preset-changed';
export function setShortcutPreset(preset) {
  try { globalThis.localStorage?.setItem(SHORTCUT_PRESET_KEY, preset); } catch { /* Storage unavailable. */ }
  globalThis.window?.dispatchEvent(new Event(PRESET_EVENT));
}
export function subscribeShortcutPreset(listener) {
  const view = globalThis.window;
  if (!view) return () => {};
  view.addEventListener(PRESET_EVENT, listener);
  view.addEventListener('storage', listener);
  return () => {
    view.removeEventListener(PRESET_EVENT, listener);
    view.removeEventListener('storage', listener);
  };
}
