import { castingMethodChoiceForAction } from "./casting-method-choice.js";
/** Preview each cast route in a separate WASM instance, preserving the live game. */
export function previewCastTargetDecision(PreviewGame, checkpoint, perspective, actions, registerSources) {
  const preview = new PreviewGame();
  try {
    registerSources(preview);
    const requirements = [];
    for (const action of actions) {
      preview.importSyncCheckpoint(checkpoint, perspective);
      let result = preview.dispatch({
        type: "priority_action", action_index: action.index, action_ref: action.action_ref,
      });
      const methodChoice = castingMethodChoiceForAction(result?.decision, action);
      if (methodChoice) result = preview.dispatch(methodChoice);
      if (result?.decision?.kind === "targets") requirements.push(...result.decision.requirements);
    }
    return { kind: "targets", player: perspective, requirements };
  } finally {
    preview.free();
  }
}
