#!/usr/bin/env node
// Run after wasm-bindgen/wasm-opt: expose instance state without effect codecs.
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const marker = '\n// Ironsmith exact-build snapshot adapter\n';
const prefix = '__ironsmith_snapshot_';
const leb = value => { const out = []; do { let byte = value & 127; value = Math.floor(value / 128); out.push(byte | (value ? 128 : 0)); } while (value); return out; };
const name = value => { const bytes = new TextEncoder().encode(value); return [...leb(bytes.length), ...bytes]; };
function reader(bytes) {
  let pos = 0;
  return { get pos() { return pos; }, byte: () => bytes[pos++],
    uint() { let value = 0, shift = 0, byte; do { byte = bytes[pos++]; if (byte == null || shift > 28) throw Error('Invalid WASM integer'); value += (byte & 127) * 2 ** shift; shift += 7; } while (byte & 128); return value; },
    skip(count) { pos += count; if (pos > bytes.length) throw Error('Truncated WASM'); },
    text() { const size = this.uint(); const result = new TextDecoder().decode(bytes.subarray(pos, pos + size)); this.skip(size); return result; },
  };
}
export function exposeInstanceState(binary) {
  const bytes = new Uint8Array(binary), r = reader(bytes); r.skip(8);
  const sections = [], globals = [], tables = [];
  while (r.pos < bytes.length) {
    const id = r.byte(), size = r.uint(), start = r.pos;
    sections.push({ id, bytes: bytes.slice(start, start + size) }); r.skip(size);
  }
  for (const section of sections) {
    const s = reader(section.bytes);
    if (section.id === 2) for (let count = s.uint(); count > 0; count--) {
      s.text(); s.text(); const kind = s.byte();
      if (kind !== 0) throw Error('Exact snapshots require function-only WASM imports');
      s.uint();
    }
    if (section.id === 4) for (let count = s.uint(); count > 0; count--) {
      const type = s.byte(), flags = s.uint();
      if (![0x70, 0x6f].includes(type) || flags > 1) throw Error('Unsupported WASM table');
      s.uint(); if (flags & 1) s.uint();
      tables.push({ name: `${prefix}table_${tables.length}`, kind: type === 0x70 ? 'function' : 'reference' });
    }
    if (section.id === 6) for (let count = s.uint(); count > 0; count--) {
      const type = s.byte(), mutable = s.byte();
      if (![0x7f, 0x7e, 0x7d, 0x7c].includes(type)) throw Error('Unsupported WASM global');
      globals.push({ name: `${prefix}global_${globals.length}`, mutable: Boolean(mutable) });
      const op = s.byte();
      if (op === 0x41 || op === 0x42) { while (s.byte() & 128) { /* signed LEB constant */ } }
      else if (op === 0x43) s.skip(4);
      else if (op === 0x44) s.skip(8);
      else throw Error('Unsupported WASM global initializer');
      if (s.byte() !== 0x0b) throw Error('Unsupported WASM constant expression');
    }
  }
  const exports = sections.find(section => section.id === 7);
  if (!exports) throw Error('WASM exports missing');
  const e = reader(exports.bytes), entries = [];
  for (let count = e.uint(); count > 0; count--) {
    const label = e.text(), kind = e.byte(), index = e.uint();
    if (!label.startsWith(prefix)) entries.push([...name(label), kind, ...leb(index)]);
  }
  globals.forEach((global, index) => entries.push([...name(global.name), 3, ...leb(index)]));
  tables.forEach((table, index) => entries.push([...name(table.name), 1, ...leb(index)]));
  exports.bytes = Uint8Array.from([...leb(entries.length), ...entries.flat()]);
  const chunks = [bytes.slice(0, 8), ...sections.flatMap(section => [Uint8Array.from([section.id, ...leb(section.bytes.length)]), section.bytes])];
  const output = new Uint8Array(chunks.reduce((size, chunk) => size + chunk.length, 0));
  let offset = 0; for (const chunk of chunks) { output.set(chunk, offset); offset += chunk.length; }
  if (!WebAssembly.validate(output)) throw Error('Snapshot instrumentation produced invalid WASM');
  return { binary: output, layout: { globals, tables } };
}
export async function prepareExactBuildSnapshots(directory) {
  const binaryPath = resolve(directory, 'engine_bg.wasm'), gluePath = resolve(directory, 'engine.js');
  const { binary, layout } = exposeInstanceState(await readFile(binaryPath));
  const glue = (await readFile(gluePath, 'utf8')).split(marker)[0];
  for (const required of ['__wbg_get_imports', '__wbg_finalize_init', 'WasmGameFinalization', '__destroy_into_raw']) {
    if (!glue.includes(required)) throw Error(`Unsupported wasm-bindgen glue: ${required}`);
  }
  const adapter = `export const exactSnapshotLayout = ${JSON.stringify(layout)};
// Only the owning worker may replace an instance. Detach its old wrapper first.
export function replaceEngineInstance() {
    if (!wasmModule) throw new Error('Engine has not been initialized');
    WASM_VECTOR_LEN = 0;
    return __wbg_finalize_init(new WebAssembly.Instance(wasmModule, __wbg_get_imports()), wasmModule);
}
export function attachExactBuildGame(pointer) {
    const game = Object.create(WasmGame.prototype);
    game.__wbg_ptr = pointer >>> 0;
    WasmGameFinalization.register(game, game.__wbg_ptr, game);
    return game;
}
`;
  const facade = await readFile(resolve(directory, 'ironsmith.js'));
  const hash = createHash('sha256').update(binary).update(glue).update(adapter).update(facade);
  // Bind the image to the JS recovery implementation as well as Rust/glue.
  // A deployment can change host bookkeeping without changing the WASM bytes.
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  for (const file of ['lib/exact-build-snapshot.js', 'lib/local-analysis-replay.js', 'hooks/useWasmGame.js',
    'hooks/peer-lobby/crypto-resync.js', 'hooks/peer-lobby/messaging.js', 'workers/wasmGameWorker.js']) {
    hash.update(file).update(await readFile(resolve(root, 'web/ui/src', file)));
  }
  const buildId = hash.digest('hex');
  await writeFile(binaryPath, binary);
  await writeFile(gluePath, `${glue}${marker}export const exactSnapshotBuildId = '${buildId}';\n${adapter}`);
  const declarationsPath = resolve(directory, 'engine.d.ts');
  const declarations = (await readFile(declarationsPath, 'utf8')).split(marker)[0];
  await writeFile(declarationsPath, `${declarations}${marker}export const exactSnapshotBuildId: string;
export const exactSnapshotLayout: { globals: Array<{name: string; mutable: boolean}>; tables: Array<{name: string; kind: 'function' | 'reference'}> };
export function replaceEngineInstance(): InitOutput;
export function attachExactBuildGame(pointer: number): WasmGame;
`);
  return buildId;
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  console.log('Exact snapshot build:', await prepareExactBuildSnapshots(resolve(process.argv[2] || dirname(fileURLToPath(import.meta.url)))));
}
