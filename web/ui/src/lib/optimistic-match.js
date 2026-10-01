// The accepted transcript is owned by the verifier. This ledger contains only
// temporary calculations and choices made against them; no entry is evidence.
// restoreVerified owns publishing the restored visible runtime snapshot.
export function createOptimisticMatch({ calculate, restoreVerified, publish,
  equivalent, onChange = () => {}, maxPending = 32 }) {
  let acceptedSequence = 0;
  let matchId = '';
  let generation = 0;
  let entries = [];
  let renderQueue = Promise.resolve();
  let calculating = false;
  let closed = true;
  const status = () => ({ matchId, generation, acceptedSequence,
    provisionalSequence: entries.at(-1)?.seq ?? acceptedSequence,
    pending: entries.length, calculating, closed });
  const changed = () => onChange(status());
  const serialize = task => {
    const result = renderQueue.then(task, task);
    renderQueue = result.catch(() => {});
    return result;
  };
  const discard = async reason => {
    generation++;
    const discarded = entries; entries = [];
    for (const entry of discarded) entry.cancel?.(reason);
    await restoreVerified();
    changed();
  };
  return {
    status,
    entries: () => entries.slice(),
    start(id, sequence = 0) {
      return serialize(async () => {
        await discard('Match replaced');
        matchId = String(id); acceptedSequence = Number(sequence); closed = false;
        changed();
      });
    },
    stage(candidate) {
      const requestedGeneration = generation;
      return serialize(async () => {
        if (closed || requestedGeneration !== generation || candidate.matchId !== matchId) return null;
        const seq = Number(candidate.seq ?? (entries.at(-1)?.seq ?? acceptedSequence) + 1);
        if (!Number.isSafeInteger(seq) || seq <= acceptedSequence) return null;
        const duplicate = entries.find(entry => entry.seq === seq);
        if (duplicate) {
          if (equivalent(duplicate, candidate)) return duplicate;
          await discard('Conflicting provisional actions');
          return null;
        }
        const parent = entries.at(-1);
        if (seq !== (parent?.seq ?? acceptedSequence) + 1 || entries.length >= maxPending) return null;
        if (candidate.parentId != null && candidate.parentId !== (parent?.id ?? '')
          && !(candidate.parentId === '' && Number(candidate.basisSequence) >= Number(parent?.seq))) return null;
        calculating = true; changed();
        try {
          // calculate is transactional and returns null if material is missing.
          const calculated = await calculate(candidate);
          if (!calculated) return null;
          if (requestedGeneration !== generation || closed) {
            await restoreVerified(); return null;
          }
          const entry = { ...candidate, ...calculated, seq,
            parentId: parent?.id ?? '', generation };
          entries.push(entry);
          publish(calculated.state, { provisional: true, seq });
          return entry;
        } finally { calculating = false; changed(); }
      });
    },
    accept(action) {
      return serialize(async () => {
        if (closed) return;
        const seq = Number(action.seq);
        if (seq <= acceptedSequence) return;
        if (seq !== acceptedSequence + 1) {
          await discard('Verified transcript was replaced');
          acceptedSequence = seq;
          changed(); return;
        }
        const first = entries[0];
        const matched = first && first.seq === seq && equivalent(first, action)
          && (!first.publicCheckpointHash || first.publicCheckpointHash === action.audit?.publicCheckpointHash);
        acceptedSequence = seq;
        if (matched) entries.shift();
        else if (entries.length) await discard('Verified action differs from provisional calculation');
        if (!entries.length) {
          // Restoration publishes a snapshot from the visible runtime, with
          // its analysis revision. Publishing the verifier's branch snapshot
          // afterward would strand the UI on an unfinished action menu.
          await restoreVerified();
        }
        changed();
      });
    },
    fail(seq, reason = 'Action verification failed') {
      return serialize(async () => {
        if (Number(seq) <= acceptedSequence || !entries.some(entry => entry.seq >= Number(seq))) return;
        // All provisional entries depend on the verified base; resetting the
        // entire suffix prevents stale IDs/choices being reused after rejection.
        await discard(reason);
      });
    },
    close(reason = 'Match closed') {
      closed = true; generation++;
      return serialize(async () => { await discard(reason); changed(); });
    },
    idle: () => renderQueue,
  };
}
