import { ziffleOriginAnchorFromMetadata } from './multiplayer-audit.js';

export function ziffleDisclosureDueForPlayer(state, owner) {
  const player = (state?.players || []).find(entry => Number(entry?.id ?? entry?.index) === Number(owner));
  return Boolean(state?.game_over || player?.has_lost || player?.hasLost || player?.has_left_game || player?.hasLeftGame);
}

// Requirements must come from this engine's endOfMatchDisclosureRequirements,
// which includes departed snapshots and durable library anchors. A claimed
// origin is never used to choose a record.
export function findZiffleDisclosureOrigin({ opening, state, requirements = [] }) {
  const owner = Number(opening?.owner);
  if (!Number.isSafeInteger(owner) || owner < 0 || !ziffleDisclosureDueForPlayer(state, owner)) return null;
  const commitment = String(opening?.positionCommitment || opening?.position_commitment || '');
  const match = /^ziffle:(.+):(\d+)$/.exec(commitment);
  if (!match) return null;
  const position = Number(match[2]);
  if (!Number.isSafeInteger(position) || position < 0
    || (opening?.position != null && Number(opening.position) !== position)) return null;
  const matches = new Map();
  for (const requirement of requirements || []) {
    if (Number(requirement?.owner) !== owner || String(requirement?.type || '') !== 'public_open') continue;
    const currentCommitment = String(requirement.publicCommitment || requirement.public_commitment || requirement.commitment || '');
    const currentPosition = requirement.publicSlot ?? requirement.public_slot ?? requirement.slot;
    if (currentCommitment !== commitment || currentPosition == null || Number(currentPosition) !== position) continue;
    const origin = ziffleOriginAnchorFromMetadata(requirement);
    if (!origin) continue;
    const objectId = requirement.objectId ?? requirement.object_id ?? null;
    matches.set(origin.originPositionCommitment, {
      ...origin,
      objectId: objectId == null ? null : Number(objectId),
      metadata: { ...requirement, owner, publicSlot: position, publicCommitment: commitment },
    });
  }
  if (matches.size > 1) throw new Error('Disclosure position has ambiguous immutable origin metadata');
  return matches.values().next().value || null;
}
