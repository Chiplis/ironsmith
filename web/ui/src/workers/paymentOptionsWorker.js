import initWasm, { WasmGame } from '../../../wasm_demo/pkg/ironsmith.js';
import { createLocalAnalysisReplica } from '../lib/local-analysis-replay.js';

const replica = createLocalAnalysisReplica(() => new WasmGame());
const yieldTask = () => new Promise(resolve => setTimeout(resolve, 0));
self.onmessage = async ({ data }) => {
  const started = performance.now();
  try {
    await initWasm({ engine: data.module, compiler: false, verifier: false });
    const game = await replica.hydrate(data.localReplay, yieldTask);
    if (data.kind === 'ranking') {
      let result = false;
      if (game.beginPaymentAnalysis(String(data.token))) {
        do {
          await yieldTask();
          result = game.stepPaymentAnalysis(String(data.token), 1);
        } while (result == null);
      }
      self.postMessage({ token: data.token, result: result || null });
      return;
    }
    const replayMs = performance.now() - started;
    const result = game.getPaymentActivationOptions(data.request);
    self.postMessage({ token: data.token, result: { ...result, __payment_options_perf: {
      replayMs, computeOptionsMs: performance.now() - started - replayMs,
      totalHandlerMs: performance.now() - started,
    } } });
  } catch (error) { self.postMessage({ token: data.token, error: error.message || String(error) }); }
};
