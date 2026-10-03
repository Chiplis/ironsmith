import initWasm, { WasmGame, compileAndRegisterCardSources } from '../../../wasm_demo/pkg/ironsmith.js';

self.onmessage = async ({ data }) => {
  try {
    await initWasm({ engine: data.module, compiler: false, verifier: false });
    const game = new WasmGame();
    game.setDeferredPriorityAnalysis(true);
    game.setDeferredManaOptions(true);
    for (const [, source] of data.sources) {
      const result = compileAndRegisterCardSources(game, [source]);
      if (result.failed?.length) throw new Error(result.failed[0].error);
    }
    for (const registration of data.registrations) {
      if (registration.method === 'createCustomCard') {
        game.createCustomCard({ ...registration.args[0], playerIndex: 0, zoneName: 'hand', skipTriggers: true, counterSeed: null });
      } else game[registration.method](...registration.args);
    }
    game.importSyncCheckpoint(data.checkpoint, data.checkpoint.perspective);
    self.postMessage({ result: game.getPaymentActivationOptions(data.request) });
  } catch (error) { self.postMessage({ error: error.message || String(error) }); }
};
