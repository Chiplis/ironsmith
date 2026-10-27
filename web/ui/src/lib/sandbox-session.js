import { SANDBOX_RULES } from './sandbox.js';

const STORAGE_KEY = 'ironsmith-sandbox-options-v1';
let fallback;

function normalize(value) {
  return {
    options: Object.fromEntries(SANDBOX_RULES.map(({ id }) => [id, value?.options?.[id] === true])),
    bothPlayers: typeof value?.bothPlayers === 'boolean' ? value.bothPlayers : true,
  };
}

export function readSandboxPreferences() {
  if (fallback) return normalize(fallback);
  try {
    return normalize(JSON.parse(window.sessionStorage.getItem(STORAGE_KEY)));
  } catch {
    return normalize(null);
  }
}

export function saveSandboxPreferences(preferences) {
  const normalized = normalize(preferences);
  // Keep the current choices across panel remounts even if storage is full or blocked.
  fallback = normalized;
  try {
    window.sessionStorage.setItem(STORAGE_KEY, JSON.stringify(normalized));
    fallback = undefined;
  } catch {
    // Session persistence is optional; it must never prevent starting a sandbox.
  }
}
