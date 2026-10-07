import initWasm, { WasmGame } from '../../../wasm_demo/pkg/ironsmith.js';
import { createLocalAnalysisReplica } from '../lib/local-analysis-replay.js';
import { restoreAnalysisSeed } from './analysis-seed.js';

// One job owns the replica at a time. A newer job or an explicit cancel stops
// the current one at its next yield, so this initialized replica survives.
let initialization = null, current = null, pending = null, pump = null;
const replica = createLocalAnalysisReplica(() => new WasmGame(),
  { restoreSeed: (image, oldGame) => restoreAnalysisSeed(initialization, image, oldGame) });
const yieldTask = () => new Promise(resolve => setTimeout(resolve, 0));
const CANCELLED = Symbol('cancelled');

async function execute(job) {
  const checkpoint = async () => {
    await yieldTask();
    if (job.cancelled) throw CANCELLED;
  };
  const started = performance.now();
  try {
    initialization ||= initWasm({ engine: job.module, compiler: false, verifier: false });
    await initialization;
    const game = await replica.hydrate(job.localReplay, checkpoint);
    // Tell the owner how far this replica is, even if the job is cancelled.
    job.replicaMark = { epoch: job.localReplay.epoch,
      position: (job.localReplay.base ?? 0) + (job.localReplay.operations?.length ?? 0) };
    if (job.cancelled) throw CANCELLED;
    if (job.kind === 'ranking') {
      let result = false;
      if (game.beginPaymentAnalysis(String(job.token))) {
        do {
          await checkpoint();
          result = game.stepPaymentAnalysis(String(job.token), 1);
        } while (result == null);
      }
      return { token: job.token, result: result || null, replicaMark: job.replicaMark };
    }
    const replayMs = performance.now() - started;
    const result = game.getPaymentActivationOptions(job.request);
    return { token: job.token, replicaMark: job.replicaMark, result: { ...result, __payment_options_perf: {
      replayMs, computeOptionsMs: performance.now() - started - replayMs,
      totalHandlerMs: performance.now() - started,
    } } };
  } catch (error) {
    if (error === CANCELLED) return { token: job.token, cancelled: true, replicaMark: job.replicaMark };
    return { token: job.token, error: error?.message || String(error) };
  }
}

function drain() {
  pump ||= (async () => {
    try {
      while (pending) {
        current = { ...pending, cancelled: false };
        pending = null;
        self.postMessage(await execute(current));
      }
    } finally { current = null; pump = null; }
  })();
  return pump;
}

self.onmessage = ({ data }) => {
  if (data.type === 'cancel') {
    if (current?.token === data.token) current.cancelled = true;
    if (pending?.token === data.token) {
      self.postMessage({ token: pending.token, cancelled: true });
      pending = null;
    }
    return pump;
  }
  if (current) current.cancelled = true;
  if (pending) self.postMessage({ token: pending.token, cancelled: true });
  pending = data;
  return drain();
};
