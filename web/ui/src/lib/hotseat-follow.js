import { samePlayerId } from "./player-display.js";

export const FOLLOW_ACTIVE_PLAYER_STORAGE_KEY = "ironsmith.followActivePlayer";

export function readFollowActivePlayer(storage = globalThis.localStorage) {
  try { return storage?.getItem(FOLLOW_ACTIVE_PLAYER_STORAGE_KEY) !== "false"; }
  catch { return true; }
}

export function storeFollowActivePlayer(enabled, storage = globalThis.localStorage) {
  try { storage?.setItem(FOLLOW_ACTIVE_PLAYER_STORAGE_KEY, String(Boolean(enabled))); }
  catch { /* The toggle still works for this session. */ }
}

const PREGAME_ACTION_KINDS = new Set([
  "keep_opening_hand", "take_mulligan", "use_pregame_action", "continue_pregame", "begin_game",
]);

function isPregameDecision(decision) {
  return (decision?.actions || []).some(action =>
    PREGAME_ACTION_KINDS.has(action?.kind) || PREGAME_ACTION_KINDS.has(action?.action_ref?.kind));
}

// Hotseat play for local games: the human sits in whichever seat has to act.
// The view follows the active player's priority (so ending a turn hands the
// table to the next player) and any non-priority choice of another seat, such
// as declaring blockers. The non-active player's plain priority passes stay
// automatic, otherwise every step would bounce the view between seats.
export function hotseatFollowTarget(state) {
  const decision = state?.decision;
  if (!decision || state.game_over || decision.player == null) return null;
  if (samePlayerId(decision.player, state.perspective)) return null;
  if (decision.kind === "priority") {
    if (!samePlayerId(decision.player, state.active_player) || isPregameDecision(decision)) return null;
  } else if (decision.kind === "mana_payment") {
    return null;
  }
  const player = Number(decision.player);
  return Number.isInteger(player) && player >= 0 ? player : null;
}
