// One private image per match/seat; bounded across matches. IndexedDB uses the
// structured clone algorithm, so JS reference data and bigint survive reload.
const key = (matchId, seat) => JSON.stringify([matchId, seat]);
let connection;
function database() {
  if (!connection) connection = new Promise((resolve, reject) => {
    const request = indexedDB.open('ironsmith-exact-snapshots-v1', 1);
    request.onupgradeneeded = () => request.result.createObjectStore('images');
    request.onerror = () => { connection = null; reject(request.error); };
    request.onsuccess = () => {
      const db = request.result;
      db.onversionchange = () => { db.close(); connection = null; };
      db.onclose = () => { connection = null; };
      resolve(db);
    };
  });
  return connection;
}
async function transaction(mode, operation) {
  const db = await database();
  return new Promise((resolve, reject) => {
    const tx = db.transaction('images', mode); let result;
    tx.oncomplete = () => resolve(result);
    tx.onerror = tx.onabort = () => reject(tx.error || new Error('Exact snapshot storage failed'));
    operation(tx.objectStore('images'), value => { result = value; });
  });
}
export const readExactSnapshot = (matchId, seat) => transaction('readonly', (store, done) => {
  const request = store.get(key(matchId, seat)); request.onsuccess = () => done(request.result);
});
export const deleteExactSnapshot = (matchId, seat) => transaction('readwrite', store => store.delete(key(matchId, seat)));
export const writeExactSnapshot = point => transaction('readwrite', store => {
  const request = store.getAll();
  request.onsuccess = () => {
    const others = request.result.filter(old => key(old.matchId, old.seat) !== key(point.matchId, point.seat))
      .sort((a, b) => b.savedAt - a.savedAt);
    for (const old of others.slice(2)) store.delete(key(old.matchId, old.seat));
    store.put({ ...point, savedAt: Date.now() }, key(point.matchId, point.seat));
  };
});
