// Local "undo this sandbox" restore points for one live engine instance.
//
// A sync checkpoint is a wire format: it carries objects, zones, players and the
// stack, but not the full rules state (continuous effects, delayed triggers,
// replacement/prevention shields, turn history, pending triggers, live
// continuations...). Importing one back into the same engine therefore rewinds
// that state to defaults and the peer diverges on the next action that reads
// it. A runtime savepoint clones the whole engine session instead, so restoring
// it is lossless. The checkpoint is only a fallback for engines that cannot
// create savepoints (old builds, or a worker that was replaced meanwhile).

function canCreateRuntimeSavepoint(game) {
  return Boolean(game)
    && game.supportsRuntimeSavepoints !== false
    && typeof game.createRuntimeSavepoint === "function"
    && typeof game.restoreRuntimeSavepoint === "function";
}

/**
 * Capture a restore point for `game`. With `keepCheckpoint`, a sync checkpoint
 * is exported as well, as a fallback for a restore that outlives the savepoint.
 */
export async function captureEngineRestorePoint(game, { keepCheckpoint = false } = {}) {
  let runtimeHandle = null;
  if (canCreateRuntimeSavepoint(game)) {
    try {
      runtimeHandle = await game.createRuntimeSavepoint();
    } catch {
      runtimeHandle = null;
    }
  }
  const needsCheckpoint = runtimeHandle == null || keepCheckpoint;
  const checkpoint = needsCheckpoint && typeof game?.exportSyncCheckpoint === "function"
    ? await game.exportSyncCheckpoint()
    : null;
  if (runtimeHandle == null && !checkpoint) {
    throw new Error("Game engine cannot capture a restore point");
  }
  return { game, runtimeHandle, checkpoint };
}

/**
 * Restore `game` to `point` and consume it. Prefers the lossless savepoint;
 * falls back to the checkpoint when the savepoint is gone or belongs to another
 * engine instance.
 */
export async function restoreEngineRestorePoint(game, point, perspective) {
  if (!point) return;
  const runtimeHandle = point.runtimeHandle;
  point.runtimeHandle = null;
  if (runtimeHandle != null && point.game === game) {
    try {
      await game.restoreRuntimeSavepoint(runtimeHandle);
      return;
    } catch (error) {
      if (!point.checkpoint) throw error;
    }
  }
  if (!point.checkpoint || typeof game?.importSyncCheckpoint !== "function") {
    throw new Error("Engine restore point has expired");
  }
  await game.importSyncCheckpoint(point.checkpoint, perspective);
}
