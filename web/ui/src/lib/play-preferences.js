export function playPreferenceStorage() {
  try { return typeof window === 'undefined' ? null : window.localStorage; }
  catch { return null; }
}

export function readPlayPreference(key, storage = playPreferenceStorage()) {
  try { return JSON.parse(storage?.getItem(key) ?? 'null'); }
  catch { return null; }
}

export function writePlayPreference(key, value, storage = playPreferenceStorage()) {
  try { storage?.setItem(key, JSON.stringify(value)); }
  catch { /* Keep controls usable when browser storage is unavailable. */ }
}

const GOLDFISH_SEATS_KEY = 'ironsmith.goldfishSeats';
export function readGoldfishSeats(storage = playPreferenceStorage()) {
  const saved = readPlayPreference(GOLDFISH_SEATS_KEY, storage);
  return new Set(Array.isArray(saved) && saved.every(seat => Number.isInteger(seat) && seat >= 0)
    ? saved : [1]);
}

export function storeGoldfishSeats(seats, storage = playPreferenceStorage()) {
  writePlayPreference(GOLDFISH_SEATS_KEY, [...seats], storage);
}
