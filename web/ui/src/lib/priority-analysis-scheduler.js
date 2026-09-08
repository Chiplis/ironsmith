import { createAdaptiveWorkBudget } from './adaptive-work-budget.js';
/** Cooperative background work on the engine's immutable priority snapshot.
 * Every slice is a separate task, so queued game commands run between slices.
 */
export function createPriorityAnalysisScheduler({ game, busy, enqueue, publish, fail,
  schedule = (fn) => setTimeout(fn, 0), cancel = clearTimeout, budget = 128,
  now = () => performance.now(), reportSlice = () => {} }) {
  const priorityBudget = createAdaptiveWorkBudget({ initial: Math.min(8, budget), max: budget, now, report: reportSlice });
  const inspectorBudget = createAdaptiveWorkBudget({ initial: 1, max: 4, now, report: reportSlice });
  let revision = 0;
  const previews = new Map();
  const queue = [];
  let activePreview = null;
  let timer = null;
  let running = false;
  const stop = () => {
    if (timer !== null) cancel(timer);
    timer = null;
    running = false;
  };
  const invalidate = () => {
    stop();
    for (const entry of previews.values()) entry.resolve([]);
    previews.clear();
    queue.length = 0;
    activePreview = null;
    revision += 1;
    game()?.cancelPriorityAnalysis?.();
  };
  const start = (viewRevision = revision) => {
    if (running || typeof game()?.beginPriorityAnalysis !== "function") return;
    const token = String(revision);
    if (!game().beginPriorityAnalysis(token)) { startPreview(); return; }
    running = true;
    const tick = () => {
      timer = null;
      if (token !== String(revision)) return;
      if (busy()) { timer = schedule(tick); return; }
      enqueue(() => {
        if (token !== String(revision)) return;
        const decision = priorityBudget.run(units => game().stepPriorityAnalysis(token, units));
        if (decision === false) { stop(); start(); return; }
        if (decision) {
          stop();
          publish({ revision: viewRevision, decision });
          startPreview();
        } else {
          timer = schedule(tick);
        }
      }).catch((error) => {
        if (token !== String(revision)) return;
        stop();
        fail({ revision, error });
        startPreview();
      });
    };
    timer = schedule(tick);
  };
  const startPreview = () => {
    if (running || activePreview || !queue.length) return;
    const entry = queue.shift();
    activePreview = entry;
    running = true;
    const token = String(revision);
    let begun = false;
    const finish = (result) => {
      stop(); activePreview = null; entry.resolve(result); startPreview();
    };
    const tick = () => {
      timer = null;
      if (token !== String(revision)) return;
      if (busy()) { timer = schedule(tick); return; }
      enqueue(() => {
        if (token !== String(revision)) return;
        if (!begun) {
          game().beginInspectorAnalysis(token, ...entry.args);
          begun = true;
        }
        const result = inspectorBudget.run(units => game().stepInspectorAnalysis(token, units));
        if (result === null) timer = schedule(tick);
        else finish(result === false ? [] : result);
      }).catch(() => {
        if (token === String(revision)) finish([]);
      });
    };
    timer = schedule(tick);
  };
  const inspector = (...args) => {
    const key = args.map(String).join(":");
    if (previews.has(key)) return previews.get(key).promise;
    let resolve;
    const promise = new Promise(done => { resolve = done; });
    const entry = { args, resolve, promise };
    previews.set(key, entry); queue.push(entry);
    start();
    return promise;
  };
  return { invalidate, start, inspector, revision: () => revision, dispose: invalidate };
}

export function mergePriorityAnalysis(state, analysis) {
  if (!state || !analysis || state.__priority_revision !== analysis.revision
      || state.decision?.kind !== "priority"
      || state.decision.analysis_complete !== false
      || state.decision.player !== analysis.decision?.player) return state;
  return { ...state, decision: analysis.decision };
}
