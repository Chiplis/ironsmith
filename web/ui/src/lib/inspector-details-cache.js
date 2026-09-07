const games = new WeakMap();

function entriesFor(game, state, create = false) {
  if (!game || !state) return null;
  let snapshots = games.get(game);
  if (!snapshots && create) games.set(game, snapshots = new WeakMap());
  let entries = snapshots?.get(state);
  if (!entries && create) snapshots.set(state, entries = new Map());
  return entries;
}

export function cachedInspectorDetails(game, state, objectId) {
  return entriesFor(game, state)?.get(String(objectId));
}

// Share pending requests and completed details only within one engine snapshot.
// A different game/state cannot reuse counters, characteristics, or rules.
export function requestInspectorDetails(game, state, objectId) {
  const entries = entriesFor(game, state, true);
  const key = String(objectId);
  if (entries?.has(key)) return entries.get(key).promise;
  const entry = { ready: false, value: null, promise: null };
  entry.promise = Promise.resolve()
    .then(() => game.objectDetails(BigInt(objectId)))
    .then(value => {
      entry.value = value || null;
      entry.ready = true;
      return entry.value;
    })
    .catch(error => {
      if (entries?.get(key) === entry) entries.delete(key);
      throw error;
    });
  entries?.set(key, entry);
  if (entries?.size > 64) entries.delete(entries.keys().next().value);
  return entry.promise;
}
