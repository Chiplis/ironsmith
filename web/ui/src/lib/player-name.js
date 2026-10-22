// The name a player goes by in multiplayer lobbies, remembered in this
// browser so it survives reloads and later sessions.
const storageKey = "ironsmith-player-name-v1";
const maxLength = 40;

export function sanitizeStoredPlayerName(raw) {
  return String(raw ?? "").replace(/\s+/g, " ").trim().slice(0, maxLength);
}

export function readStoredPlayerName() {
  try {
    return sanitizeStoredPlayerName(globalThis.localStorage?.getItem(storageKey));
  } catch {
    return "";
  }
}

export function writeStoredPlayerName(name) {
  const value = sanitizeStoredPlayerName(name);
  if (!value) return;
  try {
    globalThis.localStorage?.setItem(storageKey, value);
  } catch {
    // Private windows can refuse storage; the name still applies this session.
  }
}
