export function needsFullStateResync({
  forceCheckpoint = false,
  connected = true,
  requesterSequence = 0,
  hostSequence = 0,
} = {}) {
  if (forceCheckpoint === true || connected === false) return true;
  const requester = Number(requesterSequence);
  const host = Number(hostSequence);
  if (!Number.isSafeInteger(requester) || !Number.isSafeInteger(host)) return true;
  return requester < 0 || host < 0 || requester !== host;
}

export function localActionsForRelayRepair(actions, localPlayerIndex) {
  const player = Number(localPlayerIndex);
  if (!Number.isSafeInteger(player) || !Array.isArray(actions)) return [];
  // Preserve transcript order. Recovery is exceptional work, so O(n) once is
  // preferable to an arbitrary tail cap that can permanently lose long chains.
  return actions.filter((entry) => Number(entry?.actorIndex) === player);
}
