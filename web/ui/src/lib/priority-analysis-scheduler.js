/** Merge the current background menu without accepting an obsolete snapshot. */
export function mergePriorityAnalysis(state, analysis) {
  if (!state || !analysis || state.__priority_revision !== analysis.revision
      || state.decision?.kind !== 'priority'
      || state.decision.analysis_complete !== false
      || state.decision.player !== analysis.decision?.player
      || state.decision === analysis.decision
      || (analysis.sequence !== undefined && state.decision.__priority_analysis_sequence === analysis.sequence)
      || (state.__priority_analysis_sequence ?? -1) > (analysis.sequence ?? 0)) return state;
  // Undo belongs to the live continuation, which is intentionally absent from
  // the analysis checkpoint. Keep that authoritative control in the menu.
  const undo = (state.decision.actions || []).filter(action => action.action_ref?.kind === 'untap_land');
  const decision = undo.length ? { ...analysis.decision, actions: [
    ...(analysis.decision.actions || []).filter(action => action.action_ref?.kind !== 'untap_land'),
    ...undo,
  ].map((action, index) => ({ ...action, index })) } : analysis.decision;
  return { ...state,
    decision: analysis.sequence === undefined ? decision : { ...decision, __priority_analysis_sequence: analysis.sequence },
    __priority_analysis_sequence: analysis.sequence ?? 0,
  };
}

/** Reconcile either arrival order without waiting for a React render. */
export function subscribePriorityAnalysisSnapshots({ game, getState, setState, subscribeState,
  schedule = queueMicrotask }) {
  let queued = false, disposed = false;
  const reconcile = () => {
    if (queued || disposed) return;
    queued = true;
    schedule(() => {
      queued = false;
      if (disposed) return;
      const previous = getState();
      const next = mergePriorityAnalysis(previous, game.latestPriorityAnalysis());
      if (next !== previous) setState(next);
    });
  };
  const unsubscribeAnalysis = game.subscribePriorityAnalysis(reconcile);
  const unsubscribeState = subscribeState(reconcile);
  reconcile();
  return () => {
    disposed = true;
    unsubscribeAnalysis();
    unsubscribeState();
  };
}
