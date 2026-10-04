// Lossless local restore points are scoped to one live engine instance.
export async function captureEngineRestorePoint(game) {
  if (!game || game.supportsRuntimeSavepoints === false
    || typeof game.createRuntimeSavepoint !== 'function'
    || typeof game.restoreRuntimeSavepoint !== 'function') {
    throw new Error('Game engine does not support native runtime restore points');
  }
  return { game, runtimeHandle: await game.createRuntimeSavepoint() };
}

export async function restoreEngineRestorePoint(game, point) {
  if (!point) return;
  if (point.game !== game || point.runtimeHandle == null) throw new Error('Engine restore point has expired');
  const handle = point.runtimeHandle;
  point.runtimeHandle = null;
  await game.restoreRuntimeSavepoint(handle);
}
