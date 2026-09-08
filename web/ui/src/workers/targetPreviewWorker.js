import initWasm, { WasmGame, compileAndRegisterCardSources } from '../../../wasm_demo/pkg/ironsmith.js';
import { castingMethodChoiceForAction } from '../lib/casting-method-choice.js';

let initialization, preview, generation = 0;
const registered = new Set();
self.onmessage = async ({ data }) => {
  const token = ++generation;
  if (data.type === 'cancel') return;
  try {
    initialization ||= initWasm({ engine: data.module, compiler: false, verifier: false });
    await initialization;
    if (token !== generation) { self.postMessage({ id: data.id, result: null }); return; }
    preview ||= new WasmGame();
    for (const [route, source] of data.sources) {
      if (registered.has(route)) continue;
      if (token !== generation) { self.postMessage({ id: data.id, result: null }); return; }
      const summary = compileAndRegisterCardSources(preview, [source]);
      if (summary.failed?.length) throw new Error(summary.failed[0].error);
      registered.add(route);
      await new Promise(resolve => setTimeout(resolve, 0));
    }
    const requirements = [];
    for (const action of data.actions) {
      if (token !== generation) { self.postMessage({ id: data.id, result: null }); return; }
      preview.importSyncCheckpoint(data.checkpoint, data.perspective);
      let state = preview.dispatch({ type: 'priority_action', action_index: action.index, action_ref: action.action_ref });
      const method = castingMethodChoiceForAction(state?.decision, action);
      if (method) state = preview.dispatch(method);
      if (state?.decision?.kind === 'targets') requirements.push(...state.decision.requirements);
      await new Promise(resolve => setTimeout(resolve, 0));
    }
    self.postMessage({ id: data.id, result: token === generation ? { kind: 'targets', player: data.perspective, requirements } : null });
  } catch (error) { self.postMessage({ id: data.id, error: error.message }); }
};
