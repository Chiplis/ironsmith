import { exactSnapshotBuildId, exactSnapshotLayout, replaceEngineInstance, attachExactBuildGame } from '../../../wasm_demo/pkg/engine.js';
import { createExactBuildSnapshotRuntime } from '../lib/exact-build-snapshot.js';

// An analysis worker owns its engine instance; a seed replaces it wholesale.
let snapshots = null;
export async function restoreAnalysisSeed(initialization, image, oldGame) {
  snapshots ||= createExactBuildSnapshotRuntime({ exports: await initialization, layout: exactSnapshotLayout,
    buildId: exactSnapshotBuildId, replace: replaceEngineInstance, attach: attachExactBuildGame });
  return snapshots.restoreLocal(image, oldGame);
}
