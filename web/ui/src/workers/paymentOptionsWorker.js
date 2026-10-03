import initWasm, { WasmGame, compileAndRegisterCardSources } from '../../../wasm_demo/pkg/ironsmith.js';

let game = null, registryKey = null;

self.onmessage = async ({ data }) => {
  const started = performance.now();
  const timings = { coldGame: !game, sourceCount: data.sources.length, registrationCount: data.registrations.length };
  let phaseStarted = started;
  try {
    await initWasm({ engine: data.module, compiler: false, verifier: false });
    timings.initMs = performance.now() - phaseStarted;
    phaseStarted = performance.now();
    const nextRegistryKey = JSON.stringify([data.sources, data.registrations]);
    timings.registryKeyMs = performance.now() - phaseStarted;
    timings.registryKeyChanged = registryKey !== nextRegistryKey;
    timings.rebuiltGame = !game || timings.registryKeyChanged;
    timings.createGameMs = timings.sourceRegistrationMs = timings.registrationReplayMs = 0;
    timings.disposeGameMs = timings.constructorMs = timings.deferredPriorityConfigMs = timings.deferredManaConfigMs = 0;
    if (timings.rebuiltGame) {
      phaseStarted = performance.now();
      const createStarted = phaseStarted;
      game?.free();
      timings.disposeGameMs = performance.now() - phaseStarted;
      phaseStarted = performance.now();
      game = new WasmGame();
      timings.constructorMs = performance.now() - phaseStarted;
      phaseStarted = performance.now();
      game.setDeferredPriorityAnalysis(true);
      timings.deferredPriorityConfigMs = performance.now() - phaseStarted;
      phaseStarted = performance.now();
      game.setDeferredManaOptions(true);
      timings.deferredManaConfigMs = performance.now() - phaseStarted;
      phaseStarted = createStarted;
      timings.createGameMs = performance.now() - phaseStarted;
      phaseStarted = performance.now();
      for (const [, source] of data.sources) {
        // Fetched sources include rejected cards retained for load diagnostics.
        // Mirror the command worker: keep successful definitions and let the
        // checkpoint/operation validate the cards this analysis actually needs.
        compileAndRegisterCardSources(game, [source]);
      }
      timings.sourceRegistrationMs = performance.now() - phaseStarted;
      phaseStarted = performance.now();
      for (const registration of data.registrations) {
        if (registration.method === 'createCustomCard') {
          game.createCustomCard({ ...registration.args[0], playerIndex: 0, zoneName: 'hand', skipTriggers: true, counterSeed: null });
        } else game[registration.method](...registration.args);
      }
      timings.registrationReplayMs = performance.now() - phaseStarted;
      registryKey = nextRegistryKey;
    }
    phaseStarted = performance.now();
    game.importSyncCheckpoint(data.checkpoint, data.checkpoint.perspective);
    timings.importCheckpointMs = performance.now() - phaseStarted;
    phaseStarted = performance.now();
    const result = game.getPaymentActivationOptions(data.request);
    timings.computeOptionsMs = performance.now() - phaseStarted;
    timings.totalHandlerMs = performance.now() - started;
    self.postMessage({ token: data.token, result: { ...result, __payment_options_perf: timings } });
  } catch (error) { self.postMessage({ token: data.token, error: error.message || String(error) }); }
};
