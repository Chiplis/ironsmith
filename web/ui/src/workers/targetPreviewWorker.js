import initWasm, { WasmGame } from '../../../wasm_demo/pkg/ironsmith.js';
import { createLocalAnalysisReplica } from '../lib/local-analysis-replay.js';
import { castingMethodChoiceForAction } from '../lib/casting-method-choice.js';
import { restoreAnalysisSeed } from './analysis-seed.js';

let initialization, preview, generation = 0;
let queue = Promise.resolve();
const replica = createLocalAnalysisReplica(() => new WasmGame(),
  { restoreSeed: (image, oldGame) => restoreAnalysisSeed(initialization, image, oldGame) });
self.onmessage = async ({ data }) => {
  const token = ++generation;
  if (data.type === 'cancel') return;
  const run = async () => {
    try {
      initialization ||= initWasm({ engine: data.module, compiler: false, verifier: false });
      await initialization;
      if (token !== generation) { self.postMessage({ id: data.id, result: null }); return; }
      preview = await replica.hydrate(data.localReplay, () => new Promise(resolve => setTimeout(resolve, 0)));
      const replicaMark = { epoch: data.localReplay.epoch,
        position: (data.localReplay.base ?? 0) + data.localReplay.operations.length };
      const requirements = [];
      for (const action of data.actions) {
        if (token !== generation) { self.postMessage({ id: data.id, result: null, replicaMark }); return; }
        preview = await replica.resetWorkingState();
        preview.setPerspective(data.perspective);
        let state = preview.dispatch({ type: 'priority_action', action_index: action.index, action_ref: action.action_ref });
        const method = castingMethodChoiceForAction(state?.decision, action);
        if (method) state = preview.dispatch(method);
        if (state?.decision?.kind === 'targets') requirements.push(...state.decision.requirements);
        await new Promise(resolve => setTimeout(resolve, 0));
      }
      self.postMessage({ id: data.id, result: token === generation ? { kind: 'targets', player: data.perspective, requirements } : null, replicaMark });
    } catch (error) { self.postMessage({ id: data.id, error: error.message }); }
  };
  queue = queue.then(run, run);
  await queue;
};
