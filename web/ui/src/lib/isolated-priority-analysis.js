/** Speculation owns an isolated worker or an exact, sliced native runtime branch. */
export function createIsolatedPriorityAnalysis({ capture, identity, pending, createWorker,
  publish, fail, deliver = operation => operation(), eligible = () => true, schedule = (fn, delay = 0) => setTimeout(fn, delay), cancel = clearTimeout }) {
  let revision = 0, generation = 0, worker = null, timer = null, requestedKey = null;
  let active = null, inspectorId = 0, publicationSequence = 0, idle = false;
  const inspectors = new Map();
  let recovery = null, workerToken = null, workerPhase = 'initializing', latestViewRevision = 0;
  let cancellationSerial = 0;
  const clearRecovery = () => { if (recovery !== null) cancel(recovery); recovery = null; };
  const retire = () => {
    clearRecovery(); worker?.terminate(); worker = null; idle = false;
    workerToken = null; workerPhase = 'initializing';
  };
  const cancelWork = () => {
    if (!worker || idle) return;
    worker.postMessage({ type: 'cancel', token: workerToken, serial: ++cancellationSerial });
    if (recovery !== null) return;
    const retiring = worker;
    // Normal searches yield every slice. Allow one-time registry setup to
    // finish, but retain hard preemption for a worker stuck in synchronous WASM.
    recovery = schedule(() => {
      recovery = null;
      if (worker !== retiring) return;
      retire(); generation++; active = null; requestedKey = null;
      start(latestViewRevision);
    }, workerPhase === 'initializing' ? 5000 : 100);
  };
  const invalidate = (dispose = false) => {
    generation++;
    revision++;
    if (timer !== null) cancel(timer);
    timer = null;
    if (dispose) retire(); else cancelWork();
    active = null; requestedKey = null;
    for (const entry of inspectors.values()) entry.resolve([]);
    inspectors.clear();
  };
  const start = (viewRevision = revision) => {
    latestViewRevision = viewRevision;
    if (!pending() && !inspectors.size) return;
    const key = identity();
    if (requestedKey === key) return;
    requestedKey = key;
    const token = ++generation;
    if (timer !== null) cancel(timer);
    timer = schedule(async () => {
      timer = null;
      try {
        const input = await capture();
        if (token !== generation || identity() !== key) return;
        if (!idle) cancelWork();
        if (worker && Boolean(worker.runtimeFallback) !== Boolean(input.runtimeFallback)) retire();
        if (!worker) { worker = createWorker(input); worker.runtimeFallback = Boolean(input.runtimeFallback); }
        idle = false;
        active = { token, key, viewRevision };
        const current = () => token === generation && identity() === key;
        const failed = error => {
          if (!current()) return;
          fail({ revision: viewRevision, error });
          for (const entry of inspectors.values()) entry.reject(error);
          inspectors.clear();
          retire(); active = null; requestedKey = null;
        };
        worker.onerror = event => deliver(() => failed(new Error(event.message)));
        const targetWorker = worker;
        worker.onmessage = ({ data }) => {
          if (worker !== targetWorker) return;
          // Lifecycle acknowledgements do not read the authoritative runtime.
          // Accept an obsolete job's yield so its initialized worker can survive.
          if (data.type === 'available') {
            if (data.cancelSerial === cancellationSerial) clearRecovery();
            return;
          }
          if (data.type === 'phase') {
            if (data.token === workerToken) workerPhase = data.phase;
            return;
          }
          return deliver(() => {
            if (!current() || data.token !== token) return;
            if (data.type === 'error') { failed(new Error(data.error)); return; }
            if (data.type === 'idle' && [...inspectors.values()].every(entry => entry.settled)) idle = true;
            if (data.type === 'priority') publish({ revision: viewRevision, decision: data.decision, sequence: ++publicationSequence });
            if (data.type === 'inspector') {
              for (const entry of inspectors.values()) {
                if (entry.id === data.id) { entry.settled = true; entry.resolve(data.result); }
              }
            }
          });
        };
        workerToken = token;
        worker.postMessage({ type: 'analyze', token, ...input });
        for (const entry of inspectors.values()) worker.postMessage({ type: 'inspector', token, id: entry.id, args: entry.args });
      } catch (error) {
        if (token !== generation || identity() !== key) return;
        fail({ revision: viewRevision, error });
        for (const entry of inspectors.values()) entry.reject(error);
        inspectors.clear();
        retire();
        active = null; requestedKey = null;
      }
    });
  };
  const inspector = (...args) => {
    if (!eligible()) return Promise.resolve([]);
    const key = args.map(String).join(':');
    if (inspectors.has(key)) return inspectors.get(key).promise;
    let resolve, reject;
    const promise = new Promise((done, failed) => { resolve = done; reject = failed; });
    const entry = { id: ++inspectorId, args, promise, resolve, reject };
    inspectors.set(key, entry);
    if (worker && active?.key === identity()) {
      idle = false;
      worker.postMessage({ type: 'inspector', token: active.token, id: entry.id, args });
    } else start();
    return promise;
  };
  return { start, invalidate, inspector, revision: () => revision, dispose: () => invalidate(true) };
}
