// Exact local fallback when executable runtime state cannot cross a wire.
// Every call enters/leaves a retained native branch inside one command-queue
// task. Yielding never leaves the visible engine in the speculative branch.
export function createLocalPriorityWorker({ capture, call, release, yieldTask = () => new Promise(resolve => setTimeout(resolve, 0)) }) {
  let destroyed = false, job = null, running = false, pending = null, cancelSerial = 0;
  const inspectors = [];
  const api = {
    onmessage: null, onerror: null,
    postMessage(data) {
      if (destroyed) return;
      if (data.type === 'cancel') {
        cancelSerial = data.serial;
        if (job?.token === data.token) job.cancelled = true;
        if (pending?.token === data.token) pending = null;
        if (!running) void pump();
      } else if (data.type === 'analyze') {
        if (job) job.cancelled = true;
        pending = data; inspectors.length = 0; void pump();
      } else if (data.type === 'inspector') {
        if (data.token !== pending?.token && (data.token !== job?.token || job.cancelled)) return;
        inspectors.push(data); void pump();
      }
    },
    terminate() {
      destroyed = true; pending = null; inspectors.length = 0;
      if (job) job.cancelled = true;
      if (!running) void closeJob();
    },
  };
  const emit = data => { if (!destroyed) api.onmessage?.({ data }); };
  const closeJob = async () => {
    const old = job; job = null;
    if (old?.handle != null) await release(old.handle);
  };
  async function pump() {
    if (running || destroyed) return;
    running = true;
    try {
      do {
        if (pending) {
          const input = pending; pending = null;
          const previous = job;
          job = { token: input.token, cancelled: false, handle: null, complete: false };
          const owned = job;
          if (previous?.handle != null) await release(previous.handle);
          if (!destroyed && !owned.cancelled) {
            emit({ type: 'phase', token: owned.token, phase: 'search' });
            owned.handle = await capture(input);
          }
          if (destroyed || owned.cancelled) {
            await closeJob();
            emit({ type: 'available', token: owned.token, cancelSerial });
            continue;
          }
          const token = String(owned.token);
          const began = await call(owned.handle, 'beginPriorityAnalysis', [token]);
          owned.complete = !began;
          while (!owned.complete && !owned.cancelled && !destroyed) {
            const decision = await call(owned.handle, 'stepPriorityAnalysis', [token, 8]);
            if (owned.cancelled || destroyed) break;
            if (decision === false) throw new Error('Priority runtime branch became stale');
            if (decision) {
              emit({ type: 'priority', token: owned.token, decision });
              owned.complete = decision.analysis_complete === true;
            }
            await yieldTask();
          }
        }
        const owned = job;
        while (owned?.complete && !owned.cancelled && !destroyed && inspectors.length) {
          const request = inspectors.shift();
          if (request.token !== owned.token) continue;
          const token = `${owned.token}:${request.id}`;
          await call(owned.handle, 'beginInspectorAnalysis', [token, ...request.args]);
          let result;
          do {
            result = await call(owned.handle, 'stepInspectorAnalysis', [token, 8]);
            if (result == null) await yieldTask();
          } while (result == null && !owned.cancelled && !destroyed);
          if (owned.cancelled || destroyed) break;
          emit({ type: 'inspector', token: owned.token, id: request.id, result: result === false ? [] : result });
          await yieldTask();
        }
        if (job?.cancelled || destroyed) await closeJob();
        emit({ type: 'available', token: owned?.token, cancelSerial });
      } while (pending && !destroyed);
      if (job && !job.cancelled) emit({ type: 'idle', token: job.token });
    } catch (error) {
      const token = job?.token;
      await closeJob();
      emit({ type: 'error', token, error: error?.message || String(error) });
    } finally {
      running = false;
      if (destroyed) await closeJob();
      else if (pending) void pump();
    }
  }
  return api;
}

// Payment inventory is one bounded native query. Cancellation suppresses stale
// publication, and the owning run closure must restore its branch in finally.
export function createLocalQueryWorker(run) {
  let destroyed = false;
  const api = {
    onmessage: null, onerror: null,
    terminate() { destroyed = true; },
    postMessage(input) {
      Promise.resolve().then(() => destroyed ? null : run(input, () => destroyed))
        .then(result => { if (!destroyed) api.onmessage?.({ data: { token: input.token, result } }); },
          error => { if (!destroyed) api.onmessage?.({ data: { token: input.token, error: error?.message || String(error) } }); });
    },
  };
  return api;
}

export async function collectLocalTargetPreviews(actions, perspective, runAction, isCurrent,
  yieldTask = () => new Promise(resolve => setTimeout(resolve, 0))) {
  const requirements = [];
  for (const action of actions) {
    if (!isCurrent()) return null;
    const next = await runAction(action);
    if (next == null || !isCurrent()) return null;
    requirements.push(...next);
    await yieldTask();
  }
  return isCurrent() ? { kind: 'targets', player: perspective, requirements } : null;
}
