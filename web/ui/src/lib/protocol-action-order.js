// Requests bypass the action queue so replies can unblock an action in flight.
// They depend on the *preceding* accepted action, never their own queued action.
export function createProtocolActionOrder({ head, matchId, timeoutMs = 120000,
  setTimer = setTimeout, clearTimer = clearTimeout, onWait = () => {} }) {
  const waiters = new Set();
  let generation = 0;
  const assertCurrent = (request, label) => {
    if (String(request?.matchId || '') !== String(matchId())) {
      throw new Error(`${label} belongs to a different match`);
    }
    const seq = Number(request?.seq);
    if (!Number.isSafeInteger(seq) || seq <= 0) {
      throw new Error(`${label} has an invalid action sequence (received ${seq})`);
    }
    return seq;
  };
  const notify = () => {
    for (const waiter of [...waiters]) {
      if (waiter.generation !== generation || waiter.matchId !== String(matchId())) {
        waiter.finish(new Error('Protocol dependency was cancelled by match recovery'));
      } else if (Number(head()) >= waiter.target) waiter.finish();
    }
  };
  return {
    notify,
    reset(reason = 'Protocol dependency was cancelled by match recovery') {
      generation++;
      for (const waiter of [...waiters]) waiter.finish(new Error(reason));
    },
    async wait(request, label = 'Cryptographic material request', { signal } = {}) {
      if (signal?.aborted) throw new Error('Protocol dependency was cancelled');
      const seq = assertCurrent(request, label);
      const target = seq - 1;
      if (Number(head()) < target) {
        onWait({ seq, precedingSequence: target, acceptedSequence: Number(head()) });
        await new Promise((resolve, reject) => {
          const waiter = { generation, matchId: String(matchId()), target, timer: null,
            finish(error) {
              if (!waiters.delete(waiter)) return;
              clearTimer(waiter.timer);
              signal?.removeEventListener('abort', abort);
              if (error) reject(error); else resolve();
            } };
          const abort = () => waiter.finish(new Error('Protocol dependency was cancelled'));
          waiters.add(waiter);
          signal?.addEventListener('abort', abort, { once: true });
          waiter.timer = setTimer(() => waiter.finish(new Error(
            `${label} timed out waiting for action ${target} (accepted ${head()}, requested ${seq})`
          )), timeoutMs);
          notify();
        });
      }
      assertCurrent(request, label);
      const expected = Number(head()) + 1;
      if (seq !== expected) throw new Error(
        `${label} has an invalid action sequence (expected ${expected}, received ${seq})`
      );
    },
    pending: () => waiters.size,
  };
}
