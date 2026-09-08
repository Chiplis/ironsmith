const plain = value => value && typeof value === 'object'
  && (Array.isArray(value) || Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null);

function changesBetween(before, after, path, changes, limit) {
  if (Object.is(before, after)) return;
  if (!plain(before) || !plain(after) || Array.isArray(before) !== Array.isArray(after)
      || (Array.isArray(after) && before.length !== after.length)) {
    changes.push({ path, value: after }); return;
  }
  for (const key of Object.keys(before)) {
    if (!Object.hasOwn(after, key)) changes.push({ path: [...path, key], remove: true });
    if (changes.length > limit) return;
  }
  for (const key of Object.keys(after)) {
    if (!Object.hasOwn(before, key)) changes.push({ path: [...path, key], value: after[key] });
    else changesBetween(before[key], after[key], [...path, key], changes, limit);
    if (changes.length > limit) return;
  }
}

function applyChange(value, change, index = 0) {
  if (index === change.path.length) return change.value;
  const key = change.path[index];
  if (!plain(value)) throw new Error('Invalid snapshot patch path');
  const copy = Array.isArray(value) ? value.slice() : { ...value };
  if (index === change.path.length - 1 && change.remove) delete copy[key];
  else Object.defineProperty(copy, key, { value: applyChange(value[key], change, index + 1),
    writable: true, enumerable: true, configurable: true });
  return copy;
}

export function createSnapshotEncoder({ limit = 128 } = {}) {
  let previous = null, revision = 0;
  return {
    encode(value, { full = false } = {}) {
      const baseRevision = revision++;
      const changes = [];
      if (!full && previous !== null) changesBetween(previous, value, [], changes, limit);
      const message = full || previous === null || changes.length > limit
        ? { revision, full: value } : { revision, baseRevision, changes };
      previous = value;
      return message;
    },
    reset() { previous = null; revision = 0; },
  };
}

export function createSnapshotDecoder() {
  let previous = null, revision = 0;
  return {
    decode(message) {
      if (!Number.isSafeInteger(message.revision) || message.revision <= revision) throw new Error('Stale snapshot revision');
      if (Object.hasOwn(message, 'full')) previous = message.full;
      else {
        if (message.baseRevision !== revision || !Array.isArray(message.changes)) throw new Error('Snapshot patch requires full recovery');
        let next = previous;
        for (const change of message.changes) next = applyChange(next, change);
        previous = next;
      }
      revision = message.revision;
      return previous;
    },
    reset() { previous = null; revision = 0; },
  };
}
