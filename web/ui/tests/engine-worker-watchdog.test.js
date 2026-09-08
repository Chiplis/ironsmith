import test from "node:test";
import assert from "node:assert/strict";
import {
  engineWorkerTimeoutMs,
  makeEngineWorkerStallError,
  shouldWatchEngineMethod,
} from "../src/lib/engine-worker-watchdog.js";

test("the watchdog covers authoritative mutations but not read-only engine work", () => {
  for (const method of ["dispatch", "advancePhase", "cancelDecision", "forfeitPlayer"]) {
    assert.equal(shouldWatchEngineMethod(method), true, method);
  }
  for (const method of ["snapshot", "uiState", "exportSyncCheckpoint", "objectDetails"]) {
    assert.equal(shouldWatchEngineMethod(method), false, method);
  }
});

test("the watchdog timeout has a safe default and bounded configuration", () => {
  assert.equal(engineWorkerTimeoutMs(undefined), 15_000);
  assert.equal(engineWorkerTimeoutMs("bad"), 15_000);
  assert.equal(engineWorkerTimeoutMs(100), 5_000);
  assert.equal(engineWorkerTimeoutMs(25_000), 25_000);
  assert.equal(engineWorkerTimeoutMs(500_000), 120_000);
});

test("stall errors explicitly require authoritative resync", () => {
  const error = makeEngineWorkerStallError("dispatch", 15_000, true);
  assert.equal(error.code, "ENGINE_WORKER_STALL");
  assert.equal(error.engineRecoveryAttempted, true);
  assert.equal(error.engineCheckpointAvailable, true);
  assert.equal(error.requiresAuthoritativeResync, true);
});
