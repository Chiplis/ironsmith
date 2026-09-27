// Called inside the engine worker's authoritative queue. Keep the whole
// save/inject/preview/restore transaction together so another command cannot
// observe temporary shuffle material or be rolled back by the preview.
export function previewCryptoRequirementsWithMaterial(game, command, material) {
  const handle = game.createRuntimeSavepoint();
  try {
    game.injectTranscriptRandomSeeds(material);
    for (const epoch of material.libraryEpochs || []) {
      game.queueVerifiedHiddenLibraryEpoch(epoch);
    }
    for (const opening of material.libraryEpochOpenings || []) {
      game.queueVerifiedHiddenLibraryOpening(opening);
    }
    return game.previewCryptoRequirements(command);
  } finally {
    try {
      game.restoreRuntimeSavepoint(handle);
    } finally {
      game.releaseRuntimeSavepoint(handle);
    }
  }
}
