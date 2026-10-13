export const CUSTOM_CARD_LIBRARY_CHANGED = "custom-card-library-changed";
let connection;
let bundled;
const normalize = value => String(value || "").trim().toLowerCase();

function database() {
  if (!connection) {
    connection = new Promise((resolve, reject) => {
      if (!globalThis.indexedDB) return reject(new Error("IndexedDB is unavailable in this browser"));
      const request = indexedDB.open("ironsmith-custom-cards-v1", 1);
      request.onupgradeneeded = () => request.result.createObjectStore("cards", { keyPath: "id" });
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

export async function saveCustomCard(draft) {
  const card = savedCardRecord(draft);
  await transaction("readwrite", store => store.put(card));
  globalThis.dispatchEvent?.(new Event(CUSTOM_CARD_LIBRARY_CHANGED));
  return card;
}

export const listSavedCustomCards = () => transaction("readonly", store => store.getAll());

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
  const needle = normalize(query);
  return cards.filter(card => card.draft.faces.some(face => normalize(face.name).includes(needle)))
    .sort((a, b) => {
      const exact = card => card.draft.faces.some(face => normalize(face.name) === needle);
      return Number(exact(b)) - Number(exact(a));
    }).slice(0, limit).map(card => ({
      name: card.draft.faces.map(face => face.name).join(" // "),
      card,
    }));
}
