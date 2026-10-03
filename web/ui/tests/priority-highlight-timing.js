// Test-only source instrumentation. Absolute performance timestamps align worker
// and window events without including console transport latency in measurements.
const emit = `const __pt = (event, fields = {}) => console.log('__PRIORITY_TIMING__' + JSON.stringify({event, at: performance.timeOrigin + performance.now(), ...fields}));\n`;
export function priorityHighlightTimingPlugin() {
  return {
    name: 'priority-highlight-timing', enforce: 'pre',
    transform(source, id) {
      const file = id.split('?')[0];
      const replacements = [];
      if (file.endsWith('/lib/isolated-priority-analysis.js')) {
        replacements.push(
          ['const invalidate = (dispose = false) => {', `const invalidate = (dispose = false) => { __pt('invalidate', {generation, revision, idle, dispose, hasWorker: !!worker});`],
          ['clearRecovery(); worker?.terminate();', `__pt('retire', {workerToken, hasWorker: !!worker}); clearRecovery(); worker?.terminate();`],
          ['const token = ++generation;', `const token = ++generation; __pt('schedule', {token, viewRevision});`],
          ['const input = await capture();', `__pt('captureRequest', {token}); const input = await capture(); __pt('captureDone', {token});`],
          ['worker ||= createWorker();', `__pt('workerSelect', {token, reused: !!worker}); worker ||= createWorker();`],
          ['worker.onmessage = ({ data }) => {', `worker.onmessage = ({ data }) => { const receivedAt = performance.timeOrigin + performance.now();`],
          ['return deliver(() => {', `return deliver(() => { __pt('delivery', {token, type: data.type, sequence: data.sequence, receivedAt, current: current()});`],
          ["worker.postMessage({ type: 'analyze', token, ...input });", `__pt('sendAnalyze', {token, viewRevision, sources: input.sources.length}); worker.postMessage({ type: 'analyze', token, ...input });`],
        );
      } else if (file.endsWith('/workers/priorityAnalysisWorker.js')) {
        replacements.push(
          ["  token = data.token;", `  token = data.token; __pt('childReceive', {token});`],
          ['  const nextRegistryKey', `  __pt('wasmReady', {token}); const nextRegistryKey`],
          ['  if (!game || registryKey !== nextRegistryKey) {', `  let compileMs = 0, yieldMs = 0; __pt('registryStart', {token, rebuild: !game || registryKey !== nextRegistryKey}); if (!game || registryKey !== nextRegistryKey) {`],
          ['      const result = compileAndRegisterCardSources(game, [source]);', `      const compileStart = performance.now(); const result = compileAndRegisterCardSources(game, [source]); compileMs += performance.now() - compileStart;`],
          ['      if (result.failed?.length) throw new Error(result.failed[0].error);\n      await yieldTask();', `      if (result.failed?.length) throw new Error(result.failed[0].error); const yieldStart = performance.now(); await yieldTask(); yieldMs += performance.now() - yieldStart;`],
          ['  if (job.cancelled) return;\n  phase', `  __pt('registryDone', {token, compileMs, yieldMs}); if (job.cancelled) return;\n  phase`],
          ['  initialized = true;', `  __pt('checkpointDone', {token}); initialized = true;`],
          ['    do {\n      const decision', `    __pt('beginDone', {token}); do {\n      const decision`],
          ["        self.postMessage({ type: 'priority'", `        __pt('computed', {token, complete: decision.analysis_complete, actions: decision.actions?.length}); self.postMessage({ type: 'priority'`],
        );
      } else if (file.endsWith('/workers/wasmGameWorker.js')) {
        replacements.push(
          ['      if (method !== "copyRuntimeSavepoint") priorityAnalysis.invalidate();', `      if (method !== "copyRuntimeSavepoint") { __pt('invalidateCommand', {method, runtimeBranch: msg.runtimeBranch}); priorityAnalysis.invalidate(); }`],
          ['          priorityAnalysis.invalidate();', `          __pt('invalidateIdentity', {method, runtimeBranch: msg.runtimeBranch}); priorityAnalysis.invalidate();`],
          ['checkpoint: game.exportSyncCheckpoint(),', `checkpoint: (() => { __pt('exportStart'); const checkpoint = game.exportSyncCheckpoint(); __pt('exportDone'); return checkpoint; })(),`],
        );
      } else if (file.endsWith('/context/GameContext.jsx')) {
        replacements.push(
          ['  const { state, setState, stateRef, subscribeState, isSnapshotRendered } = useGameSnapshot();', `  const { state, setState, stateRef, subscribeState, isSnapshotRendered } = useGameSnapshot();
  useEffect(() => { __pt('render', {revision: state?.__priority_revision, phase: state?.phase, step: state?.step, turn: state?.turn_number, player: state?.decision?.player, complete: state?.decision?.analysis_complete, playable: [...document.querySelectorAll('[data-hand-object-id] [aria-label*="playable"]')].map(el => el.getAttribute('data-object-id'))}); }, [state]);`],
          ['      if (next === previous) return;', `      __pt('merge', {revision: analysis?.revision, sequence: analysis?.sequence, accepted: next !== previous, phase: previous?.phase}); if (next === previous) return;`],
        );
      } else return null;
      let code = source;
      for (const [before, after] of replacements) {
        if (!code.includes(before)) throw new Error(`Timing instrumentation anchor missing in ${file}: ${before}`);
        code = code.replace(before, after);
      }
      return { code: emit + code, map: null };
    },
  };
}
