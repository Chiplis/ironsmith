import test from 'node:test';
import assert from 'node:assert/strict';
import {exposeInstanceState} from './prepare-exact-build-snapshots.mjs';

test('snapshot instrumentation exports numeric globals and both table types without changing initial state',()=>{
 // Two tables, a one-page memory, one mutable i32 global initialized to 7,
 // and the ordinary memory export. No executable code is required.
 const binary=Uint8Array.from([0,97,115,109,1,0,0,0,
  4,7,2,0x70,0,1,0x6f,0,1,
  5,3,1,0,1,
  6,6,1,0x7f,1,0x41,7,0x0b,
  7,10,1,6,109,101,109,111,114,121,2,0]);
 const {binary:patched,layout}=exposeInstanceState(binary);
 const instance=new WebAssembly.Instance(new WebAssembly.Module(patched));
 assert.equal(instance.exports.memory.buffer.byteLength,65536);
 assert.equal(instance.exports[layout.globals[0].name].value,7);
 instance.exports[layout.globals[0].name].value=13;
 assert.equal(instance.exports[layout.globals[0].name].value,13);
 assert.deepEqual(layout.tables.map(table=>table.kind),['function','reference']);
 assert.deepEqual(exposeInstanceState(patched).binary,patched,'instrumentation is idempotent');
});
