export function needsFullStateResync({
  force = false,
  connected = true,
  requesterSequence = 0,
  hostSequence = 0,
} = {}) {
  if (force === true || connected === false) return true;
  const requester = Number(requesterSequence);
  const host = Number(hostSequence);
  if (!Number.isSafeInteger(requester) || !Number.isSafeInteger(host)) return true;
  return requester < host;
}
