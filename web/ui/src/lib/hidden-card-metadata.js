export function hiddenCardMetadataForObjectFromCheckpoint(checkpoint, objectId) {
  const normalized = Number(objectId);
  if (!Number.isSafeInteger(normalized) || normalized < 0) return null;
  const object = (checkpoint?.objects || []).find(
    (entry) => Number(entry?.id) === normalized
  );
  return hiddenCardMetadataForObject(object);
}

function hiddenCardMetadataForObject(object) {
  const normalized = Number(object?.id);
  const hidden = object?.hiddenCard || object?.hidden_card || null;
  if (!hidden) return null;
  return {
    objectId: normalized,
    owner: hidden.owner == null ? null : Number(hidden.owner),
    zone: String(object?.zone || ""),
    slot: hidden.slot == null ? null : Number(hidden.slot),
    commitment: String(hidden.commitment || ""),
    publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
    publicCommitment: String(hidden.publicCommitment || hidden.public_commitment || ""),
    originSlot: hidden.originSlot ?? hidden.origin_slot ?? null,
    originCommitment: String(hidden.originCommitment || hidden.origin_commitment || ""),
  };
}

// This projection is request-local: never retain metadata across game mutations.
export function hiddenCardMetadataAtPositionFromCheckpoint(checkpoint, owner, position, commitment) {
  const result = [];
  for (const object of checkpoint?.objects || []) {
    const metadata = hiddenCardMetadataForObject(object);
    if (!metadata || Number(metadata.owner) !== Number(owner)) continue;
    if (String(metadata.publicCommitment || metadata.commitment || "") !== String(commitment)
      || Number(metadata.publicSlot ?? metadata.slot) !== Number(position)) continue;
    result.push(metadata);
  }
  return result;
}
