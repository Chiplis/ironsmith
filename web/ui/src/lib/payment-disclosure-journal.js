// Durable, non-speculative commitments for one disclosed signed attempt.
// Only verified/signed disclosure producers may pin an entry. Engine savepoints
// and crypto previews deliberately never own or rewind this journal.
// Keep the established key: schema 2 adds RNG evidence to an existing pin rather
// than abandoning hand disclosures under a new, independent storage key.
const PREFIX = 'ironsmith.payment-disclosure.v1:';
const RNG_ENTRY_SCHEMA = 2;
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

function mergeExact(existing, incoming, label) {
  if (existing == null) return incoming == null ? null : clone(incoming);
  if (incoming != null && canonical(existing) !== canonical(incoming)) {
    throw new Error(`Random announcement changed its retained ${label}`);
  }
  return clone(existing);
}
function mergePlayerEvidence(existing = [], incoming = [], label) {
  if (!Array.isArray(existing) || !Array.isArray(incoming)) {
    throw new Error(`Invalid random announcement ${label}`);
  }
  const entries = new Map();
  for (const entry of [...existing, ...incoming]) {
    const player = entry?.player;
    if (!Number.isSafeInteger(player) || player < 0) {
      throw new Error(`Invalid random announcement ${label} player`);
    }
    entries.set(player, mergeExact(entries.get(player), entry, `${label} for player ${player}`));
  }
  return [...entries.values()].sort((a, b) => a.player - b.player);
}
function assertRandomStageConsistency(entry, intent) {
  const seen = new Map();
  const shared = {};
  function retain(fields, key, value) {
    if (value == null) return;
    if (fields[key] != null && canonical(fields[key]) !== canonical(value)) {
      throw new Error(`Random announcement has conflicting ${key} across evidence stages`);
    }
    fields[key] = value;
  }
  function stage(value, kind) {
    const required = kind === 'local contribution' ? ['commitRequestId', 'nonceHex', 'commitmentHex']
      : kind === 'reveal' ? ['requestId', 'commitRequestId', 'nonceHex', 'commitmentHex']
        : ['requestId', 'commitmentHex'];
    for (const field of required) {
      if (typeof value[field] !== 'string' || !value[field]) {
        throw new Error(`Random announcement ${kind} lacks ${field}`);
      }
    }
    const fields = seen.get(value.player) || {};
    retain(fields, 'commitRequestId', kind === 'commitment' ? value.requestId : value.commitRequestId);
    retain(fields, 'commitmentHex', value.commitmentHex);
    retain(fields, 'nonceHex', value.nonceHex);
    for (const field of ['matchId', 'seq', 'requester', 'actorIndex', 'prevStateHash']) {
      retain(shared, field, value[field]);
    }
    for (const checkpoint of [value.publicCheckpointHash, value.preActionPublicCheckpointHash]) {
      retain(shared, 'publicCheckpointHash', checkpoint);
    }
    if (value.requirementId != null && value.requirementId !== entry.requirement.id
      || value.contextKey != null && value.contextKey !== entry.contextKey) {
      throw new Error('Random announcement evidence has a different requirement domain');
    }
    seen.set(value.player, fields);
  }
  for (const value of entry.localContributions) stage(value, 'local contribution');
  for (const value of entry.commitments) stage(value, 'commitment');
  if (entry.commitSet != null) {
    if (typeof entry.commitSet.hash !== 'string' || !entry.commitSet.hash
      || !Array.isArray(entry.commitSet.commits) || !entry.commitSet.commits.length) {
      throw new Error('Invalid random announcement locked commit set');
    }
    const players = new Set();
    for (const value of entry.commitSet.commits) {
      if (!Number.isSafeInteger(value?.player) || value.player < 0 || players.has(value.player)) {
        throw new Error('Random announcement commit set has an invalid or repeated player');
      }
      players.add(value.player);
      stage(value, 'commitment');
    }
    if ([...seen.keys(), ...entry.reveals.map(value => value.player)].some(player => !players.has(player))) {
      throw new Error('Random announcement evidence has a player outside its locked commit set');
    }
  }
  for (const value of entry.reveals) stage(value, 'reveal');
  const bound = scope(intent);
  for (const field of ['matchId', 'seq', 'actorIndex', 'prevStateHash']) {
    if (shared[field] != null && shared[field] !== bound[field]) {
      throw new Error(`Random announcement ${field} differs from its signed attempt`);
    }
  }
  const checkpoint = intent.preActionPublicCheckpointHash || intent.publicCheckpointHash;
  if (shared.publicCheckpointHash != null && shared.publicCheckpointHash !== checkpoint) {
    throw new Error('Random announcement checkpoint differs from its signed attempt');
  }
  if (entry.witness != null && (entry.witness.randomCountBefore !== entry.requirement.randomCountBefore
    || entry.witness.randomCountAfter !== entry.requirement.randomCountAfter
    || typeof entry.witness.seedHex !== 'string' || !entry.witness.seedHex)) {
    throw new Error('Random announcement witness has a different native random boundary');
  }
}
function mergeRandomAnnouncements(existing = [], incoming = [], intent) {
  if (!Array.isArray(existing) || !Array.isArray(incoming)) {
    throw new Error('Invalid random announcement recovery material');
  }
  const entries = new Map();
  for (const entry of [...existing, ...incoming]) {
    const requirement = entry?.requirement;
    const id = requirement?.id;
    if (entry?.schemaVersion !== 1 || typeof id !== 'string' || !id
      || requirement.type !== 'fair_random' || !requirement.announcement
      || typeof entry.contextKey !== 'string' || !entry.contextKey) {
      throw new Error('Invalid typed random announcement recovery requirement');
    }
    const previous = entries.get(id);
    const next = {
      schemaVersion: 1,
      requirement: mergeExact(previous?.requirement, requirement, 'pre-draw requirement'),
      contextKey: mergeExact(previous?.contextKey, entry.contextKey, 'request context'),
      localContributions: mergePlayerEvidence(previous?.localContributions, entry.localContributions, 'local contribution'),
      commitments: mergePlayerEvidence(previous?.commitments, entry.commitments, 'commitment'),
      reveals: mergePlayerEvidence(previous?.reveals, entry.reveals, 'reveal'),
    };
    // A complete signed commit set is fixed before any nonce is disclosed.
    // Contribution records retain the exact nonce and request IDs needed by a
    // responder after reload; none may be regenerated under the retained pin.
    for (const field of ['commitSet', 'witness']) {
      const retained = mergeExact(previous?.[field], entry[field], field);
      if (retained != null) next[field] = retained;
    }
    assertRandomStageConsistency(next, intent);
    entries.set(id, next);
  }
  return [...entries.values()];
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
    for (const entry of entries) {
      const signedIntent = retainedSignedIntent(entry);
      if (signedIntent && (key(signedIntent) !== key(entry)
        || canonical(signedIntent.command) !== canonical(entry.command))) {
        throw new Error('Disclosure recovery signed attempt differs from its containing scope or command');
      }
      if (entry.schemaVersion != null && entry.schemaVersion !== 1
        && entry.schemaVersion !== RNG_ENTRY_SCHEMA) {
        throw new Error('Unsupported disclosure recovery schema');
      }
      if (entry.randomAnnouncements != null) {
        if (entry.schemaVersion !== RNG_ENTRY_SCHEMA) {
          throw new Error('Random announcement recovery requires its versioned schema');
        }
        if (!signedIntent?.signature || !signedIntent?.attemptId
          || !signedIntent?.preActionPublicCheckpointHash) {
          throw new Error('Random announcement recovery requires the original signed attempt and checkpoint');
        }
        mergeRandomAnnouncements([], entry.randomAnnouncements, signedIntent);
      }
    }
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
    pin(intent, { openings = [], evidence = null, timing = null, randomAnnouncements = [] } = {}) {
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
      const retainedRandom = mergeRandomAnnouncements(existing?.randomAnnouncements, randomAnnouncements, signedIntent || intent);
      if (retainedRandom.length && (!signedIntent?.signature || !signedIntent?.attemptId
        || !signedIntent?.preActionPublicCheckpointHash)) {
        throw new Error('Random announcement recovery requires the original signed attempt and checkpoint');
      }
      const retainedOpenings = new Map();
      for (const opening of [...(existing?.openings || []), ...openings]) retainedOpenings.set(canonical(opening), clone(opening));
      const next = { ...scope(intent), command: clone(intent.command),
        openings: [...retainedOpenings.values()],
        timing: mergeTiming(existing?.timing, timing),
        evidence: clone(existing?.evidence || evidence ? { ...existing?.evidence, ...evidence } : null) };
      if (retainedRandom.length) {
        next.schemaVersion = RNG_ENTRY_SCHEMA;
        next.randomAnnouncements = retainedRandom;
      } else if (existing?.schemaVersion != null) {
        next.schemaVersion = existing.schemaVersion;
      }
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
