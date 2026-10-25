export const AUTO_RESOLVE_STORAGE_KEY = 'ironsmith.autoResolveEnabled';

export function readAutoResolveMode(storage) {
  try {
    const saved = (storage ?? globalThis.localStorage)?.getItem(AUTO_RESOLVE_STORAGE_KEY);
    if (['off', 'safe', 'all'].includes(saved)) return saved;
    if (saved === 'false') return 'off';
    // The former phase-only setting and a fresh browser use the white clock.
    return 'safe';
  } catch {
    return 'safe';
  }
}
