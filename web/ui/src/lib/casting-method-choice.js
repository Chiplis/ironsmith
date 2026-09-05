/** Submit an already-chosen method inside the originating interaction gate. */
export async function finishExplicitCastingMethod(state, action, dispatch) {
  const command = castingMethodChoiceForAction(state?.decision, action);
  if (!command) return state;
  return dispatch(command);
}

/** Resolve the engine's generic-cast prompt after an explicit normal-cost choice. */
export function castingMethodChoiceForAction(decision, action) {
  const ref = action?.action_ref;
  if (ref?.kind !== "cast_spell" || ref.casting_method?.kind !== "normal") return null;
  if (decision?.kind !== "select_options"
      || !/^Choose casting method for /i.test(decision.description || "")
      || decision.source_id == null || Number(decision.source_id) !== Number(ref.spell_id)) return null;
  // collect_available_casting_methods puts the legal Normal method first,
  // including split cards whose option is labeled with their face name.
  const normal = decision.options?.[0];
  if (!normal || normal.legal === false) return null;
  return { type: "select_options", option_indices: [normal.index] };
}
