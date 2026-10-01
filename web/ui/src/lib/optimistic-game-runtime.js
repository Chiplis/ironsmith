// Material dependency checks concern calculation, not proof validity. Claims
// may fill a public identity provisionally; they never enter the audit caches.
export async function missingCalculationMaterial(game, requirements = [], ready = new Set(), localPlayerIndex = null) {
  for (const requirement of requirements) {
    const type = String(requirement.type || requirement.requirement_type || '');
    if (type === 'hidden_move' || type === 'public_view_window' || type === 'private_view_window') continue;
    if (ready.has(String(requirement.id))) continue;
    // A private disclosure to somebody else supplies no identity to this seat.
    // Continue with the same placeholders canonical replay uses for non-viewers.
    if (type === 'private_open' && localPlayerIndex != null && requirement.viewer != null
      && Number(requirement.viewer) !== Number(localPlayerIndex)) continue;
    if (type === 'public_open' || type === 'private_open') {
      const objectId = Number(requirement.objectId ?? requirement.object_id);
      if (Number.isSafeInteger(objectId) && objectId > 0) {
        const opened = await game.hiddenCardOpenState(BigInt(objectId));
        if (opened?.open) continue;
      }
    }
    // Randomness and unknown identities are never guessed. A view window or
    // shuffle needs its material before dependent choices can be calculated.
    return requirement;
  }
  return null;
}

// Use material already carried by a canonical envelope without doing signature
// or shuffle verification here. It stays inside the provisional transaction;
// canonical replay independently authenticates and installs it on its branch.
async function prepareCalculationMaterial(game, command, audit, initial) {
  const ready = new Set();
  let requirements = initial;
  for (let attempt = 0; attempt < 256; attempt++) {
    let added = false;
    for (const requirement of requirements) {
      const id = String(requirement.id);
      if (ready.has(id)) continue;
      const type = String(requirement.type);
      if (type === 'fair_random') {
        const reveal = audit?.rngReveals?.find(entry => String(entry.requirementId) === id);
        if (!/^[a-f0-9]{64}$/i.test(String(reveal?.combinedSeedHex || ''))) continue;
        await game.injectTranscriptRandomSeeds({ seeds: [reveal.combinedSeedHex], libraryShuffles: [] });
      } else if (type === 'verifiable_shuffle') {
        const proof = audit?.shuffleProofs?.find(entry => String(entry.requirementId) === id
          && Number(entry.owner) === Number(requirement.owner));
        if (!proof?.deckHash) continue;
        if (proof.inputDeck) {
          const expectedInputs = requirement.inputCommitments ?? requirement.input_commitments;
          const randomCountBefore = Number(requirement.randomCountBefore ?? requirement.random_count_before);
          if (!Array.isArray(expectedInputs) || expectedInputs.length !== Number(proof.deckCount)
            || !Number.isSafeInteger(randomCountBefore) || randomCountBefore < 0) continue;
          await game.queueVerifiedHiddenLibraryEpoch({ owner: Number(proof.owner), deckHash: String(proof.deckHash),
            count: Number(proof.deckCount), randomCountBefore, expectedInputs });
          for (const opening of audit.openings || []) {
            if (String(opening.positionCommitment || '').startsWith(`ziffle:${proof.deckHash}:`)) {
              await game.queueVerifiedHiddenLibraryOpening({ owner: Number(opening.owner), deckHash: String(proof.deckHash),
                position: Number(opening.position), originalSlot: Number(opening.slot),
                cardName: String(opening.card), commitment: String(opening.commitment || '') });
            }
          }
        } else {
          const beforeOrder = proof.beforeOrder ?? proof.before_order;
          const afterOrder = proof.afterOrder ?? proof.after_order;
          if (!Array.isArray(beforeOrder) || !Array.isArray(afterOrder) || beforeOrder.length !== afterOrder.length) continue;
          await game.injectTranscriptRandomSeeds({ seeds: [String(proof.deckHash)],
            libraryShuffles: [{ owner: Number(proof.owner), beforeOrder, afterOrder }] });
        }
      } else if (type === 'public_open') {
        const objectId = Number(requirement.objectId ?? requirement.object_id);
        const opening = audit?.openings?.find(entry => Number(entry.objectId ?? entry.object_id) === objectId
          && Number(entry.owner) === Number(requirement.owner));
        if (!opening?.card || !Number.isSafeInteger(objectId) || objectId <= 0) continue;
        // Only public disclosures requested by our own preview may be opened.
        await game.revealHiddenObject({ objectId, cardName: String(opening.card),
          commitment: String(opening.commitment || ''), recomputeDecision: true });
      } else continue;
      ready.add(id); added = true;
    }
    if (!added) return { requirements, ready };
    requirements = await game.previewCryptoRequirements(command);
  }
  throw new Error('Provisional material requirements did not converge');
}

export async function calculateOptimisticAction(game, candidate, {
  prepare = async () => {}, dispatch, requirementsFromState, checkpointHash,
  localPlayerIndex = null,
}) {
  const handle = await game.createRuntimeSavepoint();
  let keep = false;
  try {
    await prepare(game, candidate);
    const before = await game.uiState();
    const beforeHash = await checkpointHash(await game.exportPublicAuditCheckpoint());
    if (candidate.prePublicCheckpointHash && candidate.prePublicCheckpointHash !== beforeHash) return null;
    const nonDispatch = ['cancel_decision', 'forfeit_player'].includes(candidate.command?.type);
    const preview = nonDispatch ? [] : await game.previewCryptoRequirements(candidate.command);
    const { requirements, ready } = nonDispatch ? { requirements: [], ready: new Set() }
      : await prepareCalculationMaterial(game, candidate.command, candidate.calculationAudit,
        Array.isArray(preview) ? preview : []);
    if (await missingCalculationMaterial(game, requirements, ready, localPlayerIndex)) return null;
    const state = await dispatch(candidate.command, before);
    if (await missingCalculationMaterial(game, requirementsFromState(state), ready, localPlayerIndex)) return null;
    const publicCheckpointHash = await checkpointHash(await game.exportPublicAuditCheckpoint());
    if (candidate.publicCheckpointHash && candidate.publicCheckpointHash !== publicCheckpointHash) return null;
    keep = true;
    return { state, prePublicCheckpointHash: beforeHash, publicCheckpointHash };
  } finally {
    try { if (!keep) await game.restoreRuntimeSavepoint(handle); }
    finally { await game.releaseRuntimeSavepoint(handle); }
  }
}
