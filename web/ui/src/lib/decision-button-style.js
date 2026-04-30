import { getPlayerAccent } from "@/lib/player-colors";

export function decisionButtonPlayerId(state, decision = state?.decision) {
  return (
    state?.priority_player
    ?? decision?.player
    ?? state?.active_player
    ?? state?.perspective
    ?? null
  );
}

export function decisionButtonAccentVars(state, decision = state?.decision) {
  const playerId = decisionButtonPlayerId(state, decision);
  const accent = getPlayerAccent(state?.players || [], playerId);
  if (!accent) return undefined;

  return {
    "--decision-main-accent": accent.hex,
    "--decision-main-rgb": accent.rgb,
  };
}

export function isLocalDecisionButton(state, decision = state?.decision) {
  const perspective = Number(state?.perspective);
  if (!Number.isFinite(perspective)) return false;

  const decisionPlayer = Number(decision?.player);
  if (Number.isFinite(decisionPlayer) && decisionPlayer === perspective) return true;

  const priorityPlayer = Number(state?.priority_player);
  return Number.isFinite(priorityPlayer) && priorityPlayer === perspective;
}
