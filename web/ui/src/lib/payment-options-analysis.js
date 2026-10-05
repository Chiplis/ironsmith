// Completed payment searches retain their initialized runtime. Active searches
// remain disposable so Pay/Cancel never wait behind speculative synchronous work.
export function createPaymentOptionsAnalysis({ capture, createWorker }) {
  let active = null, idleWorker = null, serial = 0;
  const cancel = () => {
    if (!active) return;
    active.worker?.terminate();
    active.resolve(null);
    active = null;
  };
  const dispose = () => { cancel(); idleWorker?.terminate(); idleWorker = null; };
  const run = async (...args) => {
    cancel();
    let resolve, reject;
    const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
    const job = { resolve, reject, worker: null, token: ++serial };
    active = job;
    try {
      const input = await capture(...args);
      if (active !== job) return promise;
      if (!input || input.request === 'null') { cancel(); return promise; }
      if (idleWorker && Boolean(idleWorker.runtimeFallback) !== Boolean(input.runtimeFallback)) {
        idleWorker.terminate(); idleWorker = null;
      }
      const worker = job.worker = idleWorker || createWorker(input);
      worker.runtimeFallback = Boolean(input.runtimeFallback);
      idleWorker = null;
      const finish = (error, result) => {
        if (active !== job) return;
        active = null;
        if (error) { worker.terminate(); reject(error); }
        else { idleWorker = worker; resolve(result); }
      };
      worker.onerror = event => finish(new Error(event.message));
      worker.onmessage = ({ data }) => {
        if (data.token !== job.token) return;
        finish(data.error ? new Error(data.error) : null, data.result);
      };
      worker.postMessage({ ...input, token: job.token });
    } catch (error) {
      if (active === job) { active = null; job.worker?.terminate(); reject(error); }
    }
    return promise;
  };
  return { run, cancel, dispose };
}

export function paymentOptionsKey(state) {
  const payment = state?.mana_payment;
  return payment ? JSON.stringify([state.__priority_revision, payment.transaction_id,
    payment.request_hash, payment.plan_id]) : null;
}

export function mergePaymentOptions(state, key, options) {
  if (options == null || paymentOptionsKey(state) !== key) return state;
  return { ...state, mana_payment: { ...state.mana_payment,
    ...options, activation_options_complete: true } };
}
