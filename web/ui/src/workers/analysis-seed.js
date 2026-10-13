import * as engineBindings from '../../../wasm_demo/pkg/engine.js';
import { createAvailableExactBuildSnapshotRuntime } from '../lib/exact-build-snapshot.js';

// An analysis worker owns its engine instance; a seed replaces it wholesale.
// Standard wasm-bindgen packages carry no instance-image bindings; importing
// them by name would fail the whole worker module, so they are read off the
// namespace and a seed is simply unavailable there (callers fall back to
// replaying the session).
let snapshots = null;
export async function restoreAnalysisSeed(initialization, image, oldGame) {
  snapshots ??= createAvailableExactBuildSnapshotRuntime({ exports: await initialization, bindings: engineBindings });
  if (!snapshots) throw new Error('Exact-build snapshots are unavailable in this engine package');
  return snapshots.restoreLocal(image, oldGame);
}
