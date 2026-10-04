import initWasm, { WasmGame } from '../../../wasm_demo/pkg/ironsmith.js';
import { createLocalAnalysisReplica } from '../lib/local-analysis-replay.js';

const replica = createLocalAnalysisReplica(() => new WasmGame());
self.onmessage = async ({ data }) => {
  const started = performance.now();
  try {
    await initWasm({ engine: data.module, compiler: false, verifier: false });
    const game = await replica.hydrate(data.localReplay);
    const replayMs = performance.now() - started;
    const result = game.getPaymentActivationOptions(data.request);
    self.postMessage({ token: data.token, result: { ...result, __payment_options_perf: {
      replayMs, computeOptionsMs: performance.now() - started - replayMs,
      totalHandlerMs: performance.now() - started,
    } } });
  } catch (error) { self.postMessage({ token: data.token, error: error.message || String(error) }); }
};
