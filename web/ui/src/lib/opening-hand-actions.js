// Opening-hand (pregame mulligan) decisions arrive as a priority decision whose
// advance action is "Keep hand" (surfaced with kind `pass_priority`) plus a
// sibling `take_mulligan` action with no object attached. The action strips
// only surface object-linked actions through the board, so the mulligan needs
// a dedicated control next to the keep button.

function isKeepOpeningHandAction(action) {
  return Boolean(action) && (
    action.kind === "keep_opening_hand"
    || action?.action_ref?.kind === "keep_opening_hand"
    || /^keep\s+hand$/i.test(String(action.label || "").trim())
  );
}

function isTakeMulliganAction(action) {
  return Boolean(action) && (
    action.kind === "take_mulligan"
    || action?.action_ref?.kind === "take_mulligan"
    || /^mulligan$/i.test(String(action.label || "").trim())
  );
}

export function isOpeningHandDecision(actions = [], passAction = null) {
  return isKeepOpeningHandAction(passAction)
    || (actions || []).some(isKeepOpeningHandAction);
}

export function findOpeningHandMulliganAction(actions = [], passAction = null) {
  if (!isOpeningHandDecision(actions, passAction)) return null;
  return (actions || []).find(isTakeMulliganAction) || null;
}

// Pregame abilities can be offered both during mulligans and after keeping.
export function findPregameActions(actions = []) {
  return (actions || []).filter((action) =>
    action.kind === "use_pregame_action"
    || action?.action_ref?.kind === "use_pregame_action"
  );
}

export function canShowPhasePass(state) {
  const decision = state?.decision;
  const actions = decision?.actions || [];
  const pregameKinds = ["keep_opening_hand", "take_mulligan", "use_pregame_action", "continue_pregame", "begin_game"];
  return decision?.kind === "priority"
    && !state?.game_over
    && Number(state?.stack_size || 0) === 0
    && !isOpeningHandDecision(actions)
    && !actions.some(action => pregameKinds.includes(action.kind) || pregameKinds.includes(action.action_ref?.kind));
}
