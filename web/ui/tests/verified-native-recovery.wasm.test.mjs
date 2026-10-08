import test from 'node:test';
import assert from 'node:assert/strict';
import { webcrypto } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import init, { WasmGame } from '../../wasm_demo/pkg/engine.js';
import { recoverVerifiedRuntime } from '../src/lib/local-runtime-recovery.js';
import { publicCheckpointHash, CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION } from '../src/lib/multiplayer-audit.js';

await init({ module_or_path: await readFile(new URL('../../wasm_demo/pkg/engine_bg.wasm', import.meta.url)) });

for (const poisonSaved of [false, true]) {
  test(`real native recovery rejects corrupt current state and uses ${poisonSaved ? 'genesis' : 'a local savepoint'}`, async () => {
    const game = new WasmGame(), handles = [], failures = [];
    const hash = () => {
      const checkpoint = game.exportPublicAuditCheckpoint();
      assert.equal(CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION, 11);
      assert.equal(checkpoint.version, CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION);
      assert.ok(Object.hasOwn(checkpoint, 'lastAttackDeclarationStepPlayers'));
      return publicCheckpointHash(checkpoint, webcrypto);
    };
    const retain = seq => {
      const runtimeHandle = game.createRuntimeSavepoint();
      handles.push(runtimeHandle);
      return { level: 'local-savepoint', seq, runtimeHandle };
    };
    const deltas = [2, 3], hashes = [];
    try {
      game.resetEmpty(['Alice', 'Bob'], 20);
      game.addLifeDelta(0, deltas[0]);
      hashes.push(await hash());
      const saved = retain(1);
      game.addLifeDelta(0, deltas[1]);
      hashes.push(await hash());
      // Accepted transcript bookkeeping still claims sequence 2, while the
      // actual current runtime has diverged. Never trust that cursor alone.
      game.addLifeDelta(0, -9);
      const current = { ...retain(2), level: 'current' };
      if (poisonSaved) {
        game.releaseRuntimeSavepoint(saved.runtimeHandle);
        saved.runtimeHandle = retain(1).runtimeHandle;
      }
      const recovered = await recoverVerifiedRuntime({
        current, saved: [saved],
        restore: async point => {
          game.copyRuntimeSavepoint(point.runtimeHandle);
          assert.equal(await hash(), hashes[point.seq - 1], 'saved runtime must match its accepted anchor');
        },
        genesis: () => game.resetEmpty(['Alice', 'Bob'], 20),
        replay: seq => { for (const delta of deltas.slice(seq)) game.addLifeDelta(0, delta); },
        verify: async () => assert.equal(await hash(), hashes[1], 'recovery must reproduce the accepted head'),
        onFailure: point => failures.push(point.level),
      });
      assert.equal(recovered.level, poisonSaved ? 'genesis' : 'local-savepoint');
      assert.deepEqual(failures, poisonSaved ? ['current', 'local-savepoint'] : ['current']);
      assert.equal(game.uiState().players[0].life, 25);
    } finally {
      for (const handle of handles) game.releaseRuntimeSavepoint(handle);
      game.free();
    }
  });
}
