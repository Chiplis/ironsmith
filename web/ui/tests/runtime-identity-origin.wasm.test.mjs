import test from 'node:test';
import assert from 'node:assert/strict';
import init, { WasmGame } from '../../wasm_demo/pkg/engine.js';
import { readFile } from 'node:fs/promises';

await init({ module_or_path: await readFile(new URL('../../wasm_demo/pkg/engine_bg.wasm', import.meta.url)) });

test('analysis identity initialization carries only allocators and refuses live game replacement', () => {
  const source = new WasmGame(), replica = new WasmGame();
  try {
    const origin = source.getRuntimeIdentityOrigin();
    assert.deepEqual(Object.keys(origin).sort(), ['card', 'object', 'player', 'stackAbility']);
    assert.throws(() => replica.initializeRuntimeIdentityOrigin({ ...origin, objects: [] }), /unknown field/);
    replica.initializeRuntimeIdentityOrigin(origin);
    assert.equal(replica.getRuntimeIdentityOrigin().object, origin.object);
    assert.equal(replica.getRuntimeIdentityOrigin().stackAbility, origin.stackAbility);
    assert.throws(() => replica.initializeRuntimeIdentityOrigin(origin), /fresh runtime/);
    replica.resetEmpty(['Alice', 'Bob'], 20);
    const before = replica.exportPublicAuditCheckpoint();
    assert.throws(() => replica.initializeRuntimeIdentityOrigin(origin), /fresh runtime/);
    assert.deepEqual(replica.exportPublicAuditCheckpoint(), before);
    for (const method of ['exportSyncCheckpoint', 'exportRedactedSyncCheckpoint', 'importSyncCheckpoint', 'importForeignSyncCheckpoint', 'isReplayCheckpointBoundary']) {
      assert.equal(typeof replica[method], 'undefined', `${method} must be absent from the WASM interface`);
    }
  } finally { source.free(); replica.free(); }
});
