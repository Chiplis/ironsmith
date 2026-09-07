// Weak keys preserve results across inspector mounts without retaining old games
// or snapshots. In-flight requests are shared as well as completed results.
const games = new WeakMap();
const completed = new WeakMap();
let requestSequence = 0;

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

// Read during render, before the effect has requested the new snapshot. This
// prevents even a single frame from replacing the last result with "pending".
export function inspectorPaymentDisplay(game, state, key) {
  const current = cachedInspectorPayment(game, state, key);
  const result = current?.ready ? current : completed.get(game)?.get(key);
  return {
    payment_pending: !result,
    mana_payment_available: result?.available,
  };
}

export function requestInspectorPayment(game, state, key) {
  const entries = entriesFor(game, state, true);
  if (entries.has(key)) return entries.get(key);
  const sequence = ++requestSequence;
  const [source, ability] = key.split(":");
  const entry = { ready: false, available: undefined, promise: null };
  entries.set(key, entry);
  entry.promise = Promise.resolve()
    .then(() => game.inspectorActions(BigInt(source), Number(ability)))
    .then(actions => {
      const action = actions.find(action => inspectorPaymentKey(action) === key);
      // Cancellation and stale jobs return no action. They are not completed
      // affordability checks: keep the old snapshot pending until it is replaced.
      if (!action) return;
      entry.available = action.mana_payment_available;
      entry.ready = true;
      let previous = completed.get(game);
      if (!previous) completed.set(game, previous = new Map());
      if ((previous.get(key)?.sequence ?? -1) < sequence) {
        previous.set(key, { available: entry.available, sequence });
      }
    })
    .catch(() => {
      // A failed request is not evidence that the ability is available either.
      // A new snapshot gets a fresh entry and retries the preview.
    });
  return entry;
}
