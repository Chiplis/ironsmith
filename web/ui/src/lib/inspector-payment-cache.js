// Weak keys preserve results across inspector mounts without retaining old games
// or snapshots. In-flight requests are shared as well as completed results.
const games = new WeakMap();

function entriesFor(game, state, create = false) {
  let snapshots = games.get(game);
  if (!snapshots && create) games.set(game, snapshots = new WeakMap());
  let entries = snapshots?.get(state);
  if (!entries && create) snapshots.set(state, entries = new Map());
  return entries;
}

export function inspectorPaymentKey(action) {
  return `${action.object_id}:${action.ability_index}`;
}

export function cachedInspectorPayment(game, state, key) {
  return entriesFor(game, state)?.get(key);
}

export function requestInspectorPayment(game, state, key) {
  const entries = entriesFor(game, state, true);
  if (entries.has(key)) return entries.get(key);
  const [source, ability] = key.split(":");
  const entry = { ready: false, available: undefined, promise: null };
  entries.set(key, entry);
  entry.promise = Promise.resolve()
    .then(() => game.inspectorActions(BigInt(source), Number(ability)))
    .then(actions => {
      entry.available = actions.find(action => inspectorPaymentKey(action) === key)?.mana_payment_available;
      entry.ready = true;
    })
    .catch(() => {
      // An unavailable preview must not permanently disable a legal action.
      entry.ready = true;
    });
  return entry;
}
