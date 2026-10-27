import { prepareCustomCard, mergeMechanics, installCustomMechanics } from "./custom-mechanics.js";
import { cardSearchKey } from "./card-search.js";
export const CUSTOM_CARD_LIBRARY_CHANGED = "custom-card-library-changed";
let connection;
let bundled;
const normalize = value => String(value || "").trim().toLowerCase();

function database() {
  if (!connection) {
    connection = new Promise((resolve, reject) => {
      if (!globalThis.indexedDB) return reject(new Error("IndexedDB is unavailable in this browser"));
      const request = indexedDB.open("ironsmith-custom-cards-v1", 2);
      request.onupgradeneeded = () => {
        if (!request.result.objectStoreNames.contains("cards")) request.result.createObjectStore("cards", { keyPath: "id" });
        if (!request.result.objectStoreNames.contains("mechanics")) request.result.createObjectStore("mechanics", { keyPath: "keyword" });
      };
      request.onerror = () => reject(request.error);
      request.onblocked = () => reject(new Error("Close other Ironsmith tabs to open your card library"));
      request.onsuccess = () => {
        const db = request.result;
        db.onversionchange = () => { db.close(); connection = null; };
        db.onclose = () => { connection = null; };
        resolve(db);
      };
    }).catch(error => { connection = null; throw error; });
  }
  return connection;
}

async function transaction(mode, operation) {
  const db = await database();
  return new Promise((resolve, reject) => {
    const tx = db.transaction("cards", mode);
    const request = operation(tx.objectStore("cards"));
    tx.oncomplete = () => resolve(request.result);
    tx.onerror = tx.onabort = () => reject(tx.error || new Error("Could not access saved custom cards"));
  });
}

export function savedCardRecord(draft) {
  const name = draft?.faces?.[0]?.name?.trim();
  if (!name) throw new Error("Give your card a name before saving");
  // Saving an edited version of the same named card replaces its prior version.
  return { id: `saved:${normalize(name)}`, draft: structuredClone(draft), savedAt: Date.now() };
}

export async function listCustomMechanics() {
  const db = await database();
  return new Promise((resolve, reject) => {
    const tx = db.transaction("mechanics", "readonly");
    const request = tx.objectStore("mechanics").getAll();
    tx.oncomplete = () => { installCustomMechanics(request.result); resolve(request.result); };
    tx.onerror = tx.onabort = () => reject(tx.error || new Error("Could not load custom mechanics"));
  });
}

export async function reserveCustomMechanics(definitions) {
  if (!definitions.length) return;
  const db = await database();
  // One read/write transaction serializes competing tabs. No partial reservation
  // is committed when any of the definitions conflict.
  const stored = await new Promise((resolve, reject) => {
    const tx = db.transaction("mechanics", "readwrite"), store = tx.objectStore("mechanics");
    let merged, failure;
    const request = store.getAll();
    request.onsuccess = () => {
      try {
        merged = mergeMechanics(request.result, definitions);
        for (const definition of merged) store.put(definition);
      } catch (error) { failure = error; tx.abort(); }
    };
    tx.oncomplete = () => resolve(merged);
    tx.onerror = tx.onabort = () => reject(failure || tx.error || new Error("Could not reserve custom mechanics"));
  });
  installCustomMechanics(stored);
}

export async function previewCustomDraft(draft, compile) {
  const prepared = await prepareCustomCard(draft, compile, { known: await listCustomMechanics().catch(() => []) });
  installCustomMechanics(prepared.draft.customMechanics);
  prepared.preview.customMechanics = prepared.draft.customMechanics;
  for (const face of prepared.preview.faces || []) face.customMechanics = prepared.draft.customMechanics;
  return prepared;
}

export async function prepareCustomDraftForCreation(draft, compile) {
  const prepared = await prepareCustomCard(draft, compile, {
    known: await listCustomMechanics().catch(() => []), reserve: reserveCustomMechanics,
  });
  installCustomMechanics(prepared.draft.customMechanics);
  prepared.preview.customMechanics = prepared.draft.customMechanics;
  for (const face of prepared.preview.faces || []) face.customMechanics = prepared.draft.customMechanics;
  return prepared;
}

export async function saveCustomCard(draft, { compile } = {}) {
  if (typeof compile !== "function") throw new Error("Saving a custom card requires compiler validation.");
  savedCardRecord(draft); // Validate the name before reserving any definitions.
  await database(); // A save must never succeed without durable storage.
  const prepared = await prepareCustomDraftForCreation(draft, compile);
  const card = savedCardRecord(prepared.draft);
  await transaction("readwrite", store => store.put(card));
  globalThis.dispatchEvent?.(new Event(CUSTOM_CARD_LIBRARY_CHANGED));
  return card;
}

export const listSavedCustomCards = async () => {
  await listCustomMechanics();
  return transaction("readonly", store => store.getAll());
};

export async function loadBundledCustomCards() {
  if (!bundled) {
    bundled = fetch(`${import.meta.env?.BASE_URL || "/"}custommagic/pool.json`)
      .then(response => {
        if (!response.ok) throw new Error("Could not load bundled custom cards");
        return response.json();
      }).then(pool => pool.cards).catch(error => { bundled = null; throw error; });
  }
  return bundled;
}

export function customCardOptions(cards, query, limit = 5) {
  const needle = cardSearchKey(query);
  return cards.filter(card => card.draft.faces.some(face => cardSearchKey(face.name).includes(needle)))
    .sort((a, b) => {
      const exact = card => card.draft.faces.some(face => cardSearchKey(face.name) === needle);
      return Number(exact(b)) - Number(exact(a));
    }).slice(0, limit).map(card => ({
      name: card.draft.faces.map(face => face.name).join(" // "),
      card,
    }));
}
