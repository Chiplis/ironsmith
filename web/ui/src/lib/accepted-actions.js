import { sha256Bytes } from './sha256.js';
export const EMPTY_ACTION_PREFIX = '0'.repeat(64);
const canonical = value => JSON.stringify(value, (_key, item) => item && typeof item === 'object' && !Array.isArray(item)
  ? Object.fromEntries(Object.keys(item).sort().map(key => [key, item[key]])) : item);
export function actionPrefixHash(previous, entry) {
  const bytes = new TextEncoder().encode(canonical([previous, Number(entry.seq), Number(entry.actorIndex), entry.command, entry.clock ?? null]));
  return Array.from(sha256Bytes(bytes), byte => byte.toString(16).padStart(2, '0')).join('');
}
export function withActionPrefixes(entries) {
  let previous = EMPTY_ACTION_PREFIX;
  return entries.map(entry => {
    const prefixHash = actionPrefixHash(previous, entry);
    if (entry.prefixHash && entry.prefixHash !== prefixHash) throw new Error('Accepted transcript prefix mismatch');
    previous = prefixHash;
    return immutableAction({ ...entry, prefixHash });
  });
}
export function immutableAction(value) {
  const entry = structuredClone(value);
  const freeze = object => {
    if (!object || typeof object !== 'object' || Object.isFrozen(object)) return;
    for (const child of Object.values(object)) freeze(child);
    Object.freeze(object);
  };
  freeze(entry);
  return entry;
}

// The owned array is append-only. Cursors retain an immutable logical prefix
// without copying it. A rollback creates a new owned array only on failure.
export const actionCursor = actions => ({ entries: actions, length: actions.length });
export const restoreActionCursor = cursor => cursor.entries.slice(0, cursor.length);
