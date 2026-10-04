// Durable, non-speculative commitments for one disclosed payment command.
// Only verified/signed payment producers may pin an entry. Engine savepoints
// and crypto previews deliberately never own or rewind this journal.
const PREFIX = 'ironsmith.payment-disclosure.v1:';
const MAX_BYTES = 2 * 1024 * 1024;
function clone(value) { return JSON.parse(JSON.stringify(value)); }
function canonical(value) {
  if (Array.isArray(value)) return '[' + value.map(canonical).join(',') + ']';
  if (value && typeof value === 'object') return '{' + Object.keys(value).sort()
    .filter(key => value[key] !== undefined).map(key => JSON.stringify(key) + ':' + canonical(value[key])).join(',') + '}';
  return JSON.stringify(value);
}
function scope(value) {
  const result = { matchId: String(value?.matchId || ''), seq: Number(value?.seq),
    actorIndex: Number(value?.actorIndex ?? value?.actor), prevStateHash: String(value?.prevStateHash || '') };
  if (!result.matchId || !Number.isSafeInteger(result.seq) || result.seq < 1
    || !Number.isSafeInteger(result.actorIndex) || result.actorIndex < 0 || !result.prevStateHash) {
    throw new Error('Invalid payment disclosure commitment scope');
  }
  return result;
}
function key(value) { const s = scope(value); return canonical(s); }
function signedIntentFingerprint(value) {
  // The domain is fixed/verified by the signature layer. A new signature over
  // this identical payload is harmless; a new attempt or pre-state is not.
  return canonical({ ...scope(value), command: value.command,
    attemptId: String(value.attemptId || ''),
    preActionPublicCheckpointHash: String(value.preActionPublicCheckpointHash || value.publicCheckpointHash || '') });
}
function hasSignedIntentIdentity(value) {
  return Boolean(value?.signature || value?.attemptId || value?.preActionPublicCheckpointHash);
}
function retainedSignedIntent(entry) {
  const candidates = [entry?.signedIntent, entry?.timing?.intent, entry?.evidence?.actionIntent].filter(Boolean);
  if (candidates.some(candidate => signedIntentFingerprint(candidate) !== signedIntentFingerprint(candidates[0]))) {
    throw new Error('Disclosed payment has conflicting signed attempt evidence');
  }
  return candidates[0] || null;
}

// Call before accepting disclosure state from a peer or restoring it. The
// actor is the current decision owner, which may differ from a payment payer.
export function assertPaymentDisclosureAuthority(intent, head) {
  const incoming = scope(intent);
  const player = head?.decisionPlayer;
  if (incoming.matchId !== String(head?.matchId || '')
    || incoming.seq !== Number(head?.lastAppliedSequence) + 1
    || incoming.prevStateHash !== String(head?.prevStateHash || '')) {
    throw new Error('Payment disclosure is not based on the current accepted head');
  }
  if (player == null || !Number.isSafeInteger(Number(player)) || Number(player) < 0
    || incoming.actorIndex !== Number(player)) {
    throw new Error('Payment disclosure actor is not the current decision player');
  }
}
function mergeTiming(existing, incoming) {
  if (!existing && !incoming) return null;
  const next = { ...existing, ...incoming };
  const first = [existing?.firstObservedAtMs, incoming?.firstObservedAtMs]
    .map(Number).filter(value => Number.isFinite(value) && value > 0);
  if (!first.length) throw new Error('Disclosed payment lacks its original observation time');
  next.firstObservedAtMs = Math.min(...first);
  const elapsed = [existing?.observedElapsedAtIntentMs, incoming?.observedElapsedAtIntentMs]
    .filter(value => value != null).map(Number).filter(value => Number.isFinite(value) && value >= 0);
  next.observedElapsedAtIntentMs = elapsed.length ? Math.max(...elapsed) : null;
  next.intent = existing?.intent || incoming?.intent || null;
  return clone(next);
}
export function createPaymentDisclosureJournal(getStorage) {
  function storage() {
    const value = typeof getStorage === 'function' ? getStorage() : getStorage;
    if (!value || typeof value.getItem !== 'function' || typeof value.setItem !== 'function') {
      throw new Error('Payment recovery storage is unavailable; cannot safely publish or retry payment choices');
    }
    return value;
  }
  function read(matchId) {
    const raw = storage().getItem(PREFIX + String(matchId));
    if (!raw) return [];
    const entries = JSON.parse(raw);
    if (!Array.isArray(entries) || entries.some(entry => scope(entry).matchId !== String(matchId)
      || !entry.command || typeof entry.command !== 'object')) {
      throw new Error('Payment disclosure recovery record is invalid');
    }
    for (const entry of entries) retainedSignedIntent(entry);
    return entries;
  }
  function write(matchId, entries) {
    const encoded = JSON.stringify(entries);
    if (encoded.length > MAX_BYTES) throw new Error('Payment disclosure recovery record is too large to publish safely');
    const target = storage();
    target.setItem(PREFIX + String(matchId), encoded);
    if (target.getItem(PREFIX + String(matchId)) !== encoded) throw new Error('Payment disclosure recovery record was not retained');
  }
  function lookup(intent) { return read(scope(intent).matchId).find(entry => key(entry) === key(intent)) || null; }
  function assertCompatible(intent) {
    const incoming = scope(intent);
    const existing = read(incoming.matchId).find(entry => entry.seq === incoming.seq) || null;
    if (existing && key(existing) !== key(incoming)) {
      throw new Error('Disclosed payment actor or accepted prefix changed; recover its original payment');
    }
    if (existing && canonical(existing.command) !== canonical(intent.command)) {
      throw new Error('A disclosed payment is waiting for this exact command to be retried; its choices cannot be replaced');
    }
    const retained = retainedSignedIntent(existing);
    if (retained && hasSignedIntentIdentity(intent)
      && signedIntentFingerprint(retained) !== signedIntentFingerprint(intent)) {
      throw new Error('A disclosed payment must retry its original signed attempt and pre-action checkpoint');
    }
    return existing;
  }
  return {
    lookup,
    assertCompatible,
    entries: matchId => clone(read(matchId)),
    pin(intent, { openings = [], evidence = null, timing = null } = {}) {
      const existing = assertCompatible(intent);
      const entries = read(scope(intent).matchId);
      const candidates = [retainedSignedIntent(existing), evidence?.actionIntent, timing?.intent,
        hasSignedIntentIdentity(intent) ? intent : null].filter(Boolean);
      const signedIntent = candidates[0] || null;
      for (const candidate of candidates) {
        if (key(candidate) !== key(intent) || canonical(candidate.command) !== canonical(intent.command)
          || signedIntentFingerprint(candidate) !== signedIntentFingerprint(signedIntent)) {
          throw new Error('A disclosed payment must retain one original signed attempt and pre-action checkpoint');
        }
      }
      const retainedOpenings = new Map();
      for (const opening of [...(existing?.openings || []), ...openings]) retainedOpenings.set(canonical(opening), clone(opening));
      const next = { ...scope(intent), command: clone(intent.command),
        openings: [...retainedOpenings.values()],
        timing: mergeTiming(existing?.timing, timing),
        evidence: clone(existing?.evidence || evidence ? { ...existing?.evidence, ...evidence } : null) };
      if (signedIntent) {
        next.signedIntent = clone(signedIntent);
        next.evidence = { ...next.evidence, actionIntent: clone(signedIntent) };
        if (next.timing) next.timing.intent = clone(signedIntent);
      }
      const index = entries.findIndex(entry => key(entry) === key(intent));
      if (index < 0) entries.push(next); else entries[index] = next;
      write(next.matchId, entries);
      return clone(next);
    },
    accepted(matchId, sequence) {
      const entries = read(matchId);
      const retained = entries.filter(entry => entry.seq > Number(sequence));
      if (retained.length !== entries.length) write(matchId, retained);
    },
  };
}
