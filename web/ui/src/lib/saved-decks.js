import { PUBLIC_FORMATS } from './relay/formats.js';

export const SAVED_DECKS_CHANGED = 'ironsmith-saved-decks-changed';
const legacyKey = 'ironsmith.savedDeckPresets.v3';
let connection;
const keyFor = name => String(name || '').trim().toLowerCase();
const strings = values => Array.isArray(values) ? values.map(value => String(value || '')) : [];
function normalize(entry) {
  const name = String(entry?.name || '').trim();
  const formats = new Set(strings(entry?.formats));
  return {
    id: keyFor(name), name, texts: strings(entry?.texts),
    playerNames: strings(entry?.playerNames).map(name => name.trim()),
    formats: [...Object.keys(PUBLIC_FORMATS), 'normal', 'planechase'].filter(format => formats.has(format)),
    updatedAt: Number(entry?.updatedAt) || 0,
  };
}

function database() {
  if (!connection) connection = new Promise((resolve, reject) => {
    if (!globalThis.indexedDB) return reject(new Error('IndexedDB unavailable'));
    const request = indexedDB.open('ironsmith-saved-decks-v1', 1);
    request.onupgradeneeded = () => {
      request.result.createObjectStore('decks', { keyPath: 'id' });
      request.result.createObjectStore('migrations');
    };
    request.onerror = () => reject(request.error);
    request.onblocked = () => reject(new Error('Close other Ironsmith tabs and try again'));
    request.onsuccess = () => {
      const db = request.result;
      db.onversionchange = () => { db.close(); connection = undefined; };
      db.onclose = () => { connection = undefined; };
      resolve(db);
    };
  }).catch(error => { connection = undefined; throw error; });
  return connection;
}

// Import each legacy payload once. Markers prevent deleted decks returning
// when an older tab still has its sessionStorage copy.
async function migrate(db) {
  const payloads = [];
  for (const kind of ['localStorage', 'sessionStorage']) {
    try {
      const raw = globalThis.window?.[kind]?.getItem(legacyKey);
      const entries = JSON.parse(raw || 'null');
      if (Array.isArray(entries)) payloads.push({ raw, entries });
    } catch { /* Restricted or malformed legacy storage must not block IndexedDB. */ }
  }
  for (const {raw, entries} of payloads) {
    const candidates = new Map();
    for (const value of entries) {
      const entry = normalize(value);
      if (entry.id && (!candidates.has(entry.id) || candidates.get(entry.id).updatedAt < entry.updatedAt)) candidates.set(entry.id, entry);
    }
    await new Promise((resolve, reject) => {
      const tx = db.transaction(['decks', 'migrations'], 'readwrite');
      const decks = tx.objectStore('decks');
      const markers = tx.objectStore('migrations');
      const check = markers.get(raw);
      check.onsuccess = () => {
        if (check.result) return;
        markers.put(true, raw);
        for (const entry of candidates.values()) {
          const prior = decks.get(entry.id);
          prior.onsuccess = () => {
            if (!prior.result || prior.result.updatedAt < entry.updatedAt) decks.put(entry);
          };
        }
      };
      tx.oncomplete = resolve;
      tx.onerror = tx.onabort = () => reject(tx.error || new Error('Deck migration failed'));
    });
  }
}

async function transact(mode, operation) {
  const db = await database();
  await migrate(db);
  return new Promise((resolve, reject) => {
    const tx = db.transaction('decks', mode);
    let result;
    try { result = operation(tx.objectStore('decks')); }
    catch (error) { tx.abort(); reject(error); return; }
    tx.oncomplete = () => resolve(typeof result === 'function' ? result() : result.result);
    tx.onerror = tx.onabort = () => reject(tx.error || new Error('Deck storage failed'));
  });
}
function changed() {
  globalThis.dispatchEvent?.(new Event(SAVED_DECKS_CHANGED));
  try {
    if (typeof BroadcastChannel === 'undefined') return;
    const channel = new BroadcastChannel(SAVED_DECKS_CHANGED);
    channel.postMessage('changed');
    channel.close();
  } catch { /* Cross-tab notifications are optional; the deck is already committed. */ }
}
export async function listSavedDeckPresets() {
  const entries = await transact('readonly', store => store.getAll());
  return entries.sort((a, b) => b.updatedAt - a.updatedAt || a.name.localeCompare(b.name));
}
export async function findSavedDeckPreset(name) {
  return (await transact('readonly', store => store.get(keyFor(name)))) || null;
}
export async function saveSavedDeckPreset(name, texts, playerNames = [], formats = []) {
  const entry = normalize({name, texts, playerNames, formats, updatedAt: Date.now()});
  if (!entry.id || !entry.formats.length) return {saved: false, reason: entry.id ? 'format' : 'name'};
  try {
    const result = await transact('readwrite', store => {
      const prior = store.get(entry.id);
      store.put(entry);
      const all = store.getAll();
      return () => ({ replaced: Boolean(prior.result), entries: all.result.sort((a, b) => b.updatedAt - a.updatedAt || a.name.localeCompare(b.name)) });
    });
    changed();
    return {saved: true, ...result, entry};
  } catch (error) {
    return {saved: false, reason: 'storage', error};
  }
}
export async function removeSavedDeckPreset(name) {
  await transact('readwrite', store => store.delete(keyFor(name)));
  changed();
  return listSavedDeckPresets();
}
