import { createLocalAnalysisReplica } from '../lib/local-analysis-replay.js';
import initWasm, { WasmGame } from '../../../wasm_demo/pkg/ironsmith.js';
import { restoreAnalysisSeed } from './analysis-seed.js';

// One pump owns WASM. New snapshots supersede searches at yield boundaries;
// registry initialization finishes once and survives cancellation.
let sequence = 0, cancelSerial = 0;
let game, token, complete = false, initialized = false, running = false;
let job = null, pendingAnalysis = null;
const inspectors = [];
let initialization = null;
const replica = createLocalAnalysisReplica(() => new WasmGame(),
  { restoreSeed: (image, oldGame) => restoreAnalysisSeed(initialization, image, oldGame) });
const yieldTask = () => new Promise(resolve => setTimeout(resolve, 0));
const reportError = error => self.postMessage({ type: 'error', token, error: error.stack || error.message || String(error) });
const phase = value => self.postMessage({ type: 'phase', token, phase: value });

async function drainInspectors() {
  if (!initialized || !complete || job.cancelled) return;
  while (inspectors.length && !job.cancelled) {
    const request = inspectors.shift();
    if (request.token !== token) continue;
    const searchToken = `${token}:${request.id}`;
    game.beginInspectorAnalysis(searchToken, ...request.args);
    let result;
    do {
      result = game.stepInspectorAnalysis(searchToken, 8);
      if (result == null) await yieldTask();
    } while (result == null && !job.cancelled);
    if (job.cancelled) return;
    self.postMessage({ type: 'inspector', token, id: request.id, result: result === false ? [] : result });
    await yieldTask();
  }
}

async function analyze(data) {
  token = data.token;
  initialized = false;
  complete = false;
  phase('initializing');
  initialization ||= initWasm({ engine: data.module, compiler: false, verifier: false });
  await initialization;
  game = await replica.hydrate(data.localReplay, yieldTask);
  self.postMessage({ type: 'replica', token, mark: { epoch: data.localReplay.epoch,
    position: (data.localReplay.base ?? 0) + (data.localReplay.operations?.length ?? 0) } });
  if (job.cancelled) return;
  phase('search');
  initialized = true;
  if (game.beginPriorityAnalysis(String(token))) {
    do {
      const decision = game.stepPriorityAnalysis(String(token), 8);
      if (decision === false) throw new Error('Priority snapshot changed during isolated analysis');
      if (decision) {
        self.postMessage({ type: 'priority', token, sequence: ++sequence, decision });
        complete = decision.analysis_complete === true;
      }
      await yieldTask();
    } while (!complete && !job.cancelled);
  } else complete = true;
}

async function pump() {
  if (running) return;
  running = true;
  try {
    do {
      if (pendingAnalysis) {
        const data = pendingAnalysis;
        pendingAnalysis = null;
        job = { token: data.token, cancelled: false };
        await analyze(data);
      }
      if (job && !job.cancelled) await drainInspectors();
      // Acknowledges reaching a safe boundary even for an obsolete token.
      self.postMessage({ type: 'available', token, cancelSerial });
    } while (pendingAnalysis);
    if (job && !job.cancelled) self.postMessage({ type: 'idle', token });
  } catch (error) { reportError(error); }
  finally { running = false; }
}

self.onmessage = ({ data }) => {
  if (data.type === 'cancel') {
    cancelSerial = data.serial;
    if (job?.token === data.token) job.cancelled = true;
    if (pendingAnalysis?.token === data.token) pendingAnalysis = null;
    if (!running) self.postMessage({ type: 'available', token, cancelSerial });
    return;
  }
  if (data.type === 'inspector') {
    if (data.token !== pendingAnalysis?.token && (data.token !== job?.token || job.cancelled)) return;
    inspectors.push(data); void pump(); return;
  }
  if (data.type !== 'analyze') return;
  if (job) job.cancelled = true;
  pendingAnalysis = data;
  inspectors.length = 0;
  void pump();
};
