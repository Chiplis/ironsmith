import {
  INITIAL_AUDIT_STATE_HASH,
  ZIFFLE_OPENING_PREVIEW_BATCH_SIZE,
  auditStateHash,
  buildDeckSlotOpening,
  buildPrivateDeckManifest,
  buildSignedActionEnvelope,
  buildZiffleOpeningProof,
  cachedOpeningMatchesZifflePosition,
  canonicalMultiplayerPayload,
  checkpointObjectForId,
  checkpointObjectHiddenCard,
  checkpointObjectIsRedactedHidden,
  checkpointObjectName,
  chunkList,
  cloneMultiplayerPayload,
  collectCommandObjectIds,
  decklistHashForCards,
  fairRandomCombinedSeedHex,
  hiddenCardMetadataForObjectFromCheckpoint,
  hiddenMetadataMatchesZifflePosition,
  hiddenObjectIdForOpeningFromCheckpoint,
  isNonDispatchSyncCommand,
  isOwnerPrivateViewRequirement,
  knownCheckpointObjectMatchesOpening,
  mergeActionOpeningPreviews,
  normalizeShuffleOrder,
  notifyOpeningBuilt,
  openingHasZifflePosition,
  openingMatchesRequirement,
  openingShuffleSourceId,
  publicCheckpointHash,
  reindexPlayers,
  removeStoredRevealedOpening,
  requirementHasZifflePosition,
  resolveLocalPlayerIndex,
  sanitizeCardList,
  selectObjectCandidateForId,
  selectObjectCandidateRevealPolicy,
  shuffleProofMatchesRequirement,
  useCallback,
  verifyAuditPayload,
  verifyCardOpeningAgainstManifest,
  wasmObjectIdArg,
  withPinnedPublicZifflePosition,
  ziffleContextForCommitment,
  ziffleContextFromCeremony,
  ziffleContextFromOpening,
  ziffleDeckHashFromCommitment,
  ziffleIdentityPositionFromSources,
  ziffleKeyContextForCeremony,
  zifflePositionFromCommitment,
  zifflePublicPositionFromSources,
  ziffleRuntimeCommitment,
} from "./shared.js";

export function usePeerLobbyAuditMaterial(base, servicesRef) {
  const { auditStateHashRef, gameRef, initialPublicCheckpointHashRef, localRevealedOpeningsRef, matchStartPayloadRef, multiplayerRef, setState, stateRef, verifiedAuditOpeningsRef } = base;
  const actionHistoryEntryForSequence = useCallback((...args) => servicesRef.current.actionHistoryEntryForSequence(...args), [servicesRef]);
  const auditEncryptionPublicKeyForPlayer = useCallback((...args) => servicesRef.current.auditEncryptionPublicKeyForPlayer(...args), [servicesRef]);
  const collectZiffleRevealTokens = useCallback((...args) => servicesRef.current.collectZiffleRevealTokens(...args), [servicesRef]);
  const collectZiffleRevealTokensBatch = useCallback((...args) => servicesRef.current.collectZiffleRevealTokensBatch(...args), [servicesRef]);
  const commandObjectHiddenRefs = useCallback((...args) => servicesRef.current.commandObjectHiddenRefs(...args), [servicesRef]);
  const commandObjectStableIds = useCallback((...args) => servicesRef.current.commandObjectStableIds(...args), [servicesRef]);
  const currentAuditMatchId = useCallback((...args) => servicesRef.current.currentAuditMatchId(...args), [servicesRef]);
  const currentObjectIdForHiddenRef = useCallback((...args) => servicesRef.current.currentObjectIdForHiddenRef(...args), [servicesRef]);
  const currentObjectIdForStableId = useCallback((...args) => servicesRef.current.currentObjectIdForStableId(...args), [servicesRef]);
  const ensureAuditIdentity = useCallback((...args) => servicesRef.current.ensureAuditIdentity(...args), [servicesRef]);
  const ensureZiffleOpeningProof = useCallback((...args) => servicesRef.current.ensureZiffleOpeningProof(...args), [servicesRef]);
  const importCachedAuditPublicKey = useCallback((...args) => servicesRef.current.importCachedAuditPublicKey(...args), [servicesRef]);
  const localRevealedOpeningForExport = useCallback((...args) => servicesRef.current.localRevealedOpeningForExport(...args), [servicesRef]);
  const localRevealedOpeningForRequirement = useCallback((...args) => servicesRef.current.localRevealedOpeningForRequirement(...args), [servicesRef]);
  const localRevealedOpeningForZiffleReveal = useCallback((...args) => servicesRef.current.localRevealedOpeningForZiffleReveal(...args), [servicesRef]);
  const privateDeckManifestForOwner = useCallback((...args) => servicesRef.current.privateDeckManifestForOwner(...args), [servicesRef]);
  const publicDeckManifestForOwner = useCallback((...args) => servicesRef.current.publicDeckManifestForOwner(...args), [servicesRef]);
  const publicKeyForAuditSigner = useCallback((...args) => servicesRef.current.publicKeyForAuditSigner(...args), [servicesRef]);
  const rememberLocalRevealedOpening = useCallback((...args) => servicesRef.current.rememberLocalRevealedOpening(...args), [servicesRef]);
  const rememberPrivateDeckManifest = useCallback((...args) => servicesRef.current.rememberPrivateDeckManifest(...args), [servicesRef]);
  const rememberZiffleOpeningPosition = useCallback((...args) => servicesRef.current.rememberZiffleOpeningPosition(...args), [servicesRef]);
  const resolveLocalCryptoPlayerIndex = useCallback((...args) => servicesRef.current.resolveLocalCryptoPlayerIndex(...args), [servicesRef]);
  const rngCommitmentForNonce = useCallback((...args) => servicesRef.current.rngCommitmentForNonce(...args), [servicesRef]);
  const updateMultiplayer = useCallback((...args) => servicesRef.current.updateMultiplayer(...args), [servicesRef]);
  const verifyRngCommitmentEntry = useCallback((...args) => servicesRef.current.verifyRngCommitmentEntry(...args), [servicesRef]);
  const verifyRngRevealEntry = useCallback((...args) => servicesRef.current.verifyRngRevealEntry(...args), [servicesRef]);
  const verifyZiffleOpeningProofForOpening = useCallback((...args) => servicesRef.current.verifyZiffleOpeningProofForOpening(...args), [servicesRef]);
  const ziffleCeremonyCandidatesForOwner = useCallback((...args) => servicesRef.current.ziffleCeremonyCandidatesForOwner(...args), [servicesRef]);
  const ziffleCeremonyForOwner = useCallback((...args) => servicesRef.current.ziffleCeremonyForOwner(...args), [servicesRef]);
  const ziffleCeremonyHasObjectOrder = useCallback((...args) => servicesRef.current.ziffleCeremonyHasObjectOrder(...args), [servicesRef]);
  const ziffleObjectOrderLinksOpening = useCallback((...args) => servicesRef.current.ziffleObjectOrderLinksOpening(...args), [servicesRef]);
  const ziffleOpeningPositionForSlot = useCallback((...args) => servicesRef.current.ziffleOpeningPositionForSlot(...args), [servicesRef]);
  const zifflePositionForObjectId = useCallback((...args) => servicesRef.current.zifflePositionForObjectId(...args), [servicesRef]);
  const zifflePositionForOriginalSlot = useCallback((...args) => servicesRef.current.zifflePositionForOriginalSlot(...args), [servicesRef]);
  const ziffleShuffleObjectIdForPosition = useCallback((...args) => servicesRef.current.ziffleShuffleObjectIdForPosition(...args), [servicesRef]);
  const ziffleShuffleOriginalSlotForPosition = useCallback((...args) => servicesRef.current.ziffleShuffleOriginalSlotForPosition(...args), [servicesRef]);
  const ziffleTokensForPosition = useCallback((...args) => servicesRef.current.ziffleTokensForPosition(...args), [servicesRef]);
  function verifiedAuditOpeningKey(opening) {
    return canonicalMultiplayerPayload({
      matchId: currentAuditMatchId(),
      opening,
    });
  }

  const verifyAuditOpeningsAgainstManifests = useCallback(async (openings = [], options = {}) => {
    for (const opening of openings || []) {
      if (!opening || opening.owner == null || opening.slot == null) continue;
      const verificationKey = verifiedAuditOpeningKey(opening);
      if (verifiedAuditOpeningsRef.current.has(verificationKey)) continue;
      const manifest = publicDeckManifestForOwner(opening.owner);
      if (!manifest) {
        throw new Error(`Missing deck audit manifest for player ${Number(opening.owner) + 1}`);
      }
      const valid = await verifyCardOpeningAgainstManifest({
        manifest,
        slot: opening.slot,
        card: opening.card,
        salt: opening.salt,
      });
      if (!valid) {
        throw new Error(
          `Card opening for player ${Number(opening.owner) + 1}, slot ${Number(opening.slot)} does not match its deck commitment`
        );
      }
      await verifyZiffleOpeningProofForOpening(opening, options);
      verifiedAuditOpeningsRef.current.add(verificationKey);
    }
  }, [currentAuditMatchId, publicDeckManifestForOwner]);

  const buildDeckSlotOpeningForExport = useCallback(async ({
    manifest,
    preferredSlot,
    card,
    exportedCommitment = "",
    label = "private deck opening",
  }) => {
    const normalizedCommitment = String(exportedCommitment || "");
    let preferredOpening = null;
    try {
      preferredOpening = await buildDeckSlotOpening({
        manifest,
        slot: preferredSlot,
        card,
      });
    } catch (err) {
      if (!normalizedCommitment) throw err;
    }
    if (
      preferredOpening
      && (!normalizedCommitment || preferredOpening.commitment === normalizedCommitment)
    ) {
      return {
        opening: preferredOpening,
        remappedFromSlot: null,
      };
    }

    if (normalizedCommitment) {
      const committedSecret = (manifest?.slotSecrets || []).find(
        (secret) => String(secret?.commitment || "") === normalizedCommitment
      );
      if (committedSecret) {
        const committedSlot = Number(committedSecret.slot);
        return {
          opening: await buildDeckSlotOpening({
            manifest,
            slot: committedSlot,
            card: committedSecret.card,
          }),
          remappedFromSlot:
            Number(preferredSlot) === committedSlot ? null : Number(preferredSlot),
        };
      }
    }

    for (const secret of manifest?.slotSecrets || []) {
      const candidateSlot = Number(secret?.slot);
      if (!Number.isInteger(candidateSlot) || candidateSlot === Number(preferredSlot)) {
        continue;
      }
      try {
        const candidate = await buildDeckSlotOpening({
          manifest,
          slot: candidateSlot,
          card,
        });
        if (candidate.commitment === normalizedCommitment) {
          return {
            opening: candidate,
            remappedFromSlot: Number(preferredSlot),
          };
        }
      } catch {
        // Keep searching; a different card name cannot satisfy this slot commitment.
      }
    }

    throw new Error(
      `${label} does not match slot ${Number(preferredSlot)}`
      + ` (owner ${Number(manifest?.owner) + 1}, match ${String(manifest?.matchId || "")},`
      + ` exported ${normalizedCommitment.slice(0, 12) || "none"},`
      + ` local ${preferredOpening?.commitment?.slice(0, 12) || "none"})`
    );
  }, []);

	  const currentHiddenCardMetadataForObject = useCallback(async (objectId) => {
	    const normalized = Number(objectId);
	    if (!Number.isSafeInteger(normalized) || normalized < 0) return null;
	    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.exportSyncCheckpoint !== "function") {
      return null;
    }
	    let checkpoint = null;
	    try {
	      checkpoint = await currentGame.exportSyncCheckpoint();
	    } catch {
	      return null;
	    }
	    return hiddenCardMetadataForObjectFromCheckpoint(checkpoint, normalized);
	  }, []);

		  const sanitizeObjectBoundOpening = useCallback(async (opening) => {
		    if (!opening || typeof opening !== "object") return opening;
		    const normalizedObjectId = Number(opening.objectId ?? opening.object_id);
			    const ziffleCommitment = String(
			      opening.positionCommitment
		      || opening.position_commitment
		      || opening.publicCommitment
		      || opening.public_commitment
		      || (ziffleDeckHashFromCommitment(opening.commitment) ? opening.commitment : "")
			      || ""
			    );
		    const zifflePosition =
		      zifflePositionFromCommitment(ziffleCommitment)
		      ?? (opening.position != null ? Number(opening.position) : null);
		    if (
		      Number.isSafeInteger(zifflePosition)
		      && zifflePosition >= 0
		      && ziffleDeckHashFromCommitment(ziffleCommitment)
	    ) {
	      const candidates = ziffleCeremonyCandidatesForOwner(opening.owner, {
	        commitment: ziffleCommitment,
	        context: ziffleContextFromOpening(opening),
	      });
      const linkedCeremony = candidates.find((entry) =>
        ziffleObjectOrderLinksOpening(entry, opening.slot, zifflePosition, opening)
      );
      const ceremony = linkedCeremony
        || candidates.find((entry) => ziffleCeremonyHasObjectOrder(entry))
        || candidates[0];
      const existingShuffleObjectId = openingShuffleSourceId(opening);
      const proof = opening?.ziffleReveal || opening?.ziffleProof || opening?.positionOpeningProof || {};
      const proofShuffleObjectId = openingShuffleSourceId(proof);
      const beforeOrder = normalizeShuffleOrder(ceremony?.beforeOrder ?? ceremony?.before_order);
      const proofShuffleOriginalSlot = Number(
        proof?.shuffleOriginalSlot
        ?? proof?.shuffle_original_slot
        ?? opening?.shuffleOriginalSlot
        ?? opening?.shuffle_original_slot
        ?? opening?._debugShuffleOriginalSlot
        ?? proof?.originalSlot
      );
      const orderedSourceObjectId =
        Number.isSafeInteger(proofShuffleOriginalSlot) && proofShuffleOriginalSlot >= 0
          ? Number(beforeOrder[proofShuffleOriginalSlot])
          : NaN;
      const normalizedOrderedSourceObjectId =
        Number.isSafeInteger(orderedSourceObjectId) && orderedSourceObjectId >= 0
          ? orderedSourceObjectId
          : null;
      const orderedShuffleObjectId = ziffleShuffleObjectIdForPosition(ceremony, zifflePosition);
      const objectIdMatchesOrderedPosition = Boolean(
        Number.isSafeInteger(normalizedObjectId)
        && normalizedObjectId >= 0
        && orderedShuffleObjectId != null
        && normalizedObjectId === orderedShuffleObjectId
      );
      const ceremonyHasCommittedObjectOrder = Boolean(
        ziffleDeckHashFromCommitment(ziffleCommitment)
        && ziffleCeremonyHasObjectOrder(ceremony)
      );
	      const canInferShuffleObjectId = Boolean(
	        orderedShuffleObjectId != null
	        && (
	          linkedCeremony
	          || objectIdMatchesOrderedPosition
          || ceremonyHasCommittedObjectOrder
	          || ziffleContextFromOpening(opening)
	          || candidates.length <= 1
	        )
	      );
      const shuffleObjectId =
        proofShuffleObjectId
        ?? normalizedOrderedSourceObjectId
        ?? existingShuffleObjectId
        ?? (canInferShuffleObjectId ? orderedShuffleObjectId : null);
	      return {
	        ...opening,
	        position: zifflePosition,
	        positionCommitment: ziffleCommitment,
	        ...(Number.isSafeInteger(normalizedObjectId) && normalizedObjectId >= 0
	          ? { objectId: normalizedObjectId }
	          : orderedShuffleObjectId != null
	            ? { objectId: orderedShuffleObjectId }
            : {}),
        ...(shuffleObjectId != null ? { shuffleObjectId } : {}),
        ...(ceremony?.context ? { ziffleContext: ziffleContextFromCeremony(ceremony) } : {}),
      };
	    }
	    if (!Number.isSafeInteger(normalizedObjectId) || normalizedObjectId < 0) {
	      return opening;
	    }

	    const metadata = await currentHiddenCardMetadataForObject(normalizedObjectId);
	    const openingCommitment = String(opening.commitment || "");
	    const metadataCommitment = String(metadata?.commitment || "");
	    const metadataPublicCommitment = String(metadata?.publicCommitment || "");
	    const hiddenObjectStillMatches = Boolean(
	      metadata
	      && Number(metadata.owner) === Number(opening.owner)
	      && Number(metadata.slot) === Number(opening.slot)
	      && openingCommitment
	      && (
	        metadataCommitment === openingCommitment
	        || metadataPublicCommitment === openingCommitment
	      )
	    );
		    if (hiddenObjectStillMatches) {
		      return {
		        ...opening,
		        objectId: normalizedObjectId,
		      };
		    }
		    const hiddenObjectCanInferZifflePosition = Boolean(
		      metadata
		      && Number(metadata.owner) === Number(opening.owner)
		      && openingCommitment
		      && !ziffleDeckHashFromCommitment(openingCommitment)
		      && ziffleDeckHashFromCommitment(metadataCommitment)
		      && metadata.slot != null
		    );
		    if (hiddenObjectCanInferZifflePosition) {
		      return {
		        ...opening,
		        objectId: normalizedObjectId,
		      };
		    }

		    const currentGame = gameRef.current;
	    if (currentGame && typeof currentGame.exportSyncCheckpoint === "function") {
	      try {
	        const checkpoint = await currentGame.exportSyncCheckpoint();
	        const explicitObject = checkpointObjectForId(checkpoint, normalizedObjectId);
	        if (knownCheckpointObjectMatchesOpening(explicitObject, opening)) {
	          return {
	            ...opening,
	            objectId: normalizedObjectId,
	          };
	        }
	      } catch {
	        // Fall through to the safe slot/commitment opening below.
	      }
	    }

	    const objectCacheKey = `${currentAuditMatchId()}:object:${normalizedObjectId}`;
	    localRevealedOpeningsRef.current.delete(objectCacheKey);
	    removeStoredRevealedOpening(objectCacheKey);

	    const sanitized = { ...opening };
	    delete sanitized.objectId;
	    delete sanitized.object_id;
	    delete sanitized.shuffleObjectId;
	    delete sanitized.shuffle_object_id;
	    return sanitized;
	  }, [currentAuditMatchId, currentHiddenCardMetadataForObject, ziffleCeremonyCandidatesForOwner]);

	  const resolveCommittedZiffleRevealSlot = useCallback(async ({
    owner,
    ceremony,
    shuffleOriginalSlot,
    shuffleOriginalSlotIsVerified = false,
    position,
    card = "",
    objectId = null,
    manifest: providedManifest = null,
    payload = null,
    options = {},
  } = {}) => {
    const normalizedOwner = Number(owner);
    const normalizedPosition = Number(position);
    const parsedShuffleOriginalSlot = Number(shuffleOriginalSlot);
    const normalizedShuffleOriginalSlot =
      Number.isSafeInteger(parsedShuffleOriginalSlot) && parsedShuffleOriginalSlot >= 0
        ? parsedShuffleOriginalSlot
        : null;
    const expectedCard = String(card || "");
    const manifest = providedManifest || privateDeckManifestForOwner(
      normalizedOwner,
      payload?.auditMatchId,
    );
    const beforeOrder = normalizeShuffleOrder(ceremony?.beforeOrder ?? ceremony?.before_order);
    const afterOrder = normalizeShuffleOrder(ceremony?.afterOrder ?? ceremony?.after_order);
    const shuffleObjectId =
      normalizedShuffleOriginalSlot == null ? NaN : Number(beforeOrder[normalizedShuffleOriginalSlot]);
    const positionObjectId = Number(afterOrder[normalizedPosition]);
    const objectIds = [
      objectId,
      shuffleObjectId,
      positionObjectId,
    ].filter((entry, index, list) =>
      Number.isSafeInteger(Number(entry))
      && Number(entry) >= 0
      && list.findIndex((candidate) => Number(candidate) === Number(entry)) === index
    ).map((entry) => Number(entry));
    const expectedPositionCommitment =
      ceremony?.deckHash && Number.isSafeInteger(normalizedPosition) && normalizedPosition >= 0
        ? ziffleRuntimeCommitment(ceremony.deckHash, normalizedPosition)
        : "";
    const slotSecret = (slot) => (manifest?.slotSecrets || []).find(
      (secret) => Number(secret?.slot) === Number(slot)
    ) || null;
    const cardMatches = (secret, candidateCard = expectedCard) =>
      Boolean(secret)
      && (!candidateCard || String(secret.card || "") === String(candidateCard || ""));
    const fromSlot = (slot, source, details = {}) => {
      const normalizedSlot = Number(slot);
      if (!Number.isSafeInteger(normalizedSlot) || normalizedSlot < 0) return null;
      const secret = slotSecret(normalizedSlot);
      if (!cardMatches(secret)) return null;
      const resolvedObjectId = details.objectId == null ? null : Number(details.objectId);
      const resolvedShuffleObjectId = Number(details.shuffleObjectId ?? shuffleObjectId);
      return {
        owner: normalizedOwner,
        slot: normalizedSlot,
        card: String(secret.card || expectedCard || ""),
        commitment: String(secret.commitment || ""),
        objectId: resolvedObjectId != null && Number.isSafeInteger(resolvedObjectId) && resolvedObjectId >= 0
          ? resolvedObjectId
          : null,
        shuffleObjectId:
          Number.isSafeInteger(resolvedShuffleObjectId) && resolvedShuffleObjectId >= 0
            ? resolvedShuffleObjectId
            : null,
        position: Number.isSafeInteger(normalizedPosition) ? normalizedPosition : null,
        positionCommitment: expectedPositionCommitment,
        shuffleOriginalSlot: Number.isSafeInteger(normalizedShuffleOriginalSlot)
          ? normalizedShuffleOriginalSlot
          : null,
        positionLinked: details.positionLinked === true,
        source,
      };
    };
    const ceremonyHasObjectOrder = beforeOrder.length > 0 || afterOrder.length > 0;
    const revealLinksSlot = (candidate) =>
      Boolean(candidate)
      && (
        (
          candidate.positionLinked === true
          && expectedPositionCommitment
          && String(candidate.positionCommitment || "") === expectedPositionCommitment
          && Number(candidate.position) === normalizedPosition
        )
        || (
          !ceremonyHasObjectOrder
          && normalizedShuffleOriginalSlot != null
          && Number(candidate.slot) === normalizedShuffleOriginalSlot
        )
        || (
          normalizedShuffleOriginalSlot != null
          && ziffleObjectOrderLinksOpening(
            ceremony,
            normalizedShuffleOriginalSlot,
            normalizedPosition,
            candidate
          )
        )
      );
    const metadataExplicitlyLinksCurrentPosition = (metadata) => {
      if (!metadata) return false;
      const publicSlotRaw = metadata.publicSlot ?? metadata.public_slot ?? null;
      const publicSlot = publicSlotRaw == null ? null : Number(publicSlotRaw);
      const publicCommitment = String(
        metadata.publicCommitment ?? metadata.public_commitment ?? ""
      );
      const publicDeckHash = ziffleDeckHashFromCommitment(publicCommitment);
      if (publicSlot == null && !publicDeckHash) return false;
      if (publicSlot != null && publicSlot !== normalizedPosition) return false;
      if (publicDeckHash) {
        const expectedDeckHash = ziffleDeckHashFromCommitment(expectedPositionCommitment);
        if (expectedDeckHash && publicDeckHash !== expectedDeckHash) return false;
        if (expectedPositionCommitment && publicCommitment !== expectedPositionCommitment) return false;
        const committedPosition = zifflePositionFromCommitment(publicCommitment);
        if (committedPosition != null && committedPosition !== normalizedPosition) return false;
      }
      return true;
    };
    const verifiedRevealSlot = (() => {
      if (!shuffleOriginalSlotIsVerified || normalizedShuffleOriginalSlot == null) {
        return null;
      }
      const strict = fromSlot(normalizedShuffleOriginalSlot, "verified_ziffle_reveal_slot", {
        shuffleObjectId,
        positionLinked: true,
      });
      if (strict) return strict;
      const secret = slotSecret(normalizedShuffleOriginalSlot);
      if (!secret) return null;
      return {
        owner: normalizedOwner,
        slot: normalizedShuffleOriginalSlot,
        card: String(secret.card || ""),
        commitment: String(secret.commitment || ""),
        objectId: null,
        shuffleObjectId: Number.isSafeInteger(shuffleObjectId) && shuffleObjectId >= 0
          ? shuffleObjectId
          : null,
        position: Number.isSafeInteger(normalizedPosition) ? normalizedPosition : null,
        positionCommitment: expectedPositionCommitment,
        shuffleOriginalSlot: normalizedShuffleOriginalSlot,
        positionLinked: true,
        source: "verified_ziffle_reveal_slot_card_mismatch",
      };
    })();
    if (verifiedRevealSlot) {
      return verifiedRevealSlot;
    }
    const currentGame = gameRef.current;
	    const resolveMetadataSlot = async (
	      metadata,
	      source,
	      details = {},
      depth = 0,
      visited = new Set()
	    ) => {
	      if (!metadata || Number(metadata.owner) !== normalizedOwner) return null;
	      if (!hiddenMetadataMatchesZifflePosition(
	        metadata,
	        normalizedPosition,
	        expectedPositionCommitment
	      )) {
	        return null;
	      }
	      const metadataSlot = Number(metadata.slot);
	      if (!Number.isSafeInteger(metadataSlot) || metadataSlot < 0) return null;
      const positionLinked =
        details.positionLinked === true
        || metadataExplicitlyLinksCurrentPosition(metadata);
      const metadataCommitment = String(metadata.commitment || "");
      const metadataIsZifflePosition = Boolean(ziffleDeckHashFromCommitment(metadataCommitment));
      const metadataObjectId = Number(details.objectId ?? metadata.objectId);
      const metadataShuffleObjectId = Number(details.shuffleObjectId ?? shuffleObjectId);

      if (!metadataIsZifflePosition) {
        const resolved = fromSlot(metadataSlot, source, {
          objectId: Number.isSafeInteger(metadataObjectId) && metadataObjectId >= 0
            ? metadataObjectId
            : null,
          shuffleObjectId: Number.isSafeInteger(metadataShuffleObjectId) && metadataShuffleObjectId >= 0
            ? metadataShuffleObjectId
            : null,
          positionLinked,
        });
        if (resolved && revealLinksSlot(resolved)) {
          return {
            ...resolved,
            hiddenMetadata: metadata,
          };
        }
        return null;
      }

      if (depth >= 8 || !currentGame || typeof currentGame.ziffleRevealCard !== "function") {
        return null;
      }
      const nestedCeremony = ziffleCeremonyForOwner(normalizedOwner, {
        commitment: metadataCommitment,
        payload,
      });
      if (!nestedCeremony?.deckHash) return null;
      const nestedPosition = metadataSlot;
      const visitKey = [
        String(nestedCeremony.context || ""),
        String(nestedCeremony.deckHash || ""),
        Number(nestedPosition),
      ].join(":");
      if (visited.has(visitKey)) return null;
      visited.add(visitKey);

      const tokens = await collectZiffleRevealTokens(nestedCeremony, nestedPosition, options);
      const nestedReveal = await currentGame.ziffleRevealCard({
        deckCount: Number(nestedCeremony.deckCount),
        context: String(nestedCeremony.context || ""),
        keyContext: ziffleKeyContextForCeremony(nestedCeremony),
        keys: cloneMultiplayerPayload(nestedCeremony.keys || []),
        steps: cloneMultiplayerPayload(nestedCeremony.steps || []),
        cardPosition: nestedPosition,
        tokens,
      });
	      const nestedShuffleOriginalSlot = Number(nestedReveal.originalSlot);
	      if (!Number.isSafeInteger(nestedShuffleOriginalSlot) || nestedShuffleOriginalSlot < 0) {
	        return null;
	      }
	      const directNested = fromSlot(nestedShuffleOriginalSlot, `${source}:nested_ziffle`, {
	        objectId: Number.isSafeInteger(metadataObjectId) && metadataObjectId >= 0
	          ? metadataObjectId
	          : null,
	        shuffleObjectId: Number.isSafeInteger(metadataShuffleObjectId) && metadataShuffleObjectId >= 0
	          ? metadataShuffleObjectId
	          : null,
          positionLinked,
	      });
	      if (directNested && revealLinksSlot(directNested)) {
	        return {
	          ...directNested,
	          hiddenMetadata: metadata,
	        };
	      }
	      const nestedBeforeOrder = normalizeShuffleOrder(
	        nestedCeremony.beforeOrder ?? nestedCeremony.before_order
	      );
      const nestedAfterOrder = normalizeShuffleOrder(
        nestedCeremony.afterOrder ?? nestedCeremony.after_order
      );
      if (nestedBeforeOrder.length === 0 && nestedAfterOrder.length === 0) {
        const resolved = fromSlot(nestedShuffleOriginalSlot, source, {
          objectId: Number.isSafeInteger(metadataObjectId) && metadataObjectId >= 0
            ? metadataObjectId
            : null,
          shuffleObjectId: Number.isSafeInteger(metadataShuffleObjectId) && metadataShuffleObjectId >= 0
            ? metadataShuffleObjectId
            : null,
          positionLinked,
        });
        if (resolved && revealLinksSlot(resolved)) {
          return {
            ...resolved,
            hiddenMetadata: metadata,
          };
        }
        return null;
      }

      const nestedShuffleObjectId = Number(nestedBeforeOrder[nestedShuffleOriginalSlot]);
      const nestedPositionObjectId = Number(nestedAfterOrder[nestedPosition]);
      const nestedObjectIds = [
        metadataObjectId,
        nestedShuffleObjectId,
        nestedPositionObjectId,
      ].filter((entry, index, list) =>
        Number.isSafeInteger(Number(entry))
        && Number(entry) >= 0
        && list.findIndex((candidate) => Number(candidate) === Number(entry)) === index
      ).map((entry) => Number(entry));
      for (const nestedObjectId of nestedObjectIds) {
        const nestedMetadata = await currentHiddenCardMetadataForObject(nestedObjectId);
        const resolved = await resolveMetadataSlot(
          nestedMetadata,
          source,
          {
            objectId: nestedObjectId,
            shuffleObjectId: Number.isSafeInteger(metadataShuffleObjectId) && metadataShuffleObjectId >= 0
              ? metadataShuffleObjectId
              : nestedObjectId,
            positionLinked,
          },
          depth + 1,
          visited
        );
        if (resolved) return resolved;
      }
      return null;
    };
    const linkedOpening = localRevealedOpeningForZiffleReveal({
      owner: normalizedOwner,
      ceremony,
      shuffleOriginalSlot: normalizedShuffleOriginalSlot,
      position: normalizedPosition,
      card: expectedCard,
      objectId,
    });
    if (linkedOpening) {
      const linked = fromSlot(linkedOpening.slot, "cached_opening", {
        objectId: linkedOpening.objectId ?? objectId,
        shuffleObjectId: linkedOpening.shuffleObjectId
          ?? linkedOpening.shuffle_object_id
          ?? linkedOpening.objectId
          ?? shuffleObjectId,
      });
      if (linked && revealLinksSlot(linked)) {
        return {
          ...linked,
          card: String(linkedOpening.card || linked.card || ""),
          commitment: String(linkedOpening.commitment || linked.commitment || ""),
          positionCommitment: String(linkedOpening.positionCommitment || linked.positionCommitment || ""),
          linkedOpening,
        };
      }
    }
    for (const candidateObjectId of objectIds) {
      const metadata = await currentHiddenCardMetadataForObject(candidateObjectId);
      if (!metadata || Number(metadata.owner) !== normalizedOwner) continue;
      const metadataSlot = Number(metadata.slot);
      const metadataCommitment = String(metadata.commitment || "");
      const metadataCommitmentIsZifflePosition = Boolean(
        ziffleDeckHashFromCommitment(metadataCommitment)
      );
      const secret = metadataCommitmentIsZifflePosition ? null : slotSecret(metadataSlot);
      if (!metadataCommitmentIsZifflePosition && !cardMatches(secret)) continue;
      const metadataPublicSlot = metadata.publicSlot == null
        ? null
        : Number(metadata.publicSlot);
      const metadataPublicCommitment = String(metadata.publicCommitment || "");
      const metadataHasPublicPosition =
        metadataPublicSlot != null
        || Boolean(metadataPublicCommitment);
      const metadataCommitmentIsOriginal =
        Boolean(secret?.commitment)
        && metadataCommitment === String(secret.commitment || "");
      const metadataLinksPosition =
        metadataPublicSlot === normalizedPosition
        || metadataPublicCommitment === expectedPositionCommitment
        || (
          !metadataHasPublicPosition
          && (
            candidateObjectId === positionObjectId
            || candidateObjectId === shuffleObjectId
          )
        );
      if (
        metadataCommitmentIsOriginal
        || metadataLinksPosition
        || !ziffleDeckHashFromCommitment(metadataCommitment)
      ) {
        const metadataResolved = await resolveMetadataSlot(metadata, "hidden_metadata", {
          objectId: candidateObjectId,
          shuffleObjectId,
        });
        if (metadataResolved) return metadataResolved;
      }
    }
    if (currentGame && typeof currentGame.exportSyncCheckpoint === "function") {
      let checkpoint = null;
      try {
        checkpoint = await currentGame.exportSyncCheckpoint();
      } catch {
        checkpoint = null;
      }
      const checkpointObjectId = hiddenObjectIdForOpeningFromCheckpoint(checkpoint, {
        owner: normalizedOwner,
        position: normalizedPosition,
        positionCommitment: expectedPositionCommitment,
      });
      const checkpointMetadata = checkpointObjectId == null
        ? null
        : hiddenCardMetadataForObjectFromCheckpoint(checkpoint, checkpointObjectId);
      if (checkpointMetadata && Number(checkpointMetadata.owner) === normalizedOwner) {
        const checkpointResolved = await resolveMetadataSlot(
          checkpointMetadata,
          "checkpoint_position_metadata",
          {
            objectId: checkpointObjectId,
            shuffleObjectId,
          }
        );
        if (checkpointResolved) return checkpointResolved;
      }
    }
    const directSlot = normalizedShuffleOriginalSlot == null
      ? null
      : fromSlot(normalizedShuffleOriginalSlot, "shuffle_original_slot", {
        objectId:
          Number.isSafeInteger(positionObjectId)
          && positionObjectId >= 0
          && Number(positionObjectId) === Number(shuffleObjectId)
            ? positionObjectId
            : null,
        shuffleObjectId,
      });
    if (!ceremonyHasObjectOrder && directSlot && revealLinksSlot(directSlot)) return directSlot;
    if (
      !ceremonyHasObjectOrder
      && expectedCard
      && Number.isSafeInteger(shuffleObjectId)
      && shuffleObjectId >= 0
      && Number(positionObjectId) === shuffleObjectId
    ) {
      const firstMatchingSecret = (manifest?.slotSecrets || []).find((secret) =>
        String(secret?.card || "") === expectedCard
      );
      if (firstMatchingSecret) {
        const fallback = fromSlot(firstMatchingSecret.slot, "card_name_fallback", {
          objectId:
            Number.isSafeInteger(positionObjectId)
            && positionObjectId >= 0
            && Number(positionObjectId) === Number(shuffleObjectId)
              ? positionObjectId
              : null,
          shuffleObjectId,
        });
        if (revealLinksSlot(fallback)) {
          return fallback;
        }
      }
    }
    return null;
  }, [
	    currentHiddenCardMetadataForObject,
	    localRevealedOpeningForZiffleReveal,
	    privateDeckManifestForOwner,
      ziffleCeremonyForOwner,
	  ]);

  async function resolveCommittedSlotForZifflePosition({
    owner,
    ceremony,
    position,
    objectId = null,
    card = "",
    manifest = null,
    payload = null,
    options = {},
  } = {}) {
    const normalizedPosition = Number(position);
    const normalizedObjectId = Number(objectId);
    const shuffleOriginalSlot = ziffleShuffleOriginalSlotForPosition(
      ceremony,
      normalizedPosition,
      normalizedObjectId
    );
    const resolvedShuffleOriginalSlot =
      Number.isSafeInteger(shuffleOriginalSlot) && shuffleOriginalSlot >= 0
        ? shuffleOriginalSlot
        : null;
    const tryResolveForShuffleSlot = async (
      candidateShuffleOriginalSlot,
      shuffleOriginalSlotIsVerified = false
    ) => {
      const baseArgs = {
        owner,
        ceremony,
        shuffleOriginalSlot: candidateShuffleOriginalSlot,
        shuffleOriginalSlotIsVerified,
        position: normalizedPosition,
        objectId:
          Number.isSafeInteger(normalizedObjectId) && normalizedObjectId >= 0
            ? normalizedObjectId
            : null,
        manifest,
        payload,
        options,
      };
      let resolved = await resolveCommittedZiffleRevealSlot({
        ...baseArgs,
        card,
      });
      if (!resolved && card) {
        resolved = await resolveCommittedZiffleRevealSlot({
          ...baseArgs,
          card: "",
        });
      }
      return resolved;
    };
    const objectOrderResolvedRevealSlot = await tryResolveForShuffleSlot(
      resolvedShuffleOriginalSlot,
      false,
    );
    let cryptographicShuffleOriginalSlot = null;
    let cryptographicResolvedRevealSlot = null;
    const currentGame = gameRef.current;
    if (
      currentGame
      && typeof currentGame.ziffleRevealCard === "function"
      && Number.isSafeInteger(normalizedPosition)
      && normalizedPosition >= 0
      && ceremony?.deckHash
    ) {
      const tokens = await collectZiffleRevealTokens(ceremony, normalizedPosition, options);
      const reveal = await currentGame.ziffleRevealCard({
        deckCount: Number(ceremony.deckCount),
        context: String(ceremony.context || ""),
        keyContext: ziffleKeyContextForCeremony(ceremony),
        keys: cloneMultiplayerPayload(ceremony.keys || []),
        steps: cloneMultiplayerPayload(ceremony.steps || []),
        cardPosition: normalizedPosition,
        tokens,
      });
      const revealedSlot = Number(reveal?.originalSlot);
      if (Number.isSafeInteger(revealedSlot) && revealedSlot >= 0) {
        cryptographicShuffleOriginalSlot = revealedSlot;
        cryptographicResolvedRevealSlot = await tryResolveForShuffleSlot(revealedSlot, true);
      }
    }
    const ceremonyHasObjectOrder = ziffleCeremonyHasObjectOrder(ceremony);
    const cryptographicSlotMatchesObjectOrder =
      !ceremonyHasObjectOrder
      || cryptographicShuffleOriginalSlot == null
      || resolvedShuffleOriginalSlot == null
      || cryptographicShuffleOriginalSlot === resolvedShuffleOriginalSlot;
    const resolvedRevealSlot =
      (ceremonyHasObjectOrder && objectOrderResolvedRevealSlot)
        ? objectOrderResolvedRevealSlot
        : cryptographicSlotMatchesObjectOrder
          ? (cryptographicResolvedRevealSlot || objectOrderResolvedRevealSlot)
          : null;
    return {
      resolvedRevealSlot,
      shuffleOriginalSlot:
        resolvedRevealSlot === objectOrderResolvedRevealSlot
          ? resolvedShuffleOriginalSlot
          : cryptographicShuffleOriginalSlot ?? resolvedShuffleOriginalSlot,
    };
  }

  async function buildOpeningFromResolvedCommittedSlot({
    manifest,
    resolvedRevealSlot,
    fallbackObjectId = null,
    position,
    positionCommitment,
    ceremony = null,
    timing = null,
  } = {}) {
    const originalSlot = Number(resolvedRevealSlot?.slot);
    if (!Number.isSafeInteger(originalSlot) || originalSlot < 0) {
      throw new Error(`Missing private deck opening for ziffle position ${Number(position)}`);
    }
    const secret = (manifest?.slotSecrets || []).find(
      (candidate) => Number(candidate.slot) === originalSlot
    );
    if (!secret) {
      throw new Error(`Missing private deck opening for ziffle slot ${originalSlot}`);
    }
    const opening = await buildDeckSlotOpening({
      manifest,
      slot: originalSlot,
      card: resolvedRevealSlot?.card || secret.card,
    });
    const resolvedObjectId = resolvedRevealSlot?.objectId == null
      ? null
      : Number(resolvedRevealSlot.objectId);
    const normalizedFallbackObjectId = fallbackObjectId == null
      ? null
      : Number(fallbackObjectId);
    const openingObjectId =
      Number.isSafeInteger(resolvedObjectId) && resolvedObjectId >= 0
        ? resolvedObjectId
        : Number.isSafeInteger(normalizedFallbackObjectId) && normalizedFallbackObjectId >= 0
          ? normalizedFallbackObjectId
          : null;
    const resolvedShuffleObjectId = Number(
      resolvedRevealSlot?.shuffleObjectId
      ?? resolvedRevealSlot?.shuffle_object_id
      ?? resolvedRevealSlot?.objectId
    );
    const openingWithPosition = {
      ...opening,
      ...(openingObjectId != null ? { objectId: openingObjectId } : {}),
      ...(Number.isSafeInteger(resolvedShuffleObjectId) && resolvedShuffleObjectId >= 0
        ? { shuffleObjectId: resolvedShuffleObjectId }
        : {}),
      ...(timing ? { timing } : {}),
      position: Number(position),
      positionCommitment: String(positionCommitment || ""),
      ...(ceremony?.context ? { ziffleContext: ziffleContextFromCeremony(ceremony) } : {}),
    };
    return {
      opening,
      openingWithPosition,
      originalSlot,
      openingObjectId,
      secret,
    };
  }

	  async function addResolvedSelectObjectCommandIds(output, command, uiState = null) {
	    if (!output || command?.type !== "select_objects" || !Array.isArray(command.object_ids)) {
	      return output;
	    }
    const decision = uiState?.decision || null;
    const selectedIds = command.object_ids.map((objectId) => Number(objectId));
    const stableIds = commandObjectStableIds(command);
    const hiddenRefs = commandObjectHiddenRefs(command);
    for (const [index, selectedId] of selectedIds.entries()) {
      const candidate = selectObjectCandidateForId(decision, selectedId);
      if (selectObjectCandidateRevealPolicy(decision, candidate) !== "public") {
        continue;
      }
      const stableId = stableIds[index];
      if (stableId != null) {
        const localObjectId = await currentObjectIdForStableId(stableId);
        if (localObjectId != null) {
          output.add(Number(localObjectId));
        }
      }
      const hiddenRef = hiddenRefs[index];
      if (hiddenRef != null) {
        const localObjectId = await currentObjectIdForHiddenRef(hiddenRef);
        if (localObjectId != null) {
          output.add(Number(localObjectId));
        }
      }
	    }
	    return output;
	  }

	  async function localizeSelectObjectOpeningIds(output, command) {
	    if (!output || command?.type !== "select_objects" || !Array.isArray(command.object_ids)) {
	      return output;
	    }
	    const selectedIds = command.object_ids.map((objectId) => Number(objectId));
	    const stableIds = commandObjectStableIds(command);
	    const hiddenRefs = commandObjectHiddenRefs(command);
	    for (const [index, selectedId] of selectedIds.entries()) {
	      let localObjectId = null;
	      const hiddenRef = hiddenRefs[index];
	      if (hiddenRef != null) {
	        const cachedOpening = localRevealedOpeningForRequirement({
	          owner: hiddenRef.owner,
	          zone: hiddenRef.zone,
	          objectId: selectedId,
	          publicSlot: hiddenRef.publicSlot ?? hiddenRef.public_slot,
	          publicCommitment: hiddenRef.publicCommitment ?? hiddenRef.public_commitment,
	          position: hiddenRef.publicSlot ?? hiddenRef.public_slot,
	          positionCommitment: hiddenRef.publicCommitment ?? hiddenRef.public_commitment,
	          commitment: hiddenRef.publicCommitment ?? hiddenRef.public_commitment,
	        });
	        if (cachedOpening?.objectId != null) {
	          localObjectId = Number(cachedOpening.objectId);
	        }
	        if (localObjectId == null) {
	          localObjectId = await currentObjectIdForHiddenRef(hiddenRef);
	        }
	      }
	      const stableId = stableIds[index];
	      if (localObjectId == null && stableId != null) {
	        localObjectId = await currentObjectIdForStableId(stableId);
	      }
	      if (localObjectId == null) continue;
	      if (Number.isSafeInteger(selectedId) && selectedId > 0) {
	        output.delete(selectedId);
	      }
	      output.add(Number(localObjectId));
	    }
	    return output;
	  }

			  const buildLocalOpeningsForCommand = useCallback(async (command, cryptoRequirements = [], options = {}) => {
	    const currentGame = gameRef.current;
	    if (!currentGame || typeof currentGame.exportHiddenCardOpening !== "function") {
	      return [];
    }
    const commandObjectIds = collectCommandObjectIds(command, new Set(), options.uiState || stateRef.current);
    await addResolvedSelectObjectCommandIds(commandObjectIds, command, options.uiState || stateRef.current);
    const objectIds = new Set(commandObjectIds);
    const publicOpenRequirementByObjectId = new Map();
    for (const requirement of cryptoRequirements || []) {
      if (
        String(requirement?.type || "") === "public_open"
        && requirement.objectId != null
        && !requirementHasZifflePosition(requirement)
      ) {
        const objectId = Number(requirement.objectId);
        objectIds.add(objectId);
	        publicOpenRequirementByObjectId.set(objectId, requirement);
	      }
	    }
	    await localizeSelectObjectOpeningIds(objectIds, command);
	    await localizeSelectObjectOpeningIds(commandObjectIds, command);
	    const openings = [];
    const seen = new Set();
    const localSeat = resolveLocalCryptoPlayerIndex();
    for (const objectId of objectIds) {
      let exported = null;
      try {
        exported = await currentGame.exportHiddenCardOpening(wasmObjectIdArg(objectId));
      } catch (err) {
        void err;
        const requirement = publicOpenRequirementByObjectId.get(Number(objectId));
        if (
          requirement
          && Number(requirement.owner) === Number(localSeat)
          && requirement.slot != null
          && requirement.card
        ) {
          let opening = localRevealedOpeningForRequirement(requirement);
          const manifest = privateDeckManifestForOwner(requirement.owner);
          if (!manifest && !opening) {
            throw new Error(`Missing private deck opening for slot ${Number(requirement.slot)}`);
          }
          let remappedFromSlot = null;
          if (!opening) {
            const built = await buildDeckSlotOpeningForExport({
              manifest,
              preferredSlot: requirement.slot,
              card: requirement.card,
              exportedCommitment: requirement.commitment,
              label: "Local hidden card opening",
            });
            opening = built.opening;
            remappedFromSlot = built.remappedFromSlot;
          } else {
            opening = cloneMultiplayerPayload(opening);
          }
	          const key = `${Number(opening.owner)}:${Number(opening.slot)}`;
	          if (seen.has(key)) continue;
	          opening.objectId = Number(requirement.objectId);
	          opening.timing = options.timing || (commandObjectIds.has(Number(objectId)) ? "pre" : "post");
	          const hiddenMetadata = await currentHiddenCardMetadataForObject(
	            requirement.objectId ?? requirement.object_id ?? objectId
	          );
	          const publicPositionIdentity = zifflePublicPositionFromSources(
	            hiddenMetadata,
	            requirement
	          );
          if (opening && publicPositionIdentity?.useAsPosition === false) {
            opening = cloneMultiplayerPayload(opening);
            delete opening.position;
            delete opening.positionCommitment;
            delete opening.ziffleContext;
            delete opening.ziffleReveal;
            delete opening.ziffleProof;
            delete opening.positionOpeningProof;
          }
	          let positionCommitment = String(opening.positionCommitment || "");
	          let zifflePosition = opening.position ?? null;
	          let ziffleContext = ziffleContextFromOpening(opening);
	          if (publicPositionIdentity) {
	            opening = withPinnedPublicZifflePosition(opening, publicPositionIdentity);
	            if (publicPositionIdentity.useAsPosition !== false) {
	              positionCommitment = opening.positionCommitment;
	              zifflePosition = opening.position;
	              ziffleContext = "";
	            }
	          }
		          if (publicPositionIdentity?.useAsPosition !== true && zifflePosition == null) {
				            const objectOrderedPosition = zifflePositionForObjectId(
			              requirement.owner,
			              requirement.objectId ?? requirement.object_id ?? objectId
			            );
		            const originalSlotOrderedPosition = zifflePositionForOriginalSlot(
		              requirement.owner,
		              requirement.slot
		            );
			            const orderedPosition = objectOrderedPosition || originalSlotOrderedPosition;
            ziffleContext = ziffleContext || orderedPosition?.ziffleContext || "";
	            const publicPositionCommitment = String(
	              hiddenMetadata?.publicCommitment
	              || hiddenMetadata?.public_commitment
	              || orderedPosition?.positionCommitment
	              || ""
	            );
	            const hiddenPositionCommitment =
	              hiddenMetadata?.commitment && ziffleDeckHashFromCommitment(hiddenMetadata.commitment)
	                ? String(hiddenMetadata.commitment)
	                : "";
            if (
              publicPositionIdentity?.useAsPosition !== false
              && publicPositionCommitment
	              && ziffleDeckHashFromCommitment(publicPositionCommitment)
	              && (hiddenMetadata?.publicSlot != null || orderedPosition?.position != null)
	            ) {
	              zifflePosition = Number(hiddenMetadata?.publicSlot ?? orderedPosition.position);
	              positionCommitment = positionCommitment || publicPositionCommitment;
	            } else if (hiddenPositionCommitment && hiddenMetadata?.slot != null) {
              zifflePosition = Number(hiddenMetadata.slot);
              positionCommitment = positionCommitment || hiddenPositionCommitment;
            }
          }
          if (zifflePosition != null) {
            opening.position = Number(zifflePosition);
            const ceremony = ziffleCeremonyForOwner(opening.owner, {
              commitment: positionCommitment,
              context: ziffleContext,
            });
            if (ceremony?.deckHash && !positionCommitment) {
              positionCommitment = ziffleRuntimeCommitment(ceremony.deckHash, zifflePosition);
            }
            if (positionCommitment) {
              opening.positionCommitment = positionCommitment;
            }
            if (ceremony?.context) {
              opening.ziffleContext = ziffleContextFromCeremony(ceremony);
              ziffleContext = opening.ziffleContext;
            }
          }
          if (
            zifflePosition != null
            && opening.positionCommitment
            && ziffleDeckHashFromCommitment(opening.positionCommitment)
            && !opening.ziffleReveal
            && !opening.ziffleProof
            && !opening.positionOpeningProof
          ) {
            const ceremony = ziffleCeremonyForOwner(opening.owner, {
              commitment: opening.positionCommitment,
              context: ziffleContext,
            });
            if (ceremony?.deckHash && typeof currentGame.ziffleRevealCard === "function") {
              const tokens = await collectZiffleRevealTokens(ceremony, Number(zifflePosition), options);
              const reveal = await currentGame.ziffleRevealCard({
                deckCount: Number(ceremony.deckCount),
                context: String(ceremony.context || ""),
                keyContext: ziffleKeyContextForCeremony(ceremony),
                keys: cloneMultiplayerPayload(ceremony.keys || []),
                steps: cloneMultiplayerPayload(ceremony.steps || []),
                cardPosition: Number(zifflePosition),
                tokens,
              });
              const shuffleOriginalSlot = Number(reveal.originalSlot);
              const resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
                owner: opening.owner,
                ceremony,
                shuffleOriginalSlot,
                shuffleOriginalSlotIsVerified: true,
                position: Number(zifflePosition),
                card: opening.card || requirement.card || "",
                objectId: requirement.objectId ?? requirement.object_id ?? objectId,
                manifest,
                options,
              });
              if (resolvedRevealSlot) {
                if (Number(resolvedRevealSlot.slot) !== Number(opening.slot)) {
                  const rebuilt = await buildDeckSlotOpeningForExport({
                    manifest,
                    preferredSlot: resolvedRevealSlot.slot,
                    card: resolvedRevealSlot.card || opening.card,
                    exportedCommitment: "",
                    label: "Local hidden card opening",
                  });
                  opening = rebuilt.opening;
                }
                opening.position = Number(zifflePosition);
                opening.positionCommitment = opening.positionCommitment
                  || ziffleRuntimeCommitment(ceremony.deckHash, zifflePosition);
                opening.ziffleContext = ziffleContextFromCeremony(ceremony);
                ziffleContext = opening.ziffleContext;
                if (resolvedRevealSlot.shuffleObjectId != null || resolvedRevealSlot.objectId != null) {
                  opening.shuffleObjectId = Number(
                    resolvedRevealSlot.shuffleObjectId ?? resolvedRevealSlot.objectId
                  );
                }
                rememberZiffleOpeningPosition(opening.owner, opening.slot, Number(zifflePosition));
	                if (!ziffleCeremonyHasObjectOrder(ceremony)) {
	                  opening.ziffleReveal = buildZiffleOpeningProof({
                    opening,
                    ceremony,
                    position: Number(zifflePosition),
                    originalSlot: Number(opening.slot),
                    shuffleOriginalSlot,
                    positionCommitment: opening.positionCommitment,
                    tokens,
	                    compact: true,
	                  });
	                }
	              } else {
	                throw new Error(
	                  `Ziffle opening could not resolve committed slot `
	                  + `(owner ${Number(opening.owner) + 1}, position ${Number(zifflePosition)}, `
	                  + `shuffle slot ${shuffleOriginalSlot}, card ${String(opening.card || requirement.card || "")})`
	                );
	              }
	            }
	          }
	          if (remappedFromSlot != null) {
	            opening.reportedSlot = Number(remappedFromSlot);
	          }
	          opening = await ensureZiffleOpeningProof(opening, options);
	          opening = await sanitizeObjectBoundOpening(opening);
	          rememberLocalRevealedOpening(opening, {
	            objectId: opening.objectId,
	            position: opening.position,
	            positionCommitment: opening.positionCommitment,
	            ziffleContext,
	          });
          openings.push(opening);
          notifyOpeningBuilt(options, opening, {
            source: "command_public_open",
            index: openings.length,
            total: objectIds.size,
          });
          seen.add(key);
        }
        continue;
      }
      if (!exported || exported.owner == null || exported.slot == null || !exported.card) {
        continue;
      }
      const key = `${Number(exported.owner)}:${Number(exported.slot)}`;
      if (seen.has(key)) continue;
      if (Number(exported.owner) !== Number(localSeat)) {
        continue;
      }
      let cachedOpening = localRevealedOpeningForExport(exported);
      const manifest = privateDeckManifestForOwner(exported.owner);
      if (!manifest && !cachedOpening) {
        throw new Error(`Missing private deck opening for slot ${Number(exported.slot)}`);
      }
	      const hiddenMetadata = await currentHiddenCardMetadataForObject(
	        exported.object_id ?? exported.objectId ?? objectId
	      );
	      const exportedCommitment = String(exported.commitment || "");
	      const exportedCommitmentIsZiffle = Boolean(
	        ziffleDeckHashFromCommitment(exportedCommitment)
	      );
		      const objectOrderedPosition = zifflePositionForObjectId(
	        exported.owner,
	        exported.object_id ?? exported.objectId ?? objectId
	      );
	      const originalSlotOrderedPosition = zifflePositionForOriginalSlot(exported.owner, exported.slot);
	      const orderedPosition = objectOrderedPosition || originalSlotOrderedPosition;
	      let ziffleContext = orderedPosition?.ziffleContext || "";
	      let ziffleContextCommitment = orderedPosition?.positionCommitment || "";
		      const exportedPublicSlot = exported?.publicSlot ?? exported?.public_slot ?? null;
		      const exportedPublicCommitment = String(
		        exported?.publicCommitment || exported?.public_commitment || ""
		      );
		      const publicPositionIdentity = zifflePublicPositionFromSources(
		        hiddenMetadata,
		        {
		          publicSlot: exportedPublicSlot,
		          publicCommitment: exportedPublicCommitment,
		        }
		      );
      if (cachedOpening && publicPositionIdentity?.useAsPosition === false) {
        cachedOpening = cloneMultiplayerPayload(cachedOpening);
        delete cachedOpening.position;
        delete cachedOpening.positionCommitment;
        delete cachedOpening.ziffleContext;
        delete cachedOpening.ziffleReveal;
        delete cachedOpening.ziffleProof;
        delete cachedOpening.positionOpeningProof;
      }
		      const publicPosition = publicPositionIdentity?.useAsPosition === false
		        ? null
		        : publicPositionIdentity?.position ?? orderedPosition?.position ?? null;
	      const publicPositionCommitment =
	        publicPositionIdentity?.useAsPosition === false
	          ? ""
	          : publicPositionIdentity?.useAsPosition !== false
	          ? publicPositionIdentity?.positionCommitment
	            || (publicPosition != null ? orderedPosition?.positionCommitment || "" : "")
	          : "";
      const hiddenPositionCommitment =
        hiddenMetadata?.commitment && ziffleDeckHashFromCommitment(hiddenMetadata.commitment)
          ? String(hiddenMetadata.commitment)
          : "";
      const identityPosition = ziffleIdentityPositionFromSources(
        hiddenMetadata,
        exported
      );
      const currentZifflePosition =
        publicPosition
        ?? identityPosition?.position
        ?? (hiddenPositionCommitment ? Number(hiddenMetadata?.slot) : null);
      const currentZifflePositionCommitment =
        publicPositionCommitment || identityPosition?.positionCommitment || hiddenPositionCommitment;
      const mustUseZiffleOpening = Boolean(
        currentZifflePosition != null
        && ziffleDeckHashFromCommitment(currentZifflePositionCommitment)
      );
		      if (
		        cachedOpening
		        && currentZifflePositionCommitment
		        && !cachedOpeningMatchesZifflePosition(
		          cachedOpening,
		          currentZifflePosition,
		          currentZifflePositionCommitment
		        )
		      ) {
		        cachedOpening = null;
		      }
      if (
        cachedOpening
        && !currentZifflePositionCommitment
        && !exportedCommitmentIsZiffle
        && !openingHasZifflePosition(cachedOpening)
      ) {
        cachedOpening = cloneMultiplayerPayload(cachedOpening);
        delete cachedOpening.position;
        delete cachedOpening.positionCommitment;
        delete cachedOpening.ziffleReveal;
        delete cachedOpening.ziffleProof;
        delete cachedOpening.positionOpeningProof;
      }
	      if (
	        cachedOpening
	        && currentZifflePosition != null
        && cachedOpening.position != null
        && Number(cachedOpening.position) !== Number(currentZifflePosition)
      ) {
        cachedOpening = null;
	      }
	      let remappedFromSlot = null;
	      let opening = cachedOpening;
      if (!opening) {
        let preferredSlot = Number(exported.slot);
        let openingCard = String(exported.card || "");
        let position = null;
        let positionCommitment = "";
	      let ziffleProofCeremony = null;
	      let ziffleProofTokens = null;
		      let ziffleProofShuffleOriginalSlot = null;
		      let ziffleProofShuffleObjectId = null;
	        const exportedCommitment = String(exported.commitment || "");
		        let ziffleCommitment = currentZifflePositionCommitment || exportedCommitment;
		        let ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
		        if (!exportedCommitment || ziffleDeckHash) {
		          if (!ziffleDeckHash) {
		            ziffleCommitment = currentZifflePositionCommitment;
	            ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
	          }
	        } else {
	          if (currentZifflePosition != null && currentZifflePositionCommitment) {
	            position = currentZifflePosition;
	            positionCommitment = currentZifflePositionCommitment;
	            rememberZiffleOpeningPosition(exported.owner, preferredSlot, position);
	          }
          ziffleCommitment = "";
          ziffleDeckHash = "";
        }
        if (!exportedCommitment && !ziffleDeckHash) {
          const hiddenCommitments = [
	            publicPositionCommitment,
	            hiddenPositionCommitment,
	          ];
          ziffleCommitment = hiddenCommitments.find((commitment) =>
            ziffleDeckHashFromCommitment(commitment)
          ) || "";
          ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
        }
	        if (ziffleDeckHash) {
		          position = Number(
		            currentZifflePosition
		            ?? zifflePositionFromCommitment(ziffleCommitment)
		            ?? exported.slot
		          );
	          const ceremony = ziffleCeremonyForOwner(exported.owner, {
	            commitment: ziffleCommitment,
	            context: ziffleContextForCommitment(
	              ziffleContext,
	              ziffleContextCommitment,
	              ziffleCommitment
	            ),
	          });
	          if (!ceremony) {
	            throw new Error(`Missing ziffle ceremony for opening player ${Number(exported.owner) + 1}`);
	          }
	          ziffleContext = ziffleContextFromCeremony(ceremony);
	          ziffleContextCommitment = ziffleRuntimeCommitment(ceremony.deckHash, position);
	          if (ziffleCeremonyHasObjectOrder(ceremony)) {
	            positionCommitment = ziffleCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
	            const openingObjectId = exported.object_id ?? exported.objectId ?? objectId;
	            const {
	              resolvedRevealSlot,
	              shuffleOriginalSlot,
	            } = await resolveCommittedSlotForZifflePosition({
	              owner: exported.owner,
	              ceremony,
	              position,
	              card: openingCard,
	              objectId: openingObjectId,
	              manifest,
	              options,
	            });
	            if (!resolvedRevealSlot) {
	              throw new Error(
	                `Ziffle opening could not resolve committed slot `
	                + `(owner ${Number(exported.owner) + 1}, position ${position}, `
	                + `shuffle slot ${shuffleOriginalSlot ?? "none"}, card ${String(openingCard || "")})`
	              );
	            }
	            preferredSlot = Number(resolvedRevealSlot.slot);
	            openingCard = String(resolvedRevealSlot.card || openingCard || "");
	            ziffleProofShuffleObjectId =
	              resolvedRevealSlot.shuffleObjectId
	              ?? resolvedRevealSlot.objectId
	              ?? ziffleShuffleObjectIdForPosition(ceremony, position)
	              ?? Number(openingObjectId);
	            if (!Number.isSafeInteger(Number(ziffleProofShuffleObjectId)) || Number(ziffleProofShuffleObjectId) < 0) {
	              ziffleProofShuffleObjectId = null;
	            }
	            rememberZiffleOpeningPosition(exported.owner, preferredSlot, position);
	          } else {
	            if (typeof currentGame.ziffleRevealCard !== "function") {
	              throw new Error("Ziffle opening reveal backend is not available");
	            }
	            const tokens = await collectZiffleRevealTokens(ceremony, position, options);
	            const reveal = await currentGame.ziffleRevealCard({
	              deckCount: Number(ceremony.deckCount),
	              context: String(ceremony.context || ""),
	              keyContext: ziffleKeyContextForCeremony(ceremony),
	              keys: cloneMultiplayerPayload(ceremony.keys || []),
	              steps: cloneMultiplayerPayload(ceremony.steps || []),
	              cardPosition: position,
	              tokens,
		            });
		            ziffleProofShuffleOriginalSlot = Number(reveal.originalSlot);
		            let resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
		              owner: exported.owner,
		              ceremony,
		              shuffleOriginalSlot: ziffleProofShuffleOriginalSlot,
		              shuffleOriginalSlotIsVerified: true,
		              position,
	              card: openingCard,
	              objectId: exported.object_id ?? exported.objectId ?? objectId,
	              manifest,
	              options,
	            });
		            if (!resolvedRevealSlot && openingCard) {
		              resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
		                owner: exported.owner,
		                ceremony,
		                shuffleOriginalSlot: ziffleProofShuffleOriginalSlot,
		                shuffleOriginalSlotIsVerified: true,
		                position,
	                card: "",
	                objectId: exported.object_id ?? exported.objectId ?? objectId,
	                manifest,
	                options,
	              });
		            }
			            if (!resolvedRevealSlot) {
			              if (
			                exportedCommitment
			                && !exportedCommitmentIsZiffle
			                && !mustUseZiffleOpening
			              ) {
			                ziffleCommitment = "";
			                ziffleDeckHash = "";
			                position = null;
			                positionCommitment = "";
			                ziffleProofShuffleOriginalSlot = null;
			                ziffleProofShuffleObjectId = null;
			              } else {
			                throw new Error(
			                  `Ziffle opening could not resolve committed slot `
			                  + `(owner ${Number(exported.owner) + 1}, position ${position}, `
			                  + `shuffle slot ${ziffleProofShuffleOriginalSlot}, card ${String(exported.card || "")})`
			                );
			              }
			            }
			            if (resolvedRevealSlot) {
			              preferredSlot = Number(resolvedRevealSlot.slot);
			              openingCard = String(resolvedRevealSlot.card || openingCard || "");
			              ziffleProofShuffleObjectId =
			                resolvedRevealSlot.shuffleObjectId ?? resolvedRevealSlot.objectId ?? null;
			              positionCommitment = ziffleCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
		              ziffleProofCeremony = ceremony;
		              ziffleProofTokens = tokens;
		              rememberZiffleOpeningPosition(exported.owner, preferredSlot, position);
			            }
	          }
	        }
	        const built = await buildDeckSlotOpeningForExport({
	          manifest,
	          preferredSlot,
	          card: openingCard,
	          exportedCommitment: ziffleDeckHash || exportedCommitmentIsZiffle
	            ? ""
	            : exportedCommitment,
	          label: "Local hidden card opening",
	        });
        opening = built.opening;
        remappedFromSlot = built.remappedFromSlot;
	        if (position != null) {
	          opening.position = Number(position);
	          opening.positionCommitment = positionCommitment;
	          if (ziffleContext) {
	            opening.ziffleContext = ziffleContext;
	          }
	          if (ziffleProofShuffleObjectId != null) {
	            opening.shuffleObjectId = Number(ziffleProofShuffleObjectId);
	          }
		          if (ziffleProofCeremony && !ziffleCeremonyHasObjectOrder(ziffleProofCeremony)) {
	            opening.ziffleReveal = buildZiffleOpeningProof({
	              opening,
	              ceremony: ziffleProofCeremony,
              position,
              originalSlot: Number(opening.slot),
              shuffleOriginalSlot: ziffleProofShuffleOriginalSlot ?? Number(opening.slot),
              positionCommitment,
              tokens: ziffleProofTokens || [],
              compact: true,
            });
          }
	        }
	      }
	      if (publicPositionIdentity) {
	        opening = withPinnedPublicZifflePosition(opening, publicPositionIdentity);
	        ziffleContext = "";
	      }
	      opening.objectId = Number(exported.object_id ?? exported.objectId ?? objectId);
      opening.timing = options.timing || (commandObjectIds.has(Number(objectId)) ? "pre" : "post");
      const shouldUseRememberedZifflePosition = Boolean(
        currentZifflePositionCommitment
        || ziffleDeckHashFromCommitment(opening.positionCommitment)
      );
      const zifflePosition = shouldUseRememberedZifflePosition
        ? ziffleOpeningPositionForSlot(opening.owner, opening.slot)
        : null;
      if (zifflePosition != null) {
        opening.position = Number(zifflePosition);
        const ceremony = ziffleCeremonyForOwner(opening.owner, {
          commitment: opening.positionCommitment,
          context: ziffleContext,
        });
        if (ceremony?.deckHash && !opening.positionCommitment) {
          opening.positionCommitment = ziffleRuntimeCommitment(ceremony.deckHash, zifflePosition);
        }
        if (ceremony?.context) {
          opening.ziffleContext = ziffleContextFromCeremony(ceremony);
          ziffleContext = opening.ziffleContext;
        }
      }
	      if (remappedFromSlot != null) {
	        opening.reportedSlot = Number(remappedFromSlot);
	      }
	      opening = await ensureZiffleOpeningProof(opening, options);
	      opening = await sanitizeObjectBoundOpening(opening);
	      rememberLocalRevealedOpening(opening, {
	        objectId: opening.objectId,
	        positionCommitment: opening.positionCommitment,
	        ziffleContext,
	      });
      openings.push(opening);
      notifyOpeningBuilt(options, opening, {
        source: "command_public_open",
        index: openings.length,
        total: objectIds.size,
      });
      seen.add(key);
    }
	    return openings;
	  }, [
      buildDeckSlotOpeningForExport,
      collectZiffleRevealTokens,
	      currentHiddenCardMetadataForObject,
	      localRevealedOpeningForRequirement,
	      localRevealedOpeningForExport,
	      privateDeckManifestForOwner,
      rememberLocalRevealedOpening,
      sanitizeObjectBoundOpening,
      rememberZiffleOpeningPosition,
      resolveCommittedSlotForZifflePosition,
      resolveCommittedZiffleRevealSlot,
      resolveLocalCryptoPlayerIndex,
      ziffleCeremonyForOwner,
      ziffleOpeningPositionForSlot,
    ]);

	  const buildLocalOpeningFromRequirement = useCallback(async (requirement, exported = null, options = {}) => {
	    const owner = Number(exported?.owner ?? requirement?.owner);
	    if (!Number.isInteger(owner)) {
	      throw new Error("Crypto opening requirement is missing an owner");
		    }
	    const manifest = privateDeckManifestForOwner(owner);
	      let cachedOpeningForRequirement = exported
        ? localRevealedOpeningForExport(exported)
        : localRevealedOpeningForRequirement(requirement);
		    if (!manifest && !cachedOpeningForRequirement) {
		      throw new Error(`Missing private deck opening material for player ${owner + 1}`);
		    }

	    let originalSlot = Number(exported?.slot ?? requirement?.slot);
	    if (!Number.isInteger(originalSlot) || originalSlot < 0) {
	      throw new Error("Crypto opening requirement is missing a committed slot");
	    }
	    let position = null;
	    let positionCommitment = "";
	      let ziffleProofCeremony = null;
	      let ziffleProofTokens = null;
	      let ziffleProofShuffleOriginalSlot = null;
	      let ziffleProofShuffleObjectId = null;
	      const hiddenMetadata = await currentHiddenCardMetadataForObject(
	        exported?.object_id ?? exported?.objectId ?? requirement?.objectId
	      );
		    const exportedCommitment = String(exported?.commitment || requirement?.commitment || "");
		    const exportedCommitmentIsZiffle = Boolean(
		      ziffleDeckHashFromCommitment(exportedCommitment)
		    );
		    const directSlotCard = String(exported?.card || requirement?.card || "");
		    const directSlotSecret = (manifest?.slotSecrets || []).find(
		      (secret) => Number(secret?.slot) === Number(originalSlot)
		    );
		    const directSlotMatchesCard = Boolean(
		      directSlotSecret
		      && (!directSlotCard || String(directSlotSecret.card || "") === directSlotCard)
		    );
	      const objectOrderedPosition = zifflePositionForObjectId(
	        owner,
	        exported?.object_id ?? exported?.objectId ?? requirement?.objectId
	      );
	      const originalSlotOrderedPosition = zifflePositionForOriginalSlot(
	        owner,
	        exported?.slot ?? requirement?.slot
	      );
	      const orderedPosition = objectOrderedPosition || originalSlotOrderedPosition;
	      let ziffleContext = orderedPosition?.ziffleContext || "";
	      let ziffleContextCommitment = orderedPosition?.positionCommitment || "";
	      const exportedPublicSlot = exported?.publicSlot ?? exported?.public_slot ?? null;
	      const requirementPublicSlot = requirement?.publicSlot ?? requirement?.public_slot ?? null;
		      const exportedPublicCommitment = String(
		        exported?.publicCommitment || exported?.public_commitment || ""
		      );
		      const requirementPublicCommitment = String(
		        requirement?.publicCommitment || requirement?.public_commitment || ""
		      );
		      const publicPositionIdentity = zifflePublicPositionFromSources(
		        hiddenMetadata,
		        {
		          publicSlot: exportedPublicSlot,
		          publicCommitment: exportedPublicCommitment,
		        },
		        {
		          publicSlot: requirementPublicSlot,
		          publicCommitment: requirementPublicCommitment,
		        }
		      );
      if (cachedOpeningForRequirement && publicPositionIdentity?.useAsPosition === false) {
        cachedOpeningForRequirement = cloneMultiplayerPayload(cachedOpeningForRequirement);
        delete cachedOpeningForRequirement.position;
        delete cachedOpeningForRequirement.positionCommitment;
        delete cachedOpeningForRequirement.ziffleContext;
        delete cachedOpeningForRequirement.ziffleReveal;
        delete cachedOpeningForRequirement.ziffleProof;
        delete cachedOpeningForRequirement.positionOpeningProof;
      }
		      const publicPosition = publicPositionIdentity?.useAsPosition === false
		        ? null
		        : publicPositionIdentity?.position ?? orderedPosition?.position ?? null;
		      const publicPositionCommitment =
		        publicPositionIdentity?.useAsPosition === false
		          ? ""
		          : publicPositionIdentity?.useAsPosition !== false
		          ? publicPositionIdentity?.positionCommitment
		            || (publicPosition != null ? orderedPosition?.positionCommitment || "" : "")
		          : "";
      const hiddenPositionCommitment =
        hiddenMetadata?.commitment && ziffleDeckHashFromCommitment(hiddenMetadata.commitment)
          ? String(hiddenMetadata.commitment)
          : "";
      const identityPosition = ziffleIdentityPositionFromSources(
        hiddenMetadata,
        exported,
        requirement
      );
      const currentZifflePosition =
        publicPosition
        ?? identityPosition?.position
        ?? (hiddenPositionCommitment ? Number(hiddenMetadata?.slot) : null);
      const currentZifflePositionCommitment =
        publicPositionCommitment || identityPosition?.positionCommitment || hiddenPositionCommitment;
      const mustUseZiffleOpening = Boolean(
        currentZifflePosition != null
        && ziffleDeckHashFromCommitment(currentZifflePositionCommitment)
      );
	      const preferDirectDeckOpening = Boolean(
	        Number.isSafeInteger(originalSlot)
	        && originalSlot >= 0
	        && (exported?.card || requirement?.card)
	        && directSlotMatchesCard
	      );
      const directOpeningHasLiveHiddenPosition = Boolean(
        preferDirectDeckOpening
        && currentZifflePosition != null
        && currentZifflePositionCommitment
        && (
          hiddenMetadata
          || orderedPosition?.position != null
          || requirementPublicCommitment
          || exportedPublicCommitment
        )
      );
		      if (
		        cachedOpeningForRequirement
		        && currentZifflePositionCommitment
		        && !cachedOpeningMatchesZifflePosition(
		          cachedOpeningForRequirement,
		          currentZifflePosition,
		          currentZifflePositionCommitment
		        )
		      ) {
		        cachedOpeningForRequirement = null;
		      }
      if (
        cachedOpeningForRequirement
        && currentZifflePosition != null
        && cachedOpeningForRequirement.position != null
        && Number(cachedOpeningForRequirement.position) !== Number(currentZifflePosition)
      ) {
        cachedOpeningForRequirement = null;
      }
      if (
        cachedOpeningForRequirement
        && !currentZifflePositionCommitment
        && !exportedCommitmentIsZiffle
        && !openingHasZifflePosition(cachedOpeningForRequirement)
      ) {
        cachedOpeningForRequirement = cloneMultiplayerPayload(cachedOpeningForRequirement);
        delete cachedOpeningForRequirement.position;
        delete cachedOpeningForRequirement.positionCommitment;
        delete cachedOpeningForRequirement.ziffleReveal;
        delete cachedOpeningForRequirement.ziffleProof;
        delete cachedOpeningForRequirement.positionOpeningProof;
	      }
					      let ziffleCommitment =
	                (!preferDirectDeckOpening || directOpeningHasLiveHiddenPosition || mustUseZiffleOpening)
	                  ? currentZifflePositionCommitment || (preferDirectDeckOpening ? "" : exportedCommitment)
	                  : "";
				    let ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
			      if (!exportedCommitment || ziffleDeckHash) {
			        if (!ziffleDeckHash) {
			          ziffleCommitment = currentZifflePositionCommitment;
		          ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
		        }
		      } else {
			        if (
			          (!preferDirectDeckOpening || directOpeningHasLiveHiddenPosition)
			          && currentZifflePosition != null
			          && currentZifflePositionCommitment
			        ) {
		          position = currentZifflePosition;
		          positionCommitment = currentZifflePositionCommitment;
		          rememberZiffleOpeningPosition(owner, originalSlot, position);
		        }
        ziffleCommitment = "";
        ziffleDeckHash = "";
      }
      if (!exportedCommitment && !ziffleDeckHash) {
        const hiddenCommitments = [
	          publicPositionCommitment,
	          hiddenPositionCommitment,
	        ];
        ziffleCommitment = hiddenCommitments.find((commitment) =>
          ziffleDeckHashFromCommitment(commitment)
        ) || "";
        ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
      }
		    let ziffleResolvedCard = "";
		    if (ziffleDeckHash) {
		      const currentGame = gameRef.current;
			      position = Number(
	            currentZifflePosition
	            ?? zifflePositionFromCommitment(ziffleCommitment)
	            ?? exported?.slot
	            ?? requirement?.slot
	          );
		      const ceremony = ziffleCeremonyForOwner(owner, {
		        commitment: ziffleCommitment,
		        context: ziffleContextForCommitment(
		          ziffleContext,
		          ziffleContextCommitment,
		          ziffleCommitment
		        ),
		      });
		      if (!ceremony) {
		        throw new Error(`Missing ziffle ceremony for opening player ${owner + 1}`);
		      }
		      ziffleContext = ziffleContextFromCeremony(ceremony);
		      ziffleContextCommitment = ziffleRuntimeCommitment(ceremony.deckHash, position);
		      if (ziffleCeremonyHasObjectOrder(ceremony)) {
		        positionCommitment = ziffleCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
		        const openingObjectId =
		          exported?.object_id ?? exported?.objectId ?? requirement?.objectId ?? requirement?.object_id;
		        const {
		          resolvedRevealSlot: resolvedZiffleRevealSlot,
		          shuffleOriginalSlot,
		        } = await resolveCommittedSlotForZifflePosition({
		          owner,
		          ceremony,
		          position,
		          card: exported?.card || requirement?.card || "",
		          objectId: openingObjectId,
		          manifest,
		          options,
		        });
		        let resolvedRevealSlot = resolvedZiffleRevealSlot;
		        if (!resolvedRevealSlot) {
		          const directSlot = Number(exported?.slot ?? requirement?.slot);
		          const directSecret = (manifest?.slotSecrets || []).find(
		            (secret) => Number(secret?.slot) === directSlot
		          );
		          const expectedCard = String(exported?.card || requirement?.card || "");
		          const linkedShuffleObjectId =
		            ziffleShuffleObjectIdForPosition(ceremony, position)
		            ?? Number(openingObjectId);
			          const candidate = directSecret
			            ? {
		              owner,
		              slot: directSlot,
		              card: String(directSecret.card || expectedCard || ""),
		              commitment: String(directSecret.commitment || ""),
		              objectId: Number.isSafeInteger(Number(openingObjectId)) && Number(openingObjectId) >= 0
		                ? Number(openingObjectId)
		                : null,
			              shuffleObjectId: Number.isSafeInteger(Number(linkedShuffleObjectId))
			                && Number(linkedShuffleObjectId) >= 0
			                  ? Number(linkedShuffleObjectId)
			                  : null,
			              position,
			              positionCommitment,
			              ziffleContext,
			              shuffleOriginalSlot,
				              source: "direct_requirement_slot",
				            }
			            : null;
		          const directRequirementPositionOpening = Boolean(
		            candidate
		            && ziffleCeremonyHasObjectOrder(ceremony)
			            && exportedCommitment
			            && !ziffleDeckHashFromCommitment(exportedCommitment)
			            && String(candidate.commitment || "") === exportedCommitment
			            && ziffleDeckHashFromCommitment(positionCommitment)
			            && (
			              shuffleOriginalSlot == null
			              || Number(candidate.slot) === Number(shuffleOriginalSlot)
			            )
			          );
			          if (
			            candidate
			            && (!expectedCard || String(candidate.card || "") === expectedCard)
			            && (
			              ziffleObjectOrderLinksOpening(ceremony, shuffleOriginalSlot, position, candidate)
			              || directRequirementPositionOpening
			            )
			          ) {
			            resolvedRevealSlot = candidate;
			          }
		        }
			        if (!resolvedRevealSlot) {
			          if (preferDirectDeckOpening && !mustUseZiffleOpening) {
			            ziffleCommitment = "";
			            ziffleDeckHash = "";
			            position = null;
			            positionCommitment = "";
			            ziffleProofShuffleOriginalSlot = null;
			            ziffleProofShuffleObjectId = null;
			          } else {
			            throw new Error(
			              `Ziffle opening could not resolve committed slot `
			              + `(owner ${owner + 1}, position ${position}, `
			              + `shuffle slot ${shuffleOriginalSlot ?? "none"}, `
			              + `card ${String(exported?.card || requirement?.card || "")})`
			            );
			          }
			        }
			        if (resolvedRevealSlot) {
			          originalSlot = Number(resolvedRevealSlot.slot);
			          ziffleResolvedCard = String(resolvedRevealSlot.card || "");
			          ziffleProofShuffleOriginalSlot = shuffleOriginalSlot;
			          ziffleProofShuffleObjectId =
			            resolvedRevealSlot.shuffleObjectId
			            ?? resolvedRevealSlot.objectId
			            ?? ziffleShuffleObjectIdForPosition(ceremony, position)
			            ?? Number(openingObjectId);
			          if (!Number.isSafeInteger(Number(ziffleProofShuffleObjectId)) || Number(ziffleProofShuffleObjectId) < 0) {
			            ziffleProofShuffleObjectId = null;
			          }
			          rememberZiffleOpeningPosition(owner, originalSlot, position);
			        }
		      } else {
		        if (!currentGame || typeof currentGame.ziffleRevealCard !== "function") {
		          throw new Error("Ziffle opening reveal backend is not available");
		        }
		        const tokens = await collectZiffleRevealTokens(ceremony, position, options);
		        const reveal = await currentGame.ziffleRevealCard({
		          deckCount: Number(ceremony.deckCount),
		          context: String(ceremony.context || ""),
		          keyContext: ziffleKeyContextForCeremony(ceremony),
		          keys: cloneMultiplayerPayload(ceremony.keys || []),
		          steps: cloneMultiplayerPayload(ceremony.steps || []),
		          cardPosition: position,
		          tokens,
		        });
			      ziffleProofShuffleOriginalSlot = Number(reveal.originalSlot);
			      let resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
			        owner,
			        ceremony,
			        shuffleOriginalSlot: ziffleProofShuffleOriginalSlot,
			        shuffleOriginalSlotIsVerified: true,
			        position,
				        card: exported?.card || requirement?.card || "",
				        objectId: exported?.object_id ?? exported?.objectId ?? requirement?.objectId,
				        manifest,
	              options,
				      });
			      if (!resolvedRevealSlot && (exported?.card || requirement?.card)) {
			        resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
			          owner,
			          ceremony,
			          shuffleOriginalSlot: ziffleProofShuffleOriginalSlot,
			          shuffleOriginalSlotIsVerified: true,
			          position,
				          card: "",
				          objectId: exported?.object_id ?? exported?.objectId ?? requirement?.objectId,
				          manifest,
	                options,
				        });
			      }
				      if (!resolvedRevealSlot) {
				        if (preferDirectDeckOpening && !mustUseZiffleOpening) {
				          ziffleCommitment = "";
				          ziffleDeckHash = "";
				          position = null;
				          positionCommitment = "";
				          ziffleProofShuffleOriginalSlot = null;
				          ziffleProofShuffleObjectId = null;
				        } else {
				          throw new Error(
				            `Ziffle opening could not resolve committed slot `
				            + `(owner ${owner + 1}, position ${position}, `
				            + `shuffle slot ${ziffleProofShuffleOriginalSlot}, `
				            + `card ${String(exported?.card || requirement?.card || "")})`
				          );
				        }
				      }
				      if (resolvedRevealSlot) {
				        originalSlot = Number(resolvedRevealSlot.slot);
				        ziffleResolvedCard = String(resolvedRevealSlot.card || "");
				        ziffleProofShuffleObjectId =
				          resolvedRevealSlot.shuffleObjectId ?? resolvedRevealSlot.objectId ?? null;
				        positionCommitment = ziffleCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
		            ziffleContext = ziffleContextFromCeremony(ceremony);
		            ziffleProofCeremony = ceremony;
		            ziffleProofTokens = tokens;
			          rememberZiffleOpeningPosition(owner, originalSlot, position);
				      }
			      }
			    }
      if (
        position == null
        && cachedOpeningForRequirement?.position != null
	        && !preferDirectDeckOpening
	        && (
	          currentZifflePositionCommitment
	          || ziffleDeckHashFromCommitment(cachedOpeningForRequirement.positionCommitment)
	        )
      ) {
        position = Number(cachedOpeningForRequirement.position);
        positionCommitment = String(cachedOpeningForRequirement.positionCommitment || positionCommitment || "");
      }
	      if (
	        position == null
	        && !preferDirectDeckOpening
	        && (currentZifflePositionCommitment || ziffleDeckHashFromCommitment(positionCommitment))
	      ) {
	        const rememberedPosition = ziffleOpeningPositionForSlot(owner, originalSlot);
	        if (rememberedPosition != null) {
	          position = Number(rememberedPosition);
	          const ceremony = ziffleCeremonyForOwner(owner, {
	            commitment: positionCommitment || currentZifflePositionCommitment,
	            context: ziffleContextForCommitment(
	              ziffleContext,
	              ziffleContextCommitment,
	              positionCommitment || currentZifflePositionCommitment
	            ),
	          });
	          if (ceremony?.deckHash && !positionCommitment) {
	            positionCommitment = ziffleRuntimeCommitment(ceremony.deckHash, position);
	          }
	          if (ceremony?.context) {
	            ziffleContext = ziffleContextFromCeremony(ceremony);
	            ziffleContextCommitment = ziffleRuntimeCommitment(ceremony.deckHash, position);
	          }
	        }
	      }

		    const secret = (manifest?.slotSecrets || []).find(
	      (entry) => Number(entry.slot) === Number(originalSlot)
	    );
	    if (!secret && !cachedOpeningForRequirement && !exported?.card && !requirement?.card) {
	      throw new Error(`Missing private deck opening for slot ${Number(originalSlot)}`);
	    }
	    const card = String(ziffleResolvedCard || exported?.card || requirement?.card || secret?.card || cachedOpeningForRequirement?.card || "");
      let cachedOpening = cachedOpeningForRequirement;
      let remappedFromSlot = null;
	      let opening = cachedOpening;
	      if (!opening) {
	        let built = null;
	        try {
		          built = await buildDeckSlotOpeningForExport({
		            manifest,
		            preferredSlot: originalSlot,
		            card,
		            exportedCommitment: ziffleDeckHash || exportedCommitmentIsZiffle
		              ? ""
		              : exportedCommitment,
		            label: "Private deck opening",
		          });
	        } catch (err) {
	          const currentGame = gameRef.current;
	          const exportedCommitmentBlocksPositionFallback = Boolean(
	            exportedCommitment
	            && !ziffleDeckHashFromCommitment(exportedCommitment)
	            && !ziffleDeckHashFromCommitment(positionCommitment)
	            && !ziffleDeckHashFromCommitment(currentZifflePositionCommitment)
	          );
	          const candidatePositions = [
	            position,
	            zifflePositionFromCommitment(positionCommitment),
	            zifflePositionFromCommitment(currentZifflePositionCommitment),
	            zifflePositionFromCommitment(requirement?.positionCommitment),
	            zifflePositionFromCommitment(requirement?.position_commitment),
	            zifflePositionFromCommitment(requirement?.commitment),
	            zifflePositionFromCommitment(exported?.positionCommitment),
	            zifflePositionFromCommitment(exported?.position_commitment),
	            zifflePositionFromCommitment(exported?.commitment),
	            requirement?.position,
	            exported?.position,
	            requirement?.slot,
	            exported?.slot,
	          ]
	            .map((entry) => Number(entry))
	            .filter((entry, index, list) =>
	              Number.isSafeInteger(entry)
	              && entry >= 0
	              && list.indexOf(entry) === index
	            );
	          if (
	            !exportedCommitmentBlocksPositionFallback
	            && candidatePositions.length > 0
	            && typeof currentGame?.ziffleRevealCard === "function"
	          ) {
	            const candidateCommitments = [
	              positionCommitment,
	              currentZifflePositionCommitment,
	              requirement?.positionCommitment,
	              requirement?.position_commitment,
	              requirement?.commitment,
	              exported?.positionCommitment,
	              exported?.position_commitment,
	              exported?.commitment,
	            ].map((entry) => String(entry || "")).filter(Boolean);
	            for (const candidatePosition of candidatePositions) {
	              const matchingPositionCommitment = candidateCommitments.find((commitment) =>
	                zifflePositionFromCommitment(commitment) === candidatePosition
	              ) || "";
		              const ceremony = ziffleCeremonyForOwner(owner, {
		                commitment: matchingPositionCommitment,
		                context: ziffleContextForCommitment(
		                  ziffleContext,
		                  ziffleContextCommitment,
		                  matchingPositionCommitment
		                ),
		              });
		              if (!ceremony?.deckHash) continue;
		              if (ziffleCeremonyHasObjectOrder(ceremony)) continue;
		              ziffleContext = ziffleContextFromCeremony(ceremony);
		              ziffleContextCommitment = ziffleRuntimeCommitment(ceremony.deckHash, candidatePosition);
		              const expectedPositionCommitment = ziffleRuntimeCommitment(
		                ceremony.deckHash,
		                candidatePosition
	              );
	              const normalizedPositionCommitment = matchingPositionCommitment || expectedPositionCommitment;
	              if (normalizedPositionCommitment !== expectedPositionCommitment) {
	                continue;
	              }
	              try {
	                const tokens = await collectZiffleRevealTokens(ceremony, candidatePosition, options);
	                const reveal = await currentGame.ziffleRevealCard({
	                  deckCount: Number(ceremony.deckCount),
	                  context: String(ceremony.context || ""),
	                  keyContext: ziffleKeyContextForCeremony(ceremony),
	                  keys: cloneMultiplayerPayload(ceremony.keys || []),
	                  steps: cloneMultiplayerPayload(ceremony.steps || []),
	                  cardPosition: candidatePosition,
	                  tokens,
	                });
	                const revealShuffleOriginalSlot = Number(reveal.originalSlot);
	                let resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
	                  owner,
	                  ceremony,
	                  shuffleOriginalSlot: revealShuffleOriginalSlot,
	                  shuffleOriginalSlotIsVerified: true,
	                  position: candidatePosition,
		                  card,
		                  objectId: exported?.object_id ?? exported?.objectId ?? requirement?.objectId,
		                  manifest,
                  options,
		                });
	                if (!resolvedRevealSlot && card) {
	                  resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
	                    owner,
	                    ceremony,
	                    shuffleOriginalSlot: revealShuffleOriginalSlot,
	                    shuffleOriginalSlotIsVerified: true,
	                    position: candidatePosition,
		                    card: "",
		                    objectId: exported?.object_id ?? exported?.objectId ?? requirement?.objectId,
		                    manifest,
                    options,
		                  });
	                }
	                const candidateOriginalSlot = Number(resolvedRevealSlot?.slot);
	                if (!Number.isSafeInteger(candidateOriginalSlot) || candidateOriginalSlot < 0) {
	                  continue;
	                }
	                const candidateBuilt = await buildDeckSlotOpeningForExport({
	                  manifest,
	                  preferredSlot: candidateOriginalSlot,
	                  card: resolvedRevealSlot?.card || card,
	                  exportedCommitment: "",
	                  label: "Private deck opening",
	                });
	                position = candidatePosition;
		                originalSlot = candidateOriginalSlot;
		                positionCommitment = expectedPositionCommitment;
		                ziffleContext = ziffleContextFromCeremony(ceremony);
		                ziffleContextCommitment = expectedPositionCommitment;
		                ziffleProofCeremony = ceremony;
	                ziffleProofTokens = tokens;
	                ziffleProofShuffleOriginalSlot = revealShuffleOriginalSlot;
	                ziffleProofShuffleObjectId =
	                  resolvedRevealSlot?.shuffleObjectId ?? resolvedRevealSlot?.objectId ?? null;
	                rememberZiffleOpeningPosition(owner, originalSlot, position);
	                built = candidateBuilt;
	                break;
	              } catch {
	                // Another candidate may be the runtime position for this requirement.
	              }
	            }
	          }
	          if (!built) throw err;
	        }
	        opening = built.opening;
	        remappedFromSlot = built.remappedFromSlot;
	      }
      if (remappedFromSlot != null) {
        originalSlot = Number(opening.slot);
      }
		      if (position != null) {
				        opening = {
				          ...opening,
				          position,
			          ...(positionCommitment ? { positionCommitment } : {}),
		          ...(ziffleContext ? { ziffleContext } : {}),
		          ...(ziffleProofShuffleObjectId != null
		            ? { shuffleObjectId: Number(ziffleProofShuffleObjectId) }
	            : {}),
	        };
        if (ziffleProofCeremony && !ziffleCeremonyHasObjectOrder(ziffleProofCeremony)) {
          opening.ziffleReveal = buildZiffleOpeningProof({
            opening,
            ceremony: ziffleProofCeremony,
            position,
            originalSlot: Number(opening.slot),
            shuffleOriginalSlot: ziffleProofShuffleOriginalSlot ?? Number(opening.slot),
            positionCommitment,
            tokens: ziffleProofTokens || [],
            compact: true,
          });
		        }
		      }
		      if (publicPositionIdentity) {
		        opening = withPinnedPublicZifflePosition(opening, publicPositionIdentity);
		        ziffleContext = "";
		      }
		      opening = await ensureZiffleOpeningProof(opening, options);
	      let finalOpening = {
	        ...opening,
	        ...(requirement?.objectId != null
	          ? { objectId: Number(requirement.objectId) }
	          : exported?.object_id != null || exported?.objectId != null
	            ? { objectId: Number(exported.object_id ?? exported.objectId) }
	            : {}),
	        timing: "post",
	        ...(remappedFromSlot != null ? { reportedSlot: Number(remappedFromSlot) } : {}),
	      };
	      finalOpening = await sanitizeObjectBoundOpening(finalOpening);
	      finalOpening = await ensureZiffleOpeningProof(finalOpening, options);
	      finalOpening = await sanitizeObjectBoundOpening(finalOpening);
	      if (exported) {
		        rememberLocalRevealedOpening(finalOpening, {
		          objectId: finalOpening.objectId,
		          position: finalOpening.position ?? position,
		          positionCommitment: finalOpening.positionCommitment ?? positionCommitment,
		          ziffleContext,
		        });
	      }
		    return {
		      opening: finalOpening,
		      owner,
		      originalSlot,
		      position: finalOpening.position ?? position,
		      positionCommitment: finalOpening.positionCommitment ?? positionCommitment,
		    };
		  }, [
		    collectZiffleRevealTokens,
      buildDeckSlotOpeningForExport,
		      currentHiddenCardMetadataForObject,
	      localRevealedOpeningForExport,
	      localRevealedOpeningForRequirement,
		    privateDeckManifestForOwner,
	    rememberLocalRevealedOpening,
	    rememberZiffleOpeningPosition,
	    resolveCommittedZiffleRevealSlot,
	    sanitizeObjectBoundOpening,
	    ziffleCeremonyForOwner,
      ziffleOpeningPositionForSlot,
	  ]);

	  const prefetchZiffleRevealTokensForPublicOpenRequirements = useCallback(async (
	    requirements = [],
	    options = {}
		  ) => {
		    const localSeat = resolveLocalCryptoPlayerIndex();
		    const groups = new Map();
	    for (const requirement of requirements || []) {
	      if (String(requirement?.type || "") !== "public_open") continue;
	      if (Number(requirement.owner) !== Number(localSeat)) continue;
	      const objectId = requirement?.objectId ?? requirement?.object_id ?? null;
	      const hiddenMetadata = await currentHiddenCardMetadataForObject(objectId);
	      const objectOrderedPosition = zifflePositionForObjectId(
		        localSeat,
		        objectId
		      );
		      const originalSlotOrderedPosition = zifflePositionForOriginalSlot(localSeat, requirement?.slot);
		      const orderedPosition = objectOrderedPosition || originalSlotOrderedPosition;
		      const ziffleContext = orderedPosition?.ziffleContext || "";
	      const requirementPublicSlot = requirement?.publicSlot ?? requirement?.public_slot ?? null;
		      const requirementPublicCommitment = String(
		        requirement?.publicCommitment || requirement?.public_commitment || ""
		      );
		      const publicPositionIdentity = zifflePublicPositionFromSources(
		        hiddenMetadata,
		        {
		          publicSlot: requirementPublicSlot,
		          publicCommitment: requirementPublicCommitment,
		        }
		      );
		      const publicPosition = publicPositionIdentity?.useAsPosition === false
		        ? null
		        : publicPositionIdentity?.position ?? orderedPosition?.position ?? null;
	      const publicPositionCommitment =
	        publicPositionIdentity?.useAsPosition === false
	          ? ""
	          : publicPositionIdentity?.useAsPosition !== false
	          ? publicPositionIdentity?.positionCommitment || ""
	          : "";
		      const hiddenPositionCommitment =
		        hiddenMetadata?.commitment && ziffleDeckHashFromCommitment(hiddenMetadata.commitment)
		          ? String(hiddenMetadata.commitment)
		          : "";
		      const identityPosition = ziffleIdentityPositionFromSources(
		        hiddenMetadata,
		        {
		          slot: requirement?.slot,
		          commitment: requirement?.commitment,
		        }
		      );
			      let ziffleCommitment = String(
			        requirement?.positionCommitment
			        || requirement?.position_commitment
			        || publicPositionCommitment
			        || identityPosition?.positionCommitment
			        || requirement?.commitment
			        || ""
			      );
		      let ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
		      if (!ziffleDeckHash && publicPosition != null && publicPositionCommitment) {
		        ziffleCommitment = publicPositionCommitment;
		        ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
		      }
	      if (!ziffleDeckHash) {
	        ziffleCommitment = hiddenPositionCommitment || orderedPosition?.positionCommitment || "";
	        ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
	      }
	      if (!ziffleDeckHash) continue;
		      const position = Number(
		        publicPosition
		        ?? zifflePositionFromCommitment(ziffleCommitment)
		        ?? identityPosition?.position
		        ?? (hiddenPositionCommitment ? Number(hiddenMetadata?.slot) : null)
		        ?? requirement?.slot
		      );
	      if (!Number.isSafeInteger(position) || position < 0) continue;
	      const ceremony = ziffleCeremonyForOwner(localSeat, {
	        commitment: ziffleCommitment,
	        context: ziffleContext,
	      });
	      if (!ceremony) continue;
	      const groupKey = [
	        Number(localSeat),
	        String(ceremony.context || ""),
	        String(ceremony.deckHash || ziffleDeckHash),
	      ].join(":");
	      if (!groups.has(groupKey)) {
	        groups.set(groupKey, { ceremony, positions: new Set() });
	      }
	      groups.get(groupKey).positions.add(position);
	    }
	    for (const group of groups.values()) {
	      const positions = [...group.positions];
	      if (positions.length === 0) continue;
	      await collectZiffleRevealTokensBatch(group.ceremony, positions, {
	        ...options,
	        requirements: options.requirements || requirements,
	      });
	    }
	  }, [
		    collectZiffleRevealTokensBatch,
		    currentHiddenCardMetadataForObject,
		    privateDeckManifestForOwner,
		    resolveLocalCryptoPlayerIndex,
	    ziffleCeremonyForOwner,
	    zifflePositionForObjectId,
	    zifflePositionForOriginalSlot,
	  ]);

  const batchedOwnerPublicZiffleOpeningsForRequirements = useCallback(async (
    requirements = [],
    options = {}
	  ) => {
	    const currentGame = gameRef.current;
	    const hasBatchReveal = typeof currentGame?.ziffleRevealCards === "function";
	    const hasSingleReveal = typeof currentGame?.ziffleRevealCard === "function";
	    if (!currentGame || (!hasBatchReveal && !hasSingleReveal)) {
	      return { openings: [], handledRequirements: new Set() };
	    }
	    const localSeat = resolveLocalCryptoPlayerIndex();
	    const manifest = privateDeckManifestForOwner(localSeat);
	    if (!manifest) {
	      return { openings: [], handledRequirements: new Set() };
	    }
	    const progressiveOpeningPreviews =
	      hasSingleReveal && typeof options.onOpeningBuilt === "function";

    const groups = new Map();
	    for (const requirement of requirements || []) {
	      if (String(requirement?.type || "") !== "public_open") continue;
	      if (Number(requirement.owner) !== Number(localSeat)) continue;
	      const objectId = requirement?.objectId ?? requirement?.object_id ?? null;
	      const hiddenMetadata = await currentHiddenCardMetadataForObject(objectId);
	      const objectOrderedPosition = zifflePositionForObjectId(
	        localSeat,
	        objectId
	      );
		      const originalSlotOrderedPosition = zifflePositionForOriginalSlot(localSeat, requirement?.slot);
      const orderedPosition = objectOrderedPosition || originalSlotOrderedPosition;
      const ziffleContext = orderedPosition?.ziffleContext || "";
      const requirementPublicSlot = requirement?.publicSlot ?? requirement?.public_slot ?? null;
	      const requirementPublicCommitment = String(
	        requirement?.publicCommitment || requirement?.public_commitment || ""
	      );
	      const publicPositionIdentity = zifflePublicPositionFromSources(
	        hiddenMetadata,
	        {
	          publicSlot: requirementPublicSlot,
	          publicCommitment: requirementPublicCommitment,
	        }
	      );
	      const publicPosition = publicPositionIdentity?.useAsPosition === false
	        ? null
	        : publicPositionIdentity?.position ?? orderedPosition?.position ?? null;
	      const publicPositionCommitment =
	        publicPositionIdentity?.useAsPosition === false
	          ? ""
	          : publicPositionIdentity?.useAsPosition !== false
	          ? publicPositionIdentity?.positionCommitment || ""
	          : "";
	      const hiddenPositionCommitment =
	        hiddenMetadata?.commitment && ziffleDeckHashFromCommitment(hiddenMetadata.commitment)
	          ? String(hiddenMetadata.commitment)
	          : "";
	      const identityPosition = ziffleIdentityPositionFromSources(
	        hiddenMetadata,
	        {
	          slot: requirement?.slot,
	          commitment: requirement?.commitment,
	        }
	      );
	      const exportedCommitment = String(requirement?.commitment || "");
	      let ziffleCommitment =
	        publicPositionCommitment
	        || identityPosition?.positionCommitment
	        || orderedPosition?.positionCommitment
	        || exportedCommitment;
	      let ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
	      if (!ziffleDeckHash && publicPosition != null && publicPositionCommitment) {
	        ziffleCommitment = publicPositionCommitment;
	        ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
	      }
	      if (!exportedCommitment || ziffleDeckHash) {
	        if (!ziffleDeckHash) {
	          ziffleCommitment = publicPositionCommitment
	            || hiddenPositionCommitment
	            || orderedPosition?.positionCommitment
	            || "";
          ziffleDeckHash = ziffleDeckHashFromCommitment(ziffleCommitment);
        }
      } else {
        ziffleCommitment = "";
        ziffleDeckHash = "";
      }
      if (!ziffleDeckHash) continue;

	      const position = Number(
	        publicPosition
	        ?? zifflePositionFromCommitment(ziffleCommitment)
	        ?? identityPosition?.position
	        ?? (hiddenPositionCommitment ? Number(hiddenMetadata?.slot) : null)
	        ?? requirement?.slot
	      );
      if (!Number.isSafeInteger(position) || position < 0) continue;
      const ceremony = ziffleCeremonyForOwner(localSeat, {
        commitment: ziffleCommitment,
        context: ziffleContext,
      });
      if (!ceremony) continue;
      const positionCommitment = ziffleCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
      const groupKey = [
        Number(localSeat),
        String(ceremony.context || ""),
        String(ceremony.deckHash || ziffleDeckHash),
      ].join(":");
      if (!groups.has(groupKey)) {
        groups.set(groupKey, { ceremony, entries: [] });
      }
	      groups.get(groupKey).entries.push({
	        requirement,
	        position,
	        positionCommitment,
	        publicPositionIdentity,
	      });
    }

	    const openings = [];
	    const handledRequirements = new Set();
	    const expectedOpeningCount = [...groups.values()].reduce(
	      (total, group) => total + (Array.isArray(group.entries) ? group.entries.length : 0),
	      0
	    );
	    const seen = new Set();
	    const addRevealedPublicOpeningForEntry = async ({
	      ceremony,
	      entry,
	      shuffleOriginalSlot,
	      tokens = [],
	    }) => {
	      if (!Number.isSafeInteger(shuffleOriginalSlot) || shuffleOriginalSlot < 0) {
	        throw new Error(`Missing ziffle reveal for position ${Number(entry.position)}`);
	      }
	      let resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
	        owner: localSeat,
	        ceremony,
	        shuffleOriginalSlot,
	        shuffleOriginalSlotIsVerified: true,
	        position: entry.position,
	        card: entry.requirement?.card || "",
	        objectId: entry.requirement?.objectId ?? entry.requirement?.object_id,
	        manifest,
	        options,
	      });
	      if (!resolvedRevealSlot && entry.requirement?.card) {
	        resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
	          owner: localSeat,
	          ceremony,
	          shuffleOriginalSlot,
	          shuffleOriginalSlotIsVerified: true,
	          position: entry.position,
	          card: "",
	          objectId: entry.requirement?.objectId ?? entry.requirement?.object_id,
	          manifest,
	          options,
	        });
	      }
	      if (!resolvedRevealSlot) {
	        const beforeOrder = normalizeShuffleOrder(ceremony.beforeOrder ?? ceremony.before_order);
	        const afterOrder = normalizeShuffleOrder(ceremony.afterOrder ?? ceremony.after_order);
	        const interestingIds = [
	          entry.requirement?.objectId ?? entry.requirement?.object_id,
	          beforeOrder[Number(shuffleOriginalSlot)],
	          afterOrder[Number(entry.position)],
	        ].map((id) => Number(id)).filter((id, index, list) =>
	          Number.isSafeInteger(id) && id >= 0 && list.indexOf(id) === index
	        );
	        let candidateDebug = [];
	        try {
	          const checkpoint = await currentGame.exportSyncCheckpoint?.();
	          const objectsById = new Map((checkpoint?.objects || []).map((object) => [
	            Number(object.id),
	            object,
	          ]));
	          candidateDebug = interestingIds.map((id) => {
	            const object = objectsById.get(id) || {};
	            const hidden = object.hiddenCard || object.hidden_card || {};
	            return {
	              id,
	              name: object.name || object.identity?.name || null,
	              zone: object.zone || null,
	              stableId: object.stableId ?? object.stable_id ?? null,
	              hiddenOwner: hidden.owner ?? null,
	              hiddenSlot: hidden.slot ?? null,
	              hiddenCommitment: String(hidden.commitment || "").slice(0, 32),
	              publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
	              publicCommitment: String(hidden.publicCommitment || hidden.public_commitment || "").slice(0, 32),
	            };
	          });
	        } catch {
	          candidateDebug = [];
	        }
	        throw new Error(
	          `Ziffle opening could not resolve committed slot `
	          + `(owner ${Number(localSeat) + 1}, position ${Number(entry.position)}, `
	          + `shuffle slot ${shuffleOriginalSlot}, card ${String(entry.requirement?.card || "")}, `
	          + `requirement ${JSON.stringify(entry.requirement)}, `
	          + `ids ${JSON.stringify(interestingIds)}, candidates ${JSON.stringify(candidateDebug)})`
	        );
	      }
	      const originalSlot = Number(resolvedRevealSlot.slot);
	      const secret = (manifest.slotSecrets || []).find(
	        (candidate) => Number(candidate.slot) === Number(originalSlot)
	      );
	      if (!secret) {
	        throw new Error(`Missing private deck opening for ziffle slot ${Number(originalSlot)}`);
	      }
	      const opening = await buildDeckSlotOpening({
	        manifest,
	        slot: originalSlot,
	        card: resolvedRevealSlot?.card || secret.card,
	      });
	      const requirementObjectId = Number(entry.requirement?.objectId ?? entry.requirement?.object_id);
	      const resolvedObjectId = resolvedRevealSlot?.objectId == null
	        ? null
	        : Number(resolvedRevealSlot.objectId);
	      const openingObjectId =
	        resolvedObjectId != null && Number.isSafeInteger(resolvedObjectId) && resolvedObjectId >= 0
	          ? resolvedObjectId
	          : null;
	      let openingWithPosition = {
	        ...opening,
	        ...(openingObjectId != null ? { objectId: openingObjectId } : {}),
	        ...(resolvedRevealSlot?.shuffleObjectId != null || resolvedRevealSlot?.objectId != null
	          ? { shuffleObjectId: Number(resolvedRevealSlot.shuffleObjectId ?? resolvedRevealSlot.objectId) }
	          : {}),
	        _debugResolveSource: String(resolvedRevealSlot?.source || ""),
	        _debugShuffleOriginalSlot: Number(shuffleOriginalSlot),
	        _debugResolvedSlot: Number(resolvedRevealSlot?.slot),
	        _debugResolvedObjectId: resolvedRevealSlot?.objectId == null ? null : Number(resolvedRevealSlot.objectId),
	        timing: "post",
	        position: Number(entry.position),
		        positionCommitment: entry.positionCommitment,
		        ziffleContext: ziffleContextFromCeremony(ceremony),
		      };
		      if (entry.publicPositionIdentity) {
		        openingWithPosition = withPinnedPublicZifflePosition(
		          openingWithPosition,
		          entry.publicPositionIdentity
		        );
		      }
		      if (!ziffleCeremonyHasObjectOrder(ceremony)) {
		        openingWithPosition.ziffleReveal = buildZiffleOpeningProof({
	          opening: openingWithPosition,
	          ceremony,
	          position: Number(entry.position),
	          originalSlot,
	          shuffleOriginalSlot,
	          positionCommitment: entry.positionCommitment,
	          tokens: ziffleTokensForPosition(tokens, entry.position),
	          compact: true,
	        });
	      }
	      openingWithPosition = await sanitizeObjectBoundOpening(openingWithPosition);
	      openingWithPosition = await ensureZiffleOpeningProof(openingWithPosition, {
	        ...options,
	        requirements: options.requirements || requirements,
	        skipFreshZiffleOpeningProofVerification: Boolean(openingWithPosition.ziffleReveal),
	      });
	      openingWithPosition = await sanitizeObjectBoundOpening(openingWithPosition);
	      const key = [
	        Number(openingWithPosition.owner),
	        Number(openingWithPosition.slot),
	        openingWithPosition.objectId ?? openingObjectId ?? -1,
	      ].join(":");
	      handledRequirements.add(entry.requirement);
	      if (seen.has(key)) return;
	      rememberLocalRevealedOpening(openingWithPosition, {
	        objectId: openingWithPosition.objectId ?? (
	          Number.isSafeInteger(requirementObjectId) ? requirementObjectId : null
	        ),
	        position: openingWithPosition.position,
	        positionCommitment: openingWithPosition.positionCommitment,
	        ziffleContext: openingWithPosition.ziffleContext,
	      });
	      rememberZiffleOpeningPosition(localSeat, Number(openingWithPosition.slot ?? originalSlot), entry.position);
	      openings.push(openingWithPosition);
	      notifyOpeningBuilt(options, openingWithPosition, {
	        source: "requirement_public_open",
	        requirement: entry.requirement,
	        index: openings.length,
	        total: expectedOpeningCount || requirements.length,
	      });
	      seen.add(key);
	    };
	    for (const { ceremony, entries } of groups.values()) {
	      if (ziffleCeremonyHasObjectOrder(ceremony)) {
	        const positions = [...new Set(entries.map((entry) => Number(entry.position)))].filter(
	          (position) => Number.isSafeInteger(position) && position >= 0
	        );
	        if (positions.length > 0 && !progressiveOpeningPreviews) {
	          await collectZiffleRevealTokensBatch(ceremony, positions, {
	            ...options,
	            requirements: options.requirements || requirements,
	          });
	        }
	        for (const entry of entries) {
	          const requirementObjectId = Number(entry.requirement?.objectId ?? entry.requirement?.object_id);
	          const {
	            resolvedRevealSlot: resolvedZiffleRevealSlot,
	            shuffleOriginalSlot,
	          } = await resolveCommittedSlotForZifflePosition({
	            owner: localSeat,
	            ceremony,
	            position: entry.position,
	            card: entry.requirement?.card || "",
	            objectId: requirementObjectId,
	            manifest,
	            options,
	          });
	          let resolvedRevealSlot = resolvedZiffleRevealSlot;
	          if (!resolvedRevealSlot) {
	            const directSlot = Number(entry.requirement?.slot);
	            const directSecret = (manifest?.slotSecrets || []).find(
	              (secret) => Number(secret?.slot) === directSlot
	            );
	            const expectedCard = String(entry.requirement?.card || "");
	            const linkedShuffleObjectId =
	              ziffleShuffleObjectIdForPosition(ceremony, entry.position)
	              ?? requirementObjectId;
	            const candidate = directSecret
	              ? {
	                owner: localSeat,
	                slot: directSlot,
	                card: String(directSecret.card || expectedCard || ""),
	                commitment: String(directSecret.commitment || ""),
	                objectId:
	                  Number.isSafeInteger(requirementObjectId) && requirementObjectId >= 0
	                    ? requirementObjectId
	                    : null,
	                shuffleObjectId:
	                  Number.isSafeInteger(Number(linkedShuffleObjectId))
	                  && Number(linkedShuffleObjectId) >= 0
	                    ? Number(linkedShuffleObjectId)
	                    : null,
		                position: entry.position,
		                positionCommitment: entry.positionCommitment,
		                ziffleContext: ziffleContextFromCeremony(ceremony),
		                shuffleOriginalSlot,
		                source: "direct_requirement_slot",
	              }
	              : null;
	            const expectedCommitment = String(entry.requirement?.commitment || "");
	            const directRequirementPositionOpening = Boolean(
	              candidate
		              && expectedCommitment
		              && !ziffleDeckHashFromCommitment(expectedCommitment)
		              && String(candidate.commitment || "") === expectedCommitment
		              && ziffleDeckHashFromCommitment(entry.positionCommitment)
		              && (
		                shuffleOriginalSlot == null
		                || Number(candidate.slot) === Number(shuffleOriginalSlot)
		              )
		            );
	            if (
	              candidate
	              && (!expectedCard || String(candidate.card || "") === expectedCard)
	              && (
	                ziffleObjectOrderLinksOpening(ceremony, shuffleOriginalSlot, entry.position, candidate)
	                || directRequirementPositionOpening
	              )
	            ) {
	              resolvedRevealSlot = candidate;
	            }
	          }
	          if (!resolvedRevealSlot) {
	            throw new Error(
	              `Ziffle opening could not resolve committed slot `
	              + `(owner ${Number(localSeat) + 1}, position ${Number(entry.position)}, `
	              + `shuffle slot ${shuffleOriginalSlot ?? "none"}, `
	              + `card ${String(entry.requirement?.card || "")}, `
	              + `requirement ${JSON.stringify(entry.requirement)})`
	            );
	          }
		          const {
		            openingWithPosition,
		            originalSlot,
		            openingObjectId,
		          } = await buildOpeningFromResolvedCommittedSlot({
	            manifest,
	            resolvedRevealSlot,
	            fallbackObjectId:
	              Number.isSafeInteger(requirementObjectId) && requirementObjectId >= 0
	                ? requirementObjectId
	                : null,
	            position: entry.position,
	            positionCommitment: entry.positionCommitment,
		            ceremony,
		            timing: "post",
		          });
			          let finalOpening = entry.publicPositionIdentity
			            ? withPinnedPublicZifflePosition(openingWithPosition, entry.publicPositionIdentity)
			            : openingWithPosition;
			          finalOpening = await sanitizeObjectBoundOpening(finalOpening);
		          finalOpening = await ensureZiffleOpeningProof(finalOpening, {
		            ...options,
		            requirements: options.requirements || requirements,
		          });
		          finalOpening = await sanitizeObjectBoundOpening(finalOpening);
		          const key = [
		            Number(finalOpening.owner),
		            Number(finalOpening.slot),
		            finalOpening.objectId ?? openingObjectId ?? -1,
		          ].join(":");
		          handledRequirements.add(entry.requirement);
		          if (seen.has(key)) continue;
		          rememberLocalRevealedOpening(finalOpening, {
		            objectId: finalOpening.objectId ?? (
		              Number.isSafeInteger(requirementObjectId) ? requirementObjectId : null
		            ),
			            position: finalOpening.position,
			            positionCommitment: finalOpening.positionCommitment,
			            ziffleContext: finalOpening.ziffleContext,
			          });
		          rememberZiffleOpeningPosition(localSeat, Number(finalOpening.slot ?? originalSlot), entry.position);
		          openings.push(finalOpening);
		          notifyOpeningBuilt(options, finalOpening, {
		            source: "requirement_public_open",
		            requirement: entry.requirement,
		            index: openings.length,
		            total: expectedOpeningCount || requirements.length,
		          });
		          seen.add(key);
	        }
	        continue;
	      }
	      const positions = [...new Set(entries.map((entry) => Number(entry.position)))];
	      if (positions.length === 0) continue;
	      const revealInPreviewBatches = progressiveOpeningPreviews && hasBatchReveal;
	      if (revealInPreviewBatches) {
	        for (const chunkEntries of chunkList(entries, ZIFFLE_OPENING_PREVIEW_BATCH_SIZE)) {
	          const chunkPositions = [...new Set(chunkEntries.map((entry) => Number(entry.position)))].filter(
	            (position) => Number.isSafeInteger(position) && position >= 0
	          );
	          if (chunkPositions.length === 0) continue;
	          const tokens = await collectZiffleRevealTokensBatch(ceremony, chunkPositions, {
	            ...options,
	            requirements: options.requirements || requirements,
	          });
	          const reveals = await currentGame.ziffleRevealCards({
	            deckCount: Number(ceremony.deckCount),
	            context: String(ceremony.context || ""),
	            keyContext: ziffleKeyContextForCeremony(ceremony),
	            keys: cloneMultiplayerPayload(ceremony.keys || []),
	            steps: cloneMultiplayerPayload(ceremony.steps || []),
	            cardPositions: chunkPositions,
	            tokens,
	          });
	          const revealByPosition = new Map(
	            (Array.isArray(reveals) ? reveals : []).map((reveal) => [
	              Number(reveal.cardPosition),
	              Number(reveal.originalSlot),
	            ])
	          );
	          for (const entry of chunkEntries) {
	            const shuffleOriginalSlot = revealByPosition.get(Number(entry.position));
	            await addRevealedPublicOpeningForEntry({
	              ceremony,
	              entry,
	              shuffleOriginalSlot,
	              tokens,
	            });
	          }
	        }
	        continue;
	      }
	      const revealIndividually = !hasBatchReveal && hasSingleReveal;
	      if (revealIndividually) {
	        for (const entry of entries) {
	          const position = Number(entry.position);
	          if (!Number.isSafeInteger(position) || position < 0) continue;
	          const tokens = await collectZiffleRevealTokens(ceremony, position, {
	            ...options,
	            requirements: options.requirements || requirements,
	          });
	          const reveal = await currentGame.ziffleRevealCard({
	            deckCount: Number(ceremony.deckCount),
	            context: String(ceremony.context || ""),
	            keyContext: ziffleKeyContextForCeremony(ceremony),
	            keys: cloneMultiplayerPayload(ceremony.keys || []),
	            steps: cloneMultiplayerPayload(ceremony.steps || []),
	            cardPosition: position,
	            tokens,
	          });
	          await addRevealedPublicOpeningForEntry({
	            ceremony,
	            entry,
	            shuffleOriginalSlot: Number(reveal.originalSlot),
	            tokens,
	          });
	        }
	        continue;
	      }
	      const tokens = await collectZiffleRevealTokensBatch(ceremony, positions, {
	        ...options,
	        requirements: options.requirements || requirements,
	      });
      const reveals = await currentGame.ziffleRevealCards({
        deckCount: Number(ceremony.deckCount),
        context: String(ceremony.context || ""),
        keyContext: ziffleKeyContextForCeremony(ceremony),
        keys: cloneMultiplayerPayload(ceremony.keys || []),
        steps: cloneMultiplayerPayload(ceremony.steps || []),
        cardPositions: positions,
        tokens,
      });
      const revealByPosition = new Map(
        (Array.isArray(reveals) ? reveals : []).map((reveal) => [
          Number(reveal.cardPosition),
          Number(reveal.originalSlot),
        ])
	      );
	      for (const entry of entries) {
	        const shuffleOriginalSlot = revealByPosition.get(Number(entry.position));
	        await addRevealedPublicOpeningForEntry({
	          ceremony,
	          entry,
	          shuffleOriginalSlot,
	          tokens,
	        });
	      }
	    }
    return { openings, handledRequirements };
  }, [
    collectZiffleRevealTokensBatch,
    currentHiddenCardMetadataForObject,
    privateDeckManifestForOwner,
	    rememberLocalRevealedOpening,
	    rememberZiffleOpeningPosition,
	    resolveCommittedZiffleRevealSlot,
	    resolveLocalCryptoPlayerIndex,
	    sanitizeObjectBoundOpening,
	    ziffleCeremonyForOwner,
    zifflePositionForObjectId,
    zifflePositionForOriginalSlot,
  ]);

	  const buildLocalRequirementOpeningsForRequirements = useCallback(async (requirements = [], options = {}) => {
      const {
        openings,
        handledRequirements,
      } = await batchedOwnerPublicZiffleOpeningsForRequirements(requirements, options);
	    const remainingRequirements = (requirements || []).filter(
        (requirement) => !handledRequirements.has(requirement)
      );
	    await prefetchZiffleRevealTokensForPublicOpenRequirements(remainingRequirements, options);
	    for (const requirement of requirements || []) {
        if (handledRequirements.has(requirement)) continue;
	      if (String(requirement?.type || "") !== "public_open") continue;
		      const localSeat = resolveLocalCryptoPlayerIndex();
	      if (Number(requirement.owner) !== Number(localSeat)) continue;
	      if (openings.some((opening) => openingMatchesRequirement(opening, requirement))) {
	        handledRequirements.add(requirement);
	        continue;
	      }
	      let opening = (await buildLocalOpeningFromRequirement(requirement, null, options)).opening;
	      opening = await sanitizeObjectBoundOpening(opening);
	      opening = await ensureZiffleOpeningProof(opening, options);
	      opening = await sanitizeObjectBoundOpening(opening);
		      openings.push(opening);
		      notifyOpeningBuilt(options, opening, {
		        source: "requirement_public_open",
		        requirement,
		        index: openings.length,
		        total: requirements.length,
		      });
		    }
	    return openings;
	  }, [
      batchedOwnerPublicZiffleOpeningsForRequirements,
	    buildLocalOpeningFromRequirement,
	    ensureZiffleOpeningProof,
	    prefetchZiffleRevealTokensForPublicOpenRequirements,
	    resolveLocalCryptoPlayerIndex,
	    sanitizeObjectBoundOpening,
	  ]);

	  const buildLocalDeckAuditManifest = useCallback(
    async ({ matchId, owner, deck, sideboard, commanders, persist = true }) => {
      const normalizedMatchId = String(matchId || "");
      const normalizedOwner = Number(owner);
      const normalizedDeck = sanitizeCardList(deck);
      const normalizedSideboard = sanitizeCardList(sideboard);
      const normalizedCommanders = sanitizeCardList(commanders);
      const expectedDecklistHash = await decklistHashForCards({
        matchId: normalizedMatchId,
        owner: normalizedOwner,
        deck: normalizedDeck,
        sideboard: normalizedSideboard,
        commanders: normalizedCommanders,
      });
      const existing = privateDeckManifestForOwner(normalizedOwner, normalizedMatchId);
      if (
        existing?.decklistHash === expectedDecklistHash
        && existing?.decklistCommitment
        && Number(existing.deckCount) === normalizedDeck.length
        && Number(existing.sideboardCount || 0) === normalizedSideboard.length
        && Number(existing.commanderCount || 0) === normalizedCommanders.length
        && Array.isArray(existing.slotSecrets)
        && existing.slotSecrets.length === normalizedDeck.length
      ) {
        if (persist) rememberPrivateDeckManifest(existing);
        return existing;
      }
      const manifest = await buildPrivateDeckManifest({
        matchId: normalizedMatchId,
        owner: normalizedOwner,
        deck: normalizedDeck,
        sideboard: normalizedSideboard,
        commanders: normalizedCommanders,
      });
      if (persist) rememberPrivateDeckManifest(manifest);
      return manifest;
    },
    [privateDeckManifestForOwner, rememberPrivateDeckManifest]
  );

  const currentPublicAuditCheckpointHash = useCallback(async () => {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.exportPublicAuditCheckpoint !== "function") {
      throw new Error("Game engine cannot export a public audit checkpoint");
    }
    return publicCheckpointHash(await currentGame.exportPublicAuditCheckpoint());
  }, []);

  function currentKnownPublicAuditCheckpointHash() {
    const lastSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
    if (lastSequence > 0) {
      return String(actionHistoryEntryForSequence(lastSequence)?.audit?.publicCheckpointHash || "");
    }
    return String(initialPublicCheckpointHashRef.current || "");
  }

  const buildSequencedActionAudit = useCallback(async ({
    seq,
    actorIndex,
    command,
    clock = null,
    openings = [],
    rngReveals = [],
    shuffleProofs = [],
    privateViewProofs = [],
    publicCheckpointHash: providedPublicCheckpointHash = null,
  }) => {
    const { keyPair } = await ensureAuditIdentity();
    const signer = resolveLocalPlayerIndex(multiplayerRef.current);
    const resolvedPublicCheckpointHash = providedPublicCheckpointHash
      || await currentPublicAuditCheckpointHash();
    if (!resolvedPublicCheckpointHash) {
      throw new Error("Game engine cannot export a public audit checkpoint");
    }
    return buildSignedActionEnvelope({
      keyPair,
      matchId: currentAuditMatchId(),
      seq: Number(seq),
      actor: Number(actorIndex),
      signer: Number(signer ?? actorIndex),
      prevStateHash: auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH,
      command: cloneMultiplayerPayload(command),
      clock: cloneMultiplayerPayload(clock),
      openings: cloneMultiplayerPayload(openings),
      rngReveals: cloneMultiplayerPayload(rngReveals),
      shuffleProofs: cloneMultiplayerPayload(shuffleProofs),
      privateViewProofs: cloneMultiplayerPayload(privateViewProofs),
      publicCheckpointHash: resolvedPublicCheckpointHash,
    });
  }, [currentAuditMatchId, currentPublicAuditCheckpointHash, ensureAuditIdentity]);

  const verifyCurrentPublicCheckpointHash = useCallback(async (expectedHash, reason = "Public checkpoint hash mismatch") => {
    const expected = String(expectedHash || "");
    if (!expected) {
      throw new Error("Sequenced audit is missing public checkpoint hash");
    }
    const actual = await currentPublicAuditCheckpointHash();
    if (actual !== expected) {
      throw new Error(reason);
    }
    return actual;
  }, [currentPublicAuditCheckpointHash]);

  const verifySequencedActionAudit = useCallback(
    async ({ audit, seq, actorIndex, command, expectedPrevStateHash = null }) => {
      if (!audit || typeof audit !== "object") {
        throw new Error("Sequenced action is missing its audit signature");
      }
      if (audit.matchId !== currentAuditMatchId()) {
        throw new Error("Sequenced audit action belongs to a different match");
      }
      if (Number(audit.seq) !== Number(seq) || Number(audit.actor) !== Number(actorIndex)) {
        throw new Error("Sequenced audit action does not match broadcast action");
      }
      const expectedPrev = expectedPrevStateHash ?? auditStateHashRef.current;
      if (audit.prevStateHash !== expectedPrev) {
        throw new Error("Sequenced audit hash chain does not match local transcript");
      }
      if (canonicalMultiplayerPayload(audit.command) !== canonicalMultiplayerPayload(command)) {
        throw new Error("Sequenced audit command does not match broadcast action");
      }
      if (!audit.publicCheckpointHash) {
        throw new Error("Sequenced audit is missing public checkpoint hash");
      }
      const computedHash = await auditStateHash({
        matchId: audit.matchId,
        seq: Number(audit.seq),
        prevStateHash: audit.prevStateHash,
        command: audit.command,
        clock: audit.clock,
        openings: audit.openings || [],
        rngReveals: audit.rngReveals || [],
        shuffleProofs: audit.shuffleProofs || [],
        privateViewProofs: audit.privateViewProofs || [],
        publicCheckpointHash: audit.publicCheckpointHash,
      });
      if (computedHash !== String(audit.nextStateHash || "")) {
        throw new Error("Sequenced audit next state hash is invalid");
      }
      await verifyAuditOpeningsAgainstManifests(audit.openings || [], {
        payload: matchStartPayloadRef.current,
        shuffleProofs: audit.shuffleProofs || [],
      });
      const signer = Number(audit.signer ?? audit.actor);
      if (signer !== Number(audit.actor)) {
        throw new Error("Sequenced action must be signed by the acting player");
      }
      const publicKey = await importCachedAuditPublicKey(publicKeyForAuditSigner(signer));
      const payload = {
        matchId: audit.matchId,
        seq: Number(audit.seq),
        actor: Number(audit.actor),
        signer,
        prevStateHash: audit.prevStateHash,
        command: audit.command,
        clock: audit.clock,
        openings: audit.openings || [],
        rngReveals: audit.rngReveals || [],
        shuffleProofs: audit.shuffleProofs || [],
        privateViewProofs: audit.privateViewProofs || [],
        publicCheckpointHash: audit.publicCheckpointHash,
        nextStateHash: audit.nextStateHash,
      };
      const valid = await verifyAuditPayload(publicKey, payload, audit.signature || "");
      if (!valid) {
        throw new Error("Sequenced audit action signature is invalid");
      }
    },
    [
      currentAuditMatchId,
      importCachedAuditPublicKey,
      publicKeyForAuditSigner,
      verifyAuditOpeningsAgainstManifests,
    ]
  );

  const verifyAuditSatisfiesCryptoRequirements = useCallback(async ({ requirements = [], audit = {} }) => {
    for (const requirement of requirements || []) {
      const type = String(requirement?.type || "");
      if (!type || type === "hidden_move" || type === "hidden_order_update") continue;
      if (type === "public_open") {
        const match =
          (audit.openings || []).find((opening) =>
            openingMatchesRequirement(opening, requirement)
          )
          || localRevealedOpeningForRequirement(requirement);
        if (!match) {
          throw new Error(
            `Missing ${type} audit opening for player ${Number(requirement.owner) + 1}: `
            + JSON.stringify({
              requirement: {
                owner: requirement.owner,
                slot: requirement.slot,
                objectId: requirement.objectId ?? requirement.object_id ?? null,
                card: requirement.card,
                commitment: String(requirement.commitment || "").slice(0, 48),
                positionCommitment: String(
                  requirement.positionCommitment || requirement.position_commitment || ""
                ).slice(0, 64),
                publicSlot: requirement.publicSlot ?? requirement.public_slot ?? null,
                publicCommitment: String(
                  requirement.publicCommitment || requirement.public_commitment || ""
                ).slice(0, 64),
              },
              openings: (audit.openings || [])
                .filter((opening) => Number(opening?.owner) === Number(requirement.owner))
                .slice(0, 80)
                .map((opening) => ({
                  slot: opening.slot,
                  objectId: opening.objectId ?? opening.object_id ?? null,
                  shuffleObjectId: opening.shuffleObjectId ?? opening.shuffle_object_id ?? null,
                  card: opening.card,
                  commitment: String(opening.commitment || "").slice(0, 48),
                  position: opening.position ?? null,
                  positionCommitment: String(
                    opening.positionCommitment || opening.position_commitment || ""
                  ).slice(0, 64),
                  publicSlot: opening.publicSlot ?? opening.public_slot ?? null,
                  publicCommitment: String(
                    opening.publicCommitment || opening.public_commitment || ""
                  ).slice(0, 64),
                })),
            })
          );
        }
        continue;
      }
      if (type === "private_open") {
        if (isOwnerPrivateViewRequirement(requirement)) {
          continue;
        }
        const proof = (audit.privateViewProofs || []).find((entry) =>
          String(entry?.requirementId || "") === String(requirement.id || "")
          || (
            Number(entry?.owner) === Number(requirement.owner)
            && Number(entry?.viewer) === Number(requirement.viewer)
            && Number(entry?.objectId) === Number(requirement.objectId)
          )
        );
        if (!proof?.encryptedOpening?.ciphertextHex || !proof?.encryptedOpening?.plaintextHash) {
          throw new Error(
            `Missing encrypted private opening for player ${Number(requirement.owner) + 1}`
          );
        }
        const recipientKey = auditEncryptionPublicKeyForPlayer(requirement.viewer);
        if (
          recipientKey
          && String(proof.encryptedOpening.recipientPublicKey || "") !== recipientKey
        ) {
          throw new Error("Encrypted private opening targets the wrong viewer key");
        }
        if (
          requirement.commitment
          && (proof.positionCommitment || proof.commitment)
          && ![proof.positionCommitment, proof.commitment]
            .filter(Boolean)
            .some((commitment) => String(commitment) === String(requirement.commitment))
        ) {
          throw new Error("Encrypted private opening commitment mismatch");
        }
        continue;
      }
      if (type === "private_view_window" || type === "public_view_window") {
        if (isOwnerPrivateViewRequirement(requirement)) {
          continue;
        }
        const proofList = type === "private_view_window"
          ? (audit.privateViewProofs || []).filter(
              (entry) => String(entry?.type || "") === "encrypted_private_opening"
            )
          : (audit.openings || []);
        const matchingProofs = proofList.filter((entry) =>
          Number(entry.owner) === Number(requirement.owner)
          && (type !== "private_view_window" || Number(entry.viewer) === Number(requirement.viewer))
        );
        let materialCount = matchingProofs.length;
        if (type === "public_view_window") {
          const known = new Map();
          const addKnownOpening = (entry) => {
            if (!entry || Number(entry.owner) !== Number(requirement.owner)) return;
            const key = [
              Number(entry.owner),
              Number(entry.slot ?? -1),
              Number(entry.objectId ?? -1),
              String(entry.commitment || entry.positionCommitment || ""),
              String(entry.card || ""),
            ].join(":");
            known.set(key, entry);
          };
          matchingProofs.forEach(addKnownOpening);
          for (const opening of localRevealedOpeningsRef.current.values()) {
            addKnownOpening(opening);
          }
          materialCount = known.size;
        }
        if (materialCount < Number(requirement.count || 0)) {
          throw new Error(
            `Missing audit material for ${type} of player ${Number(requirement.owner) + 1}`
          );
        }
        continue;
      }
      if (type === "verifiable_shuffle") {
        const proof = (audit.shuffleProofs || []).find((entry) =>
          shuffleProofMatchesRequirement(entry, requirement)
        );
        if (!proof) {
          throw new Error(
            `Missing verifiable shuffle proof for player ${Number(requirement.owner) + 1}`
          );
        }
        continue;
      }
      if (type === "fair_random") {
        const reveal = (audit.rngReveals || []).find(
          (entry) => String(entry?.requirementId || "") === String(requirement.id || "")
        );
        if (!reveal) {
          throw new Error("Missing transcripted fair-random reveal");
        }
        const expectedPlayers = reindexPlayers(
          matchStartPayloadRef.current?.players || multiplayerRef.current.players || []
        ).map((player) => Number(player.index)).sort((left, right) => left - right);
        const commits = Array.isArray(reveal.commits) ? reveal.commits : [];
        const reveals = Array.isArray(reveal.reveals) ? reveal.reveals : [];
        if (commits.length !== expectedPlayers.length || reveals.length !== expectedPlayers.length) {
          throw new Error("Transcripted fair-random reveal must include every player");
        }
        for (let index = 0; index < expectedPlayers.length; index += 1) {
          if (
            Number(commits[index]?.player) !== expectedPlayers[index]
            || Number(reveals[index]?.player) !== expectedPlayers[index]
          ) {
            throw new Error("Transcripted fair-random material must be sorted and complete");
          }
        }
        const seenPlayers = new Set();
        for (const commit of commits) {
          const player = Number(commit?.player);
          if (seenPlayers.has(player)) {
            throw new Error("Transcripted fair-random material contains a duplicate player");
          }
          seenPlayers.add(player);
          await verifyRngCommitmentEntry(commit, {
            matchId: currentAuditMatchId(),
            seq: Number(audit.seq),
            requirementId: String(requirement.id || ""),
            requester: Number(commit?.requester),
            player,
          });
        }
        for (const entry of reveal.reveals || []) {
          const expected = (reveal.commits || []).find(
            (commit) => Number(commit.player) === Number(entry.player)
          );
          const actual = await rngCommitmentForNonce(entry.nonceHex);
          if (
            !expected
            || actual !== String(expected.commitmentHex || "")
            || (entry.commitmentHex && actual !== String(entry.commitmentHex || ""))
          ) {
            throw new Error("Transcripted fair-random reveal does not match commitment");
          }
          await verifyRngRevealEntry(entry, {
            matchId: currentAuditMatchId(),
            seq: Number(audit.seq),
            requirementId: String(requirement.id || ""),
            requester: Number(entry?.requester),
            player: Number(entry?.player),
          });
        }
        const combinedSeedHex = await fairRandomCombinedSeedHex({
          matchId: currentAuditMatchId(),
          seq: Number(audit.seq),
          requirementId: String(requirement.id || ""),
          commits: reveal.commits || [],
          reveals: reveal.reveals || [],
        });
        if (combinedSeedHex !== String(reveal.combinedSeedHex || "")) {
          throw new Error("Transcripted fair-random combined seed is invalid");
        }
      }
    }
  }, [auditEncryptionPublicKeyForPlayer, currentAuditMatchId, localRevealedOpeningForRequirement]);

  function previewAuditOpeningInInspector(opening, latestState = null, options = {}) {
    if (options.previewInspector === false) return;
    if (!opening || !opening.card) return;
    const viewedCards =
      latestState?.viewed_cards
      || latestState?.active_viewed_cards
      || null;
    const fallbackObjectId = Number(opening.objectId ?? opening.object_id);
    const fallbackStableId = Number(opening.stableId ?? opening.stable_id);
    const fallbackCard = {
      id: Number.isSafeInteger(fallbackObjectId) && fallbackObjectId > 0
        ? fallbackObjectId
        : (Number.isSafeInteger(fallbackStableId) ? fallbackStableId : undefined),
      stable_id: Number.isSafeInteger(fallbackStableId) ? fallbackStableId : undefined,
      name: String(opening.card || ""),
      owner: Number(opening.owner),
      controller: Number(opening.owner),
      zone: String(opening.zone || opening.toZone || options.previewZone || "exile"),
      slot: Number.isSafeInteger(Number(opening.slot)) ? Number(opening.slot) : undefined,
    };
    const nextViewedCards = {
      ...(viewedCards || {}),
      visibility: viewedCards?.visibility || "public",
      subject: viewedCards?.subject ?? Number(opening.owner),
      zone: viewedCards?.zone || fallbackCard.zone,
      description: `Revealing ${String(opening.card || "")}`,
      cards: [fallbackCard],
      card_ids: [],
      inspector_only: true,
      transient_inspector_preview: true,
    };
    const previewIndex = Number(options.previewIndex);
    const previewTotal = Number(options.previewTotal);
    updateMultiplayer((prev) => {
      if (!prev.peerWait) return prev;
      return {
        ...prev,
        peerWait: {
          ...prev.peerWait,
          operation: "Opening revealed card",
          cardName: String(opening.card || ""),
          zone: String(nextViewedCards.zone || fallbackCard.zone || ""),
          openingPreviews: mergeActionOpeningPreviews(
            prev.peerWait.openingPreviews || [],
            [fallbackCard]
          ),
          progressCurrent:
            Number.isSafeInteger(previewIndex) && Number.isSafeInteger(previewTotal) && previewTotal > 0
              ? Math.min(previewTotal, previewIndex + 1)
              : prev.peerWait.progressCurrent,
          progressTotal:
            Number.isSafeInteger(previewTotal) && previewTotal > 0
              ? previewTotal
              : prev.peerWait.progressTotal,
        },
      };
    });
    const base = stateRef.current || latestState || {};
    setState({
      ...base,
      viewed_cards: nextViewedCards,
    });
  }

  const revealAuditOpenings = useCallback(async (openings = [], options = {}) => {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.revealHiddenSlot !== "function") return;
    const timing = options.timing || null;
    const commandObjectIds = timing === "pre"
      ? collectCommandObjectIds(options.command, new Set(), options.uiState || stateRef.current)
      : new Set();
    if (timing === "pre") {
      await addResolvedSelectObjectCommandIds(
        commandObjectIds,
        options.command,
        options.uiState || stateRef.current
      );
    }
    if (timing === "post" && Array.isArray(openings) && openings.length > 0) {
      try {
        const checkpoint = await currentGame.exportSyncCheckpoint?.();
        const inconsistent = (checkpoint?.objects || []).filter((object) => {
          const hidden = object?.hiddenCard || object?.hidden_card || null;
          if (!hidden) return false;
          const commitment = String(hidden.commitment || "");
          const position = zifflePositionFromCommitment(commitment);
          return position != null
            && Number(hidden.slot) !== Number(position)
            && (hidden.publicSlot ?? hidden.public_slot) == null;
        }).slice(0, 8).map((object) => {
          const hidden = object.hiddenCard || object.hidden_card || {};
          return {
            id: object.id,
            name: object.name,
            zone: object.zone,
            slot: hidden.slot,
            commitment: hidden.commitment,
            publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
          };
        });
        if (inconsistent.length > 0) {
          throw new Error(`pre-post-open hidden ziffle inconsistency ${JSON.stringify(inconsistent)}`);
        }
      } catch (err) {
        if (String(err?.message || err || "").includes("pre-post-open hidden ziffle inconsistency")) {
          throw err;
        }
      }
    }
    let changed = false;
    let latestState = null;
    const debugProcessedPostOpenings = [];
    const openingList = Array.isArray(openings) ? openings : [];
	    for (const [openingIndex, rawOpening] of openingList.entries()) {
	      let opening = await sanitizeObjectBoundOpening(rawOpening);
	      if (!opening || opening.owner == null || opening.slot == null || !opening.card) {
	        continue;
	      }
	      if (Number(opening.owner) === Number(resolveLocalCryptoPlayerIndex())) {
	        opening = await ensureZiffleOpeningProof(opening, options);
	        opening = await sanitizeObjectBoundOpening(opening);
	      }
      const opensCommandObject =
        opening.objectId != null && commandObjectIds.has(Number(opening.objectId));
      const recomputeDecision = Boolean(timing === "pre" && opensCommandObject);
      if (timing && String(opening.timing || "pre") !== timing && !opensCommandObject) {
        continue;
      }
	      let localHiddenMetadata = null;
	      let debugOpeningEntry = null;
	      try {
	        await verifyAuditOpeningsAgainstManifests([opening], options);
	        const openingObjectId = opening.objectId == null ? null : Number(opening.objectId);
	        let localRevealObjectId =
	          Number.isSafeInteger(openingObjectId) && openingObjectId > 0
	            ? openingObjectId
	            : null;
		        localHiddenMetadata = opening.objectId != null
		          ? await currentHiddenCardMetadataForObject(opening.objectId)
		          : null;
		        const openingPositionCommitment = String(opening.positionCommitment || "");
		        if (
		          localHiddenMetadata
		          && opening.position != null
		          && ziffleDeckHashFromCommitment(openingPositionCommitment)
		          && !hiddenMetadataMatchesZifflePosition(
		            localHiddenMetadata,
		            opening.position,
		            openingPositionCommitment
		          )
		        ) {
		          localRevealObjectId = null;
		          localHiddenMetadata = null;
		        }
		        if (
		          localHiddenMetadata
		          && opening.position != null
		          && ziffleDeckHashFromCommitment(openingPositionCommitment)
		          && Number(localHiddenMetadata.slot) !== Number(opening.slot)
		        ) {
		          localRevealObjectId = null;
		          localHiddenMetadata = null;
		        }
		        let checkpoint = null;
		        let ignoredExplicitObjectIdentity = false;
		        if (
		          localHiddenMetadata
		          && openingObjectId != null
		          && opening.card
		          && typeof currentGame.exportSyncCheckpoint === "function"
		        ) {
		          try {
		            checkpoint = await currentGame.exportSyncCheckpoint();
		          } catch {
		            checkpoint = null;
		          }
		          const explicitObject = checkpointObjectForId(checkpoint, openingObjectId);
		          if (
		            explicitObject
		            && !checkpointObjectIsRedactedHidden(explicitObject)
		            && checkpointObjectName(explicitObject)
		            && checkpointObjectName(explicitObject) !== String(opening.card || "").trim()
		          ) {
		            ignoredExplicitObjectIdentity = true;
		            localRevealObjectId = null;
		            localHiddenMetadata = null;
		          }
		        }
		        debugOpeningEntry = {
	          slot: Number(opening.slot),
	          objectId: opening.objectId == null ? null : Number(opening.objectId),
	          card: String(opening.card || ""),
	          position: opening.position == null ? null : Number(opening.position),
	          positionCommitment: String(opening.positionCommitment || "").slice(0, 32),
	          source: String(opening._debugResolveSource || ""),
	          hiddenSlot: localHiddenMetadata?.slot == null ? null : Number(localHiddenMetadata.slot),
	          hiddenCommitment: String(localHiddenMetadata?.commitment || "").slice(0, 32),
	          hiddenPublicSlot: localHiddenMetadata?.publicSlot == null ? null : Number(localHiddenMetadata.publicSlot),
	        };
		        let explicitObjectPresent = false;
		        let explicitObjectExistsWithoutHidden = false;
	        if (!localHiddenMetadata && typeof currentGame.exportSyncCheckpoint === "function") {
	          if (!checkpoint) {
	            try {
	              checkpoint = await currentGame.exportSyncCheckpoint();
	            } catch {
	              checkpoint = null;
	            }
	          }
		          const explicitObject = checkpointObjectForId(checkpoint, openingObjectId);
		          explicitObjectPresent = Boolean(explicitObject);
		          explicitObjectExistsWithoutHidden = Boolean(
		            explicitObject && !checkpointObjectHiddenCard(explicitObject)
		          );
	          if (knownCheckpointObjectMatchesOpening(explicitObject, opening)) {
	            rememberLocalRevealedOpening(opening, {
	              objectId: openingObjectId,
	              position: opening.position,
	              positionCommitment: opening.positionCommitment,
	            });
	            continue;
	          }
	          if (
	            explicitObject
	            && !checkpointObjectHiddenCard(explicitObject)
	            && !ignoredExplicitObjectIdentity
	            && opening.card
	            && checkpointObjectName(explicitObject) !== String(opening.card || "").trim()
	          ) {
	            throw new Error(
	              `opened object identity does not match reveal `
	              + `(object ${Number(openingObjectId)} is ${checkpointObjectName(explicitObject) || "unknown"}, `
	              + `opening is ${String(opening.card || "")})`
	            );
	          }
	          const resolvedObjectId = hiddenObjectIdForOpeningFromCheckpoint(checkpoint, opening);
	          if (resolvedObjectId != null) {
	            const resolvedMetadata = hiddenCardMetadataForObjectFromCheckpoint(
	              checkpoint,
	              resolvedObjectId
	            );
	            if (resolvedMetadata) {
	              localRevealObjectId = resolvedObjectId;
	              localHiddenMetadata = resolvedMetadata;
		            }
			          }
			        }
			        const localOwnerAlreadyKnowsObjectOpening = Boolean(
			          timing === "post"
			          && Number(opening.owner) === Number(resolveLocalCryptoPlayerIndex())
			          && opening.objectId != null
			          && localHiddenMetadata
			          && !openingHasZifflePosition(opening)
			          && Number(localHiddenMetadata.owner) === Number(opening.owner)
			          && Number(localHiddenMetadata.slot) === Number(opening.slot)
			          && opening.commitment
			          && !ziffleDeckHashFromCommitment(opening.commitment)
			          && (
			            String(localHiddenMetadata.commitment || "") === String(opening.commitment || "")
			            || String(localHiddenMetadata.publicCommitment || "") === String(opening.commitment || "")
			          )
			        );
			        if (localOwnerAlreadyKnowsObjectOpening) {
			          rememberLocalRevealedOpening(opening, {
			            objectId: localRevealObjectId ?? opening.objectId,
			          });
			          previewAuditOpeningInInspector(opening, latestState, {
			            ...options,
			            previewIndex: openingIndex,
			            previewTotal: openingList.length,
			          });
			          if (debugOpeningEntry) {
			            debugProcessedPostOpenings.push({ ...debugOpeningEntry, status: "ok" });
			            if (debugProcessedPostOpenings.length > 10) {
			              debugProcessedPostOpenings.shift();
			            }
			          }
			          continue;
			        }
			        if (
			          !localHiddenMetadata
			          && opening.position != null
			          && ziffleDeckHashFromCommitment(openingPositionCommitment)
			        ) {
		          localRevealObjectId = null;
		        }
			        const localHiddenZiffleCommitment =
			          localHiddenMetadata?.commitment
			          && ziffleDeckHashFromCommitment(localHiddenMetadata.commitment)
	            ? String(localHiddenMetadata.commitment)
	            : "";
	        const openingPublicZiffleCommitment = String(
	          opening.publicCommitment || opening.public_commitment || ""
	        );
	        const openingPublicZifflePosition =
	          zifflePositionFromCommitment(openingPublicZiffleCommitment)
	          ?? (
	            ziffleDeckHashFromCommitment(openingPublicZiffleCommitment)
	            && opening.publicSlot != null
	              ? Number(opening.publicSlot)
	              : opening.public_slot != null
	                ? Number(opening.public_slot)
	                : null
	          );
	        const revealPosition =
	          opening.position != null
	            ? Number(opening.position)
	            : localHiddenZiffleCommitment && localHiddenMetadata?.slot != null
	              ? Number(localHiddenMetadata.slot)
	              : openingPublicZifflePosition;
		        const revealPositionCommitment =
		          String(opening.positionCommitment || "")
		          || localHiddenZiffleCommitment
		          || openingPublicZiffleCommitment;
	        const isZifflePositionReveal = Boolean(
	          revealPosition != null
	          && ziffleDeckHashFromCommitment(revealPositionCommitment)
	        );
	        if (isZifflePositionReveal && localRevealObjectId != null) {
	          const metadataMatchesReveal =
	            localHiddenMetadata
	            && hiddenMetadataMatchesZifflePosition(
	              localHiddenMetadata,
	              revealPosition,
	              revealPositionCommitment
	            );
	          const metadataMatchesOriginalSlot =
	            localHiddenMetadata
	            && localHiddenMetadata.slot != null
	            && Number(localHiddenMetadata.slot) === Number(opening.slot);
	          if (!metadataMatchesReveal || !metadataMatchesOriginalSlot) {
	            localRevealObjectId = null;
	            localHiddenMetadata = null;
	          }
	        }
	        const revealByObjectMetadata = async () => {
	          if (
	            localRevealObjectId == null
	            || !localHiddenMetadata
	            || typeof currentGame.revealHiddenObject !== "function"
	          ) {
            return null;
	          }
	          const metadataSlot = Number(localHiddenMetadata.slot);
	          const metadataCommitment = String(localHiddenMetadata.commitment || "");
	          const metadataPublicSlot = localHiddenMetadata.publicSlot == null
	            ? null
	            : Number(localHiddenMetadata.publicSlot);
	          const metadataPublicCommitment = String(localHiddenMetadata.publicCommitment || "");
	          const matchesOriginal =
	            Number.isSafeInteger(metadataSlot)
	            && metadataSlot === Number(opening.slot)
	            && (
	              !opening.commitment
	              || metadataCommitment === String(opening.commitment || "")
	              || metadataPublicCommitment === String(opening.commitment || "")
	            );
	          const matchesPosition =
	            revealPosition != null
	            && (
	              metadataSlot === Number(revealPosition)
	              || metadataPublicSlot === Number(revealPosition)
	            )
	            && (
	              !revealPositionCommitment
	              || metadataCommitment === revealPositionCommitment
	              || metadataPublicCommitment === revealPositionCommitment
	            );
          if (!matchesOriginal && !matchesPosition) {
            return null;
	          }
	          return currentGame.revealHiddenObject({
	            objectId: Number(localRevealObjectId),
	            slot: metadataSlot,
	            cardName: String(opening.card),
	            commitment: metadataCommitment || undefined,
            recomputeDecision,
          });
        };
        const revealByCommittedSlot = () => currentGame.revealHiddenSlot({
          owner: Number(opening.owner),
          slot: Number(opening.slot),
          cardName: String(opening.card),
          commitment: opening.commitment || undefined,
          recomputeDecision,
        });
        if (
          revealPosition != null
          && typeof currentGame.revealHiddenPosition === "function"
        ) {
          const ceremony = ziffleCeremonyForOwner(opening.owner, {
            commitment: revealPositionCommitment,
            context: ziffleContextFromOpening(opening),
          });
          try {
            latestState = await currentGame.revealHiddenPosition({
              owner: Number(opening.owner),
              ...(localRevealObjectId != null ? { objectId: Number(localRevealObjectId) } : {}),
              position: Number(revealPosition),
              originalSlot: Number(opening.slot),
              cardName: String(opening.card),
              positionCommitment: revealPositionCommitment
                || (ceremony
                  ? ziffleRuntimeCommitment(ceremony.deckHash, revealPosition)
                  : undefined),
              commitment: opening.commitment || undefined,
              recomputeDecision,
            });
          } catch (err) {
            const message = String(err?.message || err || "");
            if (!message.includes("not present") && !message.includes("not a hidden")) {
              throw err;
            }
            latestState = await revealByObjectMetadata();
            if (!latestState) {
              const metadataSlot = localHiddenMetadata?.slot == null
                ? null
                : Number(localHiddenMetadata.slot);
              const metadataCommitment = String(localHiddenMetadata?.commitment || "");
              const canRevealByCommittedSlot =
                (
                  metadataSlot === Number(opening.slot)
                  && (!opening.commitment || metadataCommitment === String(opening.commitment || ""))
                )
	                || (
	                  !localHiddenMetadata
	                  && !explicitObjectExistsWithoutHidden
	                  && !isZifflePositionReveal
	                  && Boolean(opening.commitment || opening.positionCommitment)
	                );
              if (
                canRevealByCommittedSlot
              ) {
                latestState = await revealByCommittedSlot();
              } else {
                throw err;
              }
            }
          }
        } else {
	          latestState = await revealByObjectMetadata();
		          const hasSpecificOpeningObject = opening.objectId !== null && opening.objectId !== undefined;
		          const explicitObjectMissing =
		            hasSpecificOpeningObject
		            && checkpoint
		            && !explicitObjectPresent;
		          const mayRevealCommittedSlot =
		            !explicitObjectExistsWithoutHidden
		            && (!hasSpecificOpeningObject || localHiddenMetadata || explicitObjectMissing)
		            && (
		              opening.objectId == null
		              || localHiddenMetadata
		              || explicitObjectMissing
		              || (!hasSpecificOpeningObject && (opening.commitment || opening.positionCommitment))
		            );
          if (!latestState && mayRevealCommittedSlot) {
            latestState = await revealByCommittedSlot();
          }
	        }
	        rememberLocalRevealedOpening(opening, {
	          objectId: localRevealObjectId ?? opening.objectId,
	          position: revealPosition,
	          positionCommitment: revealPositionCommitment,
	        });
        previewAuditOpeningInInspector(opening, latestState, {
          ...options,
          previewIndex: openingIndex,
          previewTotal: openingList.length,
        });
	        if (debugOpeningEntry) {
	          debugProcessedPostOpenings.push({ ...debugOpeningEntry, status: "ok" });
	          if (debugProcessedPostOpenings.length > 10) {
	            debugProcessedPostOpenings.shift();
	          }
	        }
        changed = true;
      } catch (err) {
        const message = String(err?.message || err || "");
        if (
          opensCommandObject
          || (!message.includes("not present") && !message.includes("not a hidden"))
        ) {
          throw new Error(
            `${message}; opening owner ${Number(opening.owner)} slot ${Number(opening.slot)}`
            + ` object ${opening.objectId == null ? "none" : Number(opening.objectId)}`
            + ` card ${String(opening.card || "")}`
            + ` resolveSource ${String(opening._debugResolveSource || "none")}`
            + ` shuffleOriginalSlot ${opening._debugShuffleOriginalSlot == null ? "none" : Number(opening._debugShuffleOriginalSlot)}`
            + ` resolvedSlot ${opening._debugResolvedSlot == null ? "none" : Number(opening._debugResolvedSlot)}`
            + ` resolvedObject ${opening._debugResolvedObjectId == null ? "none" : Number(opening._debugResolvedObjectId)}`
            + ` commitment ${String(opening.commitment || "").slice(0, 24) || "none"}`
            + ` position ${opening.position == null ? "none" : Number(opening.position)}`
            + ` positionCommitment ${String(opening.positionCommitment || "").slice(0, 32) || "none"}`
            + ` hiddenSlot ${localHiddenMetadata?.slot == null ? "none" : Number(localHiddenMetadata.slot)}`
            + ` hiddenCommitment ${String(localHiddenMetadata?.commitment || "").slice(0, 32) || "none"}`
            + ` hiddenPublicSlot ${localHiddenMetadata?.publicSlot == null ? "none" : Number(localHiddenMetadata.publicSlot)}`
            + ` hiddenPublicCommitment ${String(localHiddenMetadata?.publicCommitment || "").slice(0, 32) || "none"}`
            + ` processed ${JSON.stringify(debugProcessedPostOpenings)}`
          );
        }
      }
    }
    if (changed && options.updateState !== false) {
      const nextState = latestState || await currentGame.uiState();
      stateRef.current = nextState;
      setState(nextState);
    }
    return latestState;
  }, [
	    currentHiddenCardMetadataForObject,
	    rememberLocalRevealedOpening,
	    sanitizeObjectBoundOpening,
	    setState,
    verifyAuditOpeningsAgainstManifests,
    ziffleCeremonyForOwner,
  ]);

  async function previewRequirementsForCommand(command) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.previewCryptoRequirements !== "function") {
      return [];
    }
    // cancel_decision / forfeit_player are not engine decision commands: routing
    // them through previewCryptoRequirements (which dispatches the command) would
    // throw "invalid command payload: unknown variant ...". They never produce
    // hidden-card material, so they have no crypto requirements.
    if (isNonDispatchSyncCommand(command)) {
      return [];
    }
    const requirements = await currentGame.previewCryptoRequirements(command);
    return Array.isArray(requirements) ? requirements : [];
  }

  async function currentHiddenObjectIdForOpening(opening) {
    if (!opening || opening.owner == null) return null;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.exportSyncCheckpoint !== "function") {
      return null;
    }
    let checkpoint = null;
    try {
      checkpoint = await currentGame.exportSyncCheckpoint();
    } catch {
      return null;
    }
    return hiddenObjectIdForOpeningFromCheckpoint(checkpoint, opening);
  }


  return { addResolvedSelectObjectCommandIds, batchedOwnerPublicZiffleOpeningsForRequirements, buildDeckSlotOpeningForExport, buildLocalDeckAuditManifest, buildLocalOpeningFromRequirement, buildLocalOpeningsForCommand, buildLocalRequirementOpeningsForRequirements, buildOpeningFromResolvedCommittedSlot, buildSequencedActionAudit, currentHiddenCardMetadataForObject, currentHiddenObjectIdForOpening, currentKnownPublicAuditCheckpointHash, currentPublicAuditCheckpointHash, localizeSelectObjectOpeningIds, prefetchZiffleRevealTokensForPublicOpenRequirements, previewAuditOpeningInInspector, previewRequirementsForCommand, resolveCommittedSlotForZifflePosition, resolveCommittedZiffleRevealSlot, revealAuditOpenings, sanitizeObjectBoundOpening, verifiedAuditOpeningKey, verifyAuditOpeningsAgainstManifests, verifyAuditSatisfiesCryptoRequirements, verifyCurrentPublicCheckpointHash, verifySequencedActionAudit };
}
