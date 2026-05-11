import { isCombatPhase, isEndingPhase, isMainPhase } from "./constants.js";
import { priorityCommandForAction } from "./sync-commands.js";

export const LOCAL_STACK_MANUAL_HOLD_REASON = "manual stack resolve";
export const LOCAL_EMPTY_STACK_HOLD_REASON = "local empty-stack priority";
export const OPPONENT_STACK_HOLD_REASON = "opponent stack action";
export const UNKNOWN_STACK_HOLD_REASON = "unknown stack action";
export const NO_PASS_ACTION_HOLD_REASON = "no pass action available";
export const CUSTOM_PASS_ACTION_HOLD_REASON = "custom pass action";
export const INVALID_PASS_ACTION_HOLD_REASON = "invalid pass action";

function toPlayerNumber(value) {
  const player = Number(value);
  return Number.isFinite(player) ? player : null;
}

function stackSizeOf(currentState) {
  const stackSize = Number(currentState?.stack_size || 0);
  return Number.isFinite(stackSize) ? stackSize : 0;
}

function topStackObject(currentState) {
  return Array.isArray(currentState?.stack_objects) && currentState.stack_objects.length > 0
    ? currentState.stack_objects[0]
    : null;
}

export function topStackController(currentState) {
  return toPlayerNumber(topStackObject(currentState)?.controller);
}

export function findPassPriorityAction(decision) {
  return (decision?.actions || []).find((action) => action.kind === "pass_priority") || null;
}

export function priorityHoldReason({
  autoPassEnabled,
  holdRule,
  decision,
  currentState,
  perspectiveMode = "any",
  requireNonEmptyStack = false,
  manualResolveOnLocalStack = false,
}) {
  if (!autoPassEnabled) return "auto-pass disabled";
  if (!decision || decision.kind !== "priority") return "not a priority decision";

  const perspective = toPlayerNumber(currentState?.perspective);
  const decisionPlayer = toPlayerNumber(decision.player);
  if (perspectiveMode === "local" && decisionPlayer !== perspective) return "not local priority";
  if (perspectiveMode === "opponent" && decisionPlayer === perspective) return "not opponent priority";

  const stackSize = stackSizeOf(currentState);
  if (manualResolveOnLocalStack && perspectiveMode === "local" && stackSize > 0) {
    return LOCAL_STACK_MANUAL_HOLD_REASON;
  }
  if (requireNonEmptyStack && stackSize <= 0) return "stack empty";

  if (holdRule === "never") return null;
  if (holdRule === "always") return "always hold";
  if (holdRule === "stack" && stackSize > 0) return "stack non-empty";
  if (holdRule === "main" && isMainPhase(currentState?.phase)) return "main phase";
  if (holdRule === "combat" && isCombatPhase(currentState?.phase)) return "combat phase";
  if (holdRule === "ending" && isEndingPhase(currentState?.phase)) return "ending phase";
  if (holdRule === "if_actions") {
    const hasNonPass = (decision.actions || []).some((action) => action.kind !== "pass_priority");
    if (hasNonPass) {
      return perspectiveMode === "opponent"
        ? "opponent has playable actions"
        : "playable actions available";
    }
  }

  return null;
}

export function buildMultiplayerSmartAutoPass({
  autoPassEnabled,
  holdRule,
  decision,
  currentState,
}) {
  const baseHoldReason = priorityHoldReason({
    autoPassEnabled,
    holdRule,
    decision,
    currentState,
    perspectiveMode: "local",
  });
  if (baseHoldReason) {
    return { command: null, holdReason: baseHoldReason, passAction: null };
  }

  const passAction = findPassPriorityAction(decision);
  if (!passAction) {
    return { command: null, holdReason: NO_PASS_ACTION_HOLD_REASON, passAction: null };
  }
  if (passAction.label && passAction.label !== "Pass priority") {
    return { command: null, holdReason: CUSTOM_PASS_ACTION_HOLD_REASON, passAction };
  }

  const actionIndex = Number(passAction.index);
  if (!Number.isFinite(actionIndex)) {
    return { command: null, holdReason: INVALID_PASS_ACTION_HOLD_REASON, passAction };
  }

  const perspective = toPlayerNumber(currentState?.perspective);
  const activePlayer = toPlayerNumber(currentState?.active_player);
  const stackSize = stackSizeOf(currentState);

  if (stackSize <= 0) {
    if (perspective !== null && activePlayer === perspective) {
      return { command: null, holdReason: LOCAL_EMPTY_STACK_HOLD_REASON, passAction };
    }
    return {
      command: priorityCommandForAction(passAction),
      holdReason: null,
      passAction,
    };
  }

  const stackController = topStackController(currentState);
  if (stackController === null || perspective === null) {
    return { command: null, holdReason: UNKNOWN_STACK_HOLD_REASON, passAction };
  }
  if (stackController !== perspective) {
    return { command: null, holdReason: OPPONENT_STACK_HOLD_REASON, passAction };
  }

  return {
    command: priorityCommandForAction(passAction),
    holdReason: null,
    passAction,
  };
}
