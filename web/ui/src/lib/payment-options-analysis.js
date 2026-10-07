import { localReplayTransfer } from './local-analysis-replay.js';

// A cancelled job that never reaches a yield is stuck in synchronous WASM.
export const PAYMENT_WORKER_STUCK_MS = 10_000;

// Pay/Cancel never wait behind speculative work: cancelling settles the job
// at once. The worker stops at its next yield and keeps its caught-up replica,
// so a superseded search does not force the next one to replay the session.
export function createPaymentOptionsAnalysis({ capture, createWorker,
  schedule = (fn, ms) => setTimeout(fn, ms), clearSchedule = clearTimeout, stuckMs = PAYMENT_WORKER_STUCK_MS }) {
  let active = null, worker = null, serial = 0;
  const retire = () => {
    if (!worker) return;
    if (worker.watchdog != null) clearSchedule(worker.watchdog);
    worker.terminate();
    worker = null;
  };
  const settle = (job, value) => {
    if (active === job) active = null;
    job.resolve(value);
  };
  const post = (target, job) => {
    target.outstanding.add(job.token);
    target.postMessage({ ...job.input, token: job.token }, localReplayTransfer(job.input.localReplay));
  };
  const spawn = input => {
    const created = createWorker(input);
    // `cancelled` holds superseded jobs the worker has not yet acknowledged.
    Object.assign(created, { runtimeFallback: Boolean(input.runtimeFallback), replicaMark: null,
      outstanding: new Set(), cancelled: new Set(), watchdog: null });
    created.onerror = event => {
      if (worker !== created) return;
      const job = active;
      retire();
      if (job) { active = null; job.reject(new Error(event.message)); }
    };
    created.onmessage = ({ data }) => {
      if (worker !== created) return;
      if (data.replicaMark) created.replicaMark = data.replicaMark;
      created.outstanding.delete(data.token);
      created.cancelled.delete(data.token);
      if (!created.cancelled.size && created.watchdog != null) {
        clearSchedule(created.watchdog);
        created.watchdog = null;
      }
      const job = active;
      if (!job || data.token !== job.token) return;
      if (data.error) { retire(); active = null; job.reject(new Error(data.error)); return; }
      settle(job, data.cancelled ? null : data.result);
    };
    return created;
  };
  const armWatchdog = target => {
    if (target.watchdog != null) return;
    target.watchdog = schedule(() => {
      target.watchdog = null;
      if (worker !== target || !target.cancelled.size) return;
      const job = active;
      retire();
      // Re-capture for a fresh, seeded replacement rather than abandoning the
      // newest request; its earlier seed memory moved to the retired worker.
      if (job && !job.capturing) void submit(job);
    }, stuckMs);
  };
  const cancel = () => {
    const job = active;
    if (!job) return;
    active = null;
    job.resolve(null);
    if (worker?.outstanding.has(job.token)) {
      worker.cancelled.add(job.token);
      worker.postMessage({ type: 'cancel', token: job.token });
      armWatchdog(worker);
    }
  };
  const dispose = () => { cancel(); retire(); };
  const submit = async job => {
    job.capturing = true;
    try {
      const input = await capture(job.args, { replicaMark: worker?.replicaMark ?? null });
      if (active !== job) return;
      if (!input || input.request === 'null') { cancel(); return; }
      if (worker && Boolean(worker.runtimeFallback) !== Boolean(input.runtimeFallback)) retire();
      job.input = input;
      worker ||= spawn(input);
      post(worker, job);
    } catch (error) {
      if (active === job) { active = null; job.reject(error); }
    } finally { job.capturing = false; }
  };
  const run = (...args) => {
    cancel();
    let resolve, reject;
    const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
    const job = active = { resolve, reject, token: ++serial, args, input: null, capturing: false };
    void submit(job);
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
