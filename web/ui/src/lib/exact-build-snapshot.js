// Private, exact-build instance images. Never accept these from another peer.
export const EXACT_SNAPSHOT_VERSION = 1;
const PAGE = 65536;
const hex = bytes => Array.from(bytes, value => value.toString(16).padStart(2, '0')).join('');
export async function snapshotDigest(bytes, crypto = globalThis.crypto) {
  if (!crypto?.subtle) throw new Error('Exact snapshots require WebCrypto');
  return hex(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)));
}
// Preserve aliases/cycles, undefined, bigint, Map and Set in the integrity hash.
// These are data objects, not executable JavaScript or host resources.
export function snapshotDataBytes(value) {
  const seen = new Map();
  function encode(value, depth = 0) {
    if (depth > 256 || seen.size > 1_000_000) throw new Error('Snapshot data exceeds limits');
    if (value === null) return ['null'];
    const type = typeof value;
    if (type === 'undefined') return ['undefined'];
    if (type === 'bigint') return ['bigint', String(value)];
    if (type === 'number') return ['number', Object.is(value, -0) ? '-0' : String(value)];
    if (type === 'string' || type === 'boolean') return [type, value];
    if (type !== 'object') throw new Error('Snapshot retains an unsupported JavaScript reference');
    if (seen.has(value)) return ['ref', seen.get(value)];
    const id = seen.size; seen.set(value, id);
    const next = child => encode(child, depth + 1);
    if (Array.isArray(value)) return ['array', id, value.map(next)];
    if (value instanceof Map) return ['map', id, [...value].map(([key, item]) => [next(key), next(item)])];
    if (value instanceof Set) return ['set', id, [...value].map(next)];
    if (value instanceof Date) return ['date', id, value.toISOString()];
    if (value instanceof ArrayBuffer) return ['buffer', id, hex(new Uint8Array(value))];
    if (ArrayBuffer.isView(value)) return [value.constructor.name, id, hex(new Uint8Array(value.buffer, value.byteOffset, value.byteLength))];
    if (![Object.prototype, null].includes(Object.getPrototypeOf(value))) throw new Error('Snapshot retains an unsupported host object');
    return ['object', id, Object.keys(value).sort().map(key => [key, next(value[key])])];
  }
  return new TextEncoder().encode(JSON.stringify(encode(value)));
}
const snapshotHeader = ({ integrity, memory, ...rest }) => rest;
function assertCloneableData(root) {
  const seen = new Set(), pending = [root];
  while (pending.length) {
    const value = pending.pop();
    if (value == null || ['string', 'boolean', 'number', 'bigint', 'undefined'].includes(typeof value)) continue;
    if (typeof value !== 'object') throw new Error('Snapshot retains an unsupported JavaScript reference');
    if (seen.has(value)) continue;
    seen.add(value);
    if (seen.size > 1_000_000) throw new Error('Snapshot data exceeds limits');
    if (value instanceof Date || value instanceof ArrayBuffer || ArrayBuffer.isView(value)) continue;
    if (value instanceof Map) { for (const [key, item] of value) pending.push(key, item); continue; }
    if (value instanceof Set) { for (const item of value) pending.push(item); continue; }
    if (!Array.isArray(value) && ![Object.prototype, null].includes(Object.getPrototypeOf(value))) throw new Error('Snapshot retains an unsupported host object');
    for (const key of Object.keys(value)) {
      const descriptor = Object.getOwnPropertyDescriptor(value, key);
      if (!Object.hasOwn(descriptor, 'value')) throw new Error('Snapshot retains a JavaScript accessor');
      pending.push(descriptor.value);
    }
  }
}
export async function sealExactSnapshot(snapshot) {
  snapshot.memoryHash = await snapshotDigest(snapshot.memory);
  snapshot.integrity = await snapshotDigest(snapshotDataBytes(snapshotHeader(snapshot)));
  return snapshot;
}
export async function validateExactSnapshotHeader(snapshot) {
  if (snapshot?.version !== EXACT_SNAPSHOT_VERSION) throw new Error('Exact snapshot schema is incompatible');
  if (await snapshotDigest(snapshotDataBytes(snapshotHeader(snapshot))) !== snapshot.integrity) throw new Error('Exact snapshot integrity mismatch');
}
export function validateExactSnapshotShape(snapshot, buildId) {
  if (snapshot?.version !== EXACT_SNAPSHOT_VERSION || snapshot.buildId !== buildId) throw new Error('Exact snapshot build is incompatible');
  if (!(snapshot.memory instanceof Uint8Array) || !snapshot.memory.byteLength || snapshot.memory.byteLength % PAGE
      || snapshot.memory.byteLength > 2 ** 32 || !Number.isInteger(snapshot.pointer) || snapshot.pointer < 8
      || snapshot.pointer % 4 || snapshot.pointer >= snapshot.memory.byteLength) throw new Error('Invalid exact snapshot memory or root');
}
export async function validateExactSnapshot(snapshot, buildId) {
  validateExactSnapshotShape(snapshot, buildId);
  await validateExactSnapshotHeader(snapshot);
  if (await snapshotDigest(snapshot.memory) !== snapshot.memoryHash) throw new Error('Exact snapshot integrity mismatch');
}
// Standard wasm-bindgen packages do not expose the instance-image bindings.
// Keep native savepoints and verified transcript replay available in those builds.
export function createAvailableExactBuildSnapshotRuntime({ exports, bindings }) {
  const { exactSnapshotBuildId, exactSnapshotLayout, replaceEngineInstance, attachExactBuildGame } = bindings;
  if (typeof exactSnapshotBuildId !== 'string' || !exactSnapshotBuildId
      || !Array.isArray(exactSnapshotLayout?.globals) || !Array.isArray(exactSnapshotLayout?.tables)
      || typeof replaceEngineInstance !== 'function' || typeof attachExactBuildGame !== 'function') return null;
  return createExactBuildSnapshotRuntime({ exports, layout: exactSnapshotLayout, buildId: exactSnapshotBuildId,
    replace: replaceEngineInstance, attach: attachExactBuildGame });
}
export function createExactBuildSnapshotRuntime({ exports: initial, layout, buildId, replace, attach }) {
  let exports = initial;
  const initialFunctions = layout.tables.filter(table => table.kind === 'function').map(({ name }) => {
    const table = exports[name];
    return { name, entries: Array.from({ length: table.length }, (_, index) => table.get(index)) };
  });
  const checkFunctions = () => {
    for (const { name, entries } of initialFunctions) {
      const table = exports[name];
      if (table.length !== entries.length || entries.some((entry, index) => table.get(index) !== entry)) throw new Error('Snapshot function table has changed');
    }
  };
  // Copy synchronously; the caller's worker queue owns this quiet boundary.
  const copyInstance = (game, extra = {}) => {
    checkFunctions();
    const data = { version: EXACT_SNAPSHOT_VERSION, buildId,
      pointer: game.__wbg_ptr,
      globals: layout.globals.map(({ name }) => exports[name].value),
      references: layout.tables.filter(table => table.kind === 'reference').map(({ name }) => {
        const table = exports[name]; return Array.from({ length: table.length }, (_, index) => table.get(index));
      }), ...extra };
    // structuredClone would silently erase a custom class's prototype.
    // Refuse such host resources before copying any reference table entries.
    assertCloneableData(data);
    return { ...structuredClone(data), memory: new Uint8Array(exports.memory.buffer).slice() };
  };
  const apply = (snapshot, oldGame) => {
    if (snapshot.globals.length !== layout.globals.length
        || snapshot.references.length !== layout.tables.filter(table => table.kind === 'reference').length) throw new Error('Exact snapshot instance layout mismatch');
    // Do not let the old wrapper's finalizer free a pointer in the new heap.
    // A fresh analysis worker has no wrapper to detach.
    oldGame?.__destroy_into_raw();
    exports = replace();
    const difference = snapshot.memory.byteLength - exports.memory.buffer.byteLength;
    if (difference < 0) throw new Error('Exact snapshot memory is smaller than the initial instance');
    if (difference) exports.memory.grow(difference / PAGE);
    new Uint8Array(exports.memory.buffer).set(snapshot.memory);
    layout.globals.forEach(({ name, mutable }, index) => {
      if (mutable) exports[name].value = snapshot.globals[index];
      else if (!Object.is(exports[name].value, snapshot.globals[index])) throw new Error('Exact snapshot immutable global mismatch');
    });
    const references = structuredClone(snapshot.references);
    layout.tables.filter(table => table.kind === 'reference').forEach(({ name }, index) => {
      const table = exports[name], values = references[index];
      if (!Array.isArray(values) || values.length < table.length) throw new Error('Exact snapshot reference table mismatch');
      if (values.length > table.length) table.grow(values.length - table.length);
      for (let index = 0; index < values.length; index++) table.set(index, values[index]);
    });
    // Recreated function references have different JS identities. Rebase the
    // capture check onto this instance's table, whose element segments match.
    for (const baseline of initialFunctions) baseline.entries = Array.from({ length: exports[baseline.name].length }, (_, index) => exports[baseline.name].get(index));
    return attach(snapshot.pointer);
  };
  return {
    buildId,
    get exports() { return exports; },
    async capture(game, recovery) {
      return sealExactSnapshot(copyInstance(game, { recovery }));
    },
    async restore(snapshot, oldGame) {
      await validateExactSnapshot(snapshot, buildId);
      return apply(snapshot, oldGame);
    },
    // Analysis seeds never leave this browser session, so they skip the
    // integrity digests (~6x the copy cost) but keep every structural check.
    captureLocal(game) {
      return copyInstance(game);
    },
    restoreLocal(snapshot, oldGame) {
      validateExactSnapshotShape(snapshot, buildId);
      return apply(snapshot, oldGame);
    },
    reset() {
      exports = replace();
      for (const baseline of initialFunctions) baseline.entries = Array.from({ length: exports[baseline.name].length }, (_, index) => exports[baseline.name].get(index));
      return exports;
    },
  };
}
// Match/seat/prefix checks happen before touching WASM; signed public hashes
// are checked after restore and again after replaying the remaining actions.
export function exactSnapshotMatches(point, { matchId, seat, actions }) {
  const anchor = actions[point?.seq - 1];
  return point?.matchId === matchId && point.seat === seat && Number.isSafeInteger(point.seq) && point.seq > 0
    && point.seq <= actions.length && point.prefixHash === anchor?.prefixHash
    && point.auditStateHash === anchor?.audit?.nextStateHash
    && point.publicStateHash === anchor?.audit?.publicCheckpointHash;
}
