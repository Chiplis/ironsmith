export function decisionButtonPlayerId(state, decision = state?.decision) {
  return (
    decision?.player
    ?? state?.priority_player
    ?? state?.perspective
    ?? null
  );
}

// Main actions use a consistent yellow accent, independent of player identity.
export function decisionButtonAccentVars() {
  return {
    "--decision-main-accent": "#ffe083",
    "--decision-main-rgb": "255, 224, 131",
    "--player-accent": "#ffe083",
    "--panel-accent": "#ffe083",
    "--player-accent-rgb": "255, 224, 131",
  };
}

export function useDecisionButtonAccent(state, decision = state?.decision) {
  return {
    style: decisionButtonAccentVars(),
    isLocal: isLocalDecisionButton(state, decision),
  };
}

export function isLocalDecisionButton(state, decision = state?.decision) {
  const perspective = Number(state?.perspective);
  if (!Number.isFinite(perspective)) return false;

  const decisionPlayer = Number(decision?.player);
  if (Number.isFinite(decisionPlayer) && decisionPlayer === perspective) return true;
  if (Number.isFinite(decisionPlayer)) return false;

  const priorityPlayer = Number(state?.priority_player);
  return Number.isFinite(priorityPlayer) && priorityPlayer === perspective;
}
