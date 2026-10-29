const legacyKey = 'ironsmith-community-library-v1';
let connection;
let migration;
function database() {
  if (!connection) connection = new Promise((resolve, reject) => {
    if (!globalThis.indexedDB) { reject(new Error('IndexedDB is unavailable. Community items could not be saved.')); return; }
    const request = indexedDB.open('ironsmith-community-v1', 1);
    request.onupgradeneeded = () => {
      for (const name of ['formats', 'sets']) request.result.createObjectStore(name, { keyPath: 'name' });
      request.result.createObjectStore('metadata');
    };
    request.onerror = () => reject(request.error);
    request.onblocked = () => reject(new Error('Close other Ironsmith tabs to open the Community library.'));
    request.onsuccess = () => {
      const db = request.result;
      db.onversionchange = () => { db.close(); connection = null; migration = null; };
      resolve(db);
    };
  }).catch(error => { connection = null; throw error; });
  return connection;
}
async function migrate() {
  if (!migration) migration = (async () => {
    const db = await database();
    let legacy = null;
    try { legacy = JSON.parse(globalThis.localStorage?.getItem(legacyKey) || 'null'); } catch { /* Keep unreadable legacy data untouched. */ }
    await new Promise((resolve, reject) => {
      const tx = db.transaction(['formats', 'sets', 'metadata'], 'readwrite');
      const marker = tx.objectStore('metadata').get('localStorageMigrated');
      marker.onsuccess = () => {
        if (marker.result) return;
        for (const kind of ['formats', 'sets']) {
          for (const entry of Array.isArray(legacy?.[kind]) ? legacy[kind] : []) {
            if (!entry || typeof entry.name !== 'string' || !entry.name.trim()) continue;
            const store = tx.objectStore(kind);
            const existing = store.get(entry.name);
            existing.onsuccess = () => { if (!existing.result) store.put(entry); };
          }
        }
        tx.objectStore('metadata').put(true, 'localStorageMigrated');
      };
      tx.oncomplete = resolve;
      tx.onerror = tx.onabort = () => reject(tx.error || new Error('Could not migrate the Community library.'));
    });
    // Retain the original as a backup; the durable marker prevents reimport.
  })().catch(error => { migration = null; throw error; });
  return migration;
}
export async function loadCommunityLibrary() {
  await migrate();
  const db = await database();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(['formats', 'sets'], 'readonly');
    const formats = tx.objectStore('formats').getAll(), sets = tx.objectStore('sets').getAll();
    tx.oncomplete = () => resolve({ formats: formats.result, sets: sets.result });
    tx.onerror = tx.onabort = () => reject(tx.error || new Error('Could not load the Community library.'));
  });
}
export async function saveCommunityItem(kind, entry, { keepExisting = false } = {}) {
  if (!['set', 'format'].includes(kind)) throw new Error('Unknown Community item type.');
  await migrate();
  const db = await database();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(kind === 'set' ? 'sets' : 'formats', 'readwrite');
    const store = tx.objectStore(kind === 'set' ? 'sets' : 'formats');
    const request = store.getAllKeys();
    let saved;
    request.onsuccess = () => {
      const taken = new Set(request.result.map(name => name.toLowerCase()));
      let name = entry.name;
      if (keepExisting) {
        let suffix = 2;
        while (taken.has(name.toLowerCase())) name = `${entry.name.slice(0, 60)} (imported ${suffix++})`;
      }
      saved = { ...entry, name };
      store.put(saved);
    };
    tx.oncomplete = () => resolve(saved);
    tx.onerror = tx.onabort = () => reject(tx.error || new Error('Could not save the Community item. Browser storage may be full.'));
  });
}
