// Recover public, signed action payloads only. The receiver's normal verifier
// remains the authority for signatures, openings, RNG, clocks, and state hashes.
export function createSequencedActionRecovery({ head, stateHash, matchId, send, apply,
  onWait = () => {}, onFailure = () => {}, onResolved = () => {},
  requestId = () => crypto.randomUUID(), setTimer = setTimeout, clearTimer = clearTimeout,
  retryMs = 4000, maxAttempts = 3 }) {
  let current = null;
  const clear = () => { if (current?.timer) clearTimer(current.timer); current = null; };
  const fail = (record, error) => {
    if (current !== record) return;
    clearTimer(record.timer); record.timer = null; record.failed = true;
    onFailure({ sequence: Math.max(record.fromSequence, head() + 1), error: String(error?.message || error) });
  };
  const attempt = record => {
    if (current !== record) return;
    if (record.matchId !== matchId()) { clear(); return; }
    if (head() >= record.throughSequence) { clear(); onResolved(); return; }
    if (record.attempts >= maxAttempts) {
      fail(record, `Could not recover action ${head() + 1} after ${maxAttempts} attempts`); return;
    }
    record.attempts++;
    record.requestId = requestId(); record.fromSequence = head() + 1;
    record.prevStateHash = stateHash();
    onWait({ sequence: record.fromSequence, attempt: record.attempts });
    try {
      send(record.actorIndex, { type: 'signed_action_recovery_request', requestId: record.requestId,
        matchId: record.matchId, fromSequence: record.fromSequence,
        throughSequence: Math.min(record.throughSequence, record.fromSequence + 63),
        prevStateHash: record.prevStateHash });
    } catch {
      // A closed route may reconnect during the bounded retry window.
    }
    record.timer = setTimer(() => attempt(record), retryMs);
  };
  return {
    request(message) {
      const seq = Number(message?.seq);
      if (!Number.isSafeInteger(seq) || seq <= head() + 1) return false;
      if (current && current.matchId !== matchId()) clear();
      if (current) { current.throughSequence = Math.max(current.throughSequence, seq); return true; }
      current = { matchId: matchId(), throughSequence: seq, actorIndex: Number(message.actorIndex),
        attempts: 0, timer: null, failed: false };
      attempt(current); return true;
    },
    async receive(message) {
      const record = current;
      if (!record || record.failed || message.requestId !== record.requestId
        || message.matchId !== record.matchId || matchId() !== record.matchId) return false;
      const actions = message.actions;
      if (!Array.isArray(actions) || !actions.length || actions.length > 64) return false;
      if (Number(actions[0].seq) !== record.fromSequence
        || String(actions[0].audit?.prevStateHash || '') !== record.prevStateHash) return false;
      if (actions.some((action, index) => Number(action.seq) !== record.fromSequence + index
        || Number(action.seq) > Math.min(record.throughSequence, record.fromSequence + 63))) return false;
      clearTimer(record.timer); record.timer = null;
      try {
        for (const action of actions) {
          if (current !== record || matchId() !== record.matchId) return false;
          await apply({ ...action, type: 'apply_action' });
          if (head() < Number(action.seq)) throw Error(`Action ${action.seq} was not accepted`);
        }
        if (current !== record) return true;
        if (head() >= record.throughSequence) { clear(); onResolved(); }
        else attempt(record);
        return true;
      } catch (error) { fail(record, error); return false; }
    },
    notify() {
      if (current && (current.matchId !== matchId() || head() >= current.throughSequence)) {
        clear(); onResolved();
      }
    },
    reset: clear,
    pending: () => Boolean(current),
  };
}
