import { getPlayerAccent, hexToRgbString } from "./player-colors.js";

const LOCAL_DECISION_ACCENT = "#731bde";

export function decisionButtonPlayerId(state, decision = state?.decision) {
  return (
    decision?.player
    ?? state?.priority_player
    ?? state?.perspective
    ?? null
  );
}

export function decisionButtonAccentVars(state, decision = state?.decision, accentOverrides = null) {
  const playerId = decisionButtonPlayerId(state, decision);
  if (Number(playerId) === Number(state?.perspective)) {
    return {
      "--decision-main-accent": LOCAL_DECISION_ACCENT,
      "--decision-main-rgb": hexToRgbString(LOCAL_DECISION_ACCENT),
    };
  }
  const accent = getPlayerAccent(state?.players || [], playerId, state?.perspective, accentOverrides);
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
  if (Number.isFinite(decisionPlayer)) return false;

  const priorityPlayer = Number(state?.priority_player);
  return Number.isFinite(priorityPlayer) && priorityPlayer === perspective;
}
