import { EMPTY_ACTION_PREFIX } from '../accepted-actions.js';
export function matchingActionPrefix(actions, sequence, prefixHash) {
  return Number.isSafeInteger(sequence) && sequence >= 0 && sequence <= actions.length
    && prefixHash === (sequence === 0 ? EMPTY_ACTION_PREFIX : actions[sequence - 1]?.prefixHash);
}
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
