const COLOR_NAMES = { W: "white", U: "blue", B: "black", R: "red", G: "green", C: "colorless" };

/** The permanent whose mana ability an action activates. */
export function manaActionSourceId(action) {
  const ref = action?.action_ref;
  const id = action?.source_id ?? ref?.source ?? ref?.permanent_id ?? action?.object_id;
  return id == null ? null : String(id);
}

/**
 * A mana strip click names its color up front. When activating "Add one mana
 * of any color" then asks for that color, answer with the one already picked
 * instead of showing the prompt. Anything else stays with the player.
 */
export function manaColorChoiceCommand(decision, choice) {
  const letter = String(choice?.color || "").toUpperCase();
  const name = COLOR_NAMES[letter];
  if (!name || decision?.kind !== "select_options" || Number(decision.max) !== 1) return null;
  if (decision.choice_kind !== "colors" && !/\bmana\b/i.test(`${decision.description || ""} ${decision.context_text || ""}`)) return null;
  if (decision.source_id != null && choice.sourceId != null && String(decision.source_id) !== String(choice.sourceId)) return null;
  const option = (decision.options || []).find(option => option.legal !== false && (
    String(option.description || "").trim().toLowerCase() === name
    || String(option.description || "").trim().toUpperCase() === `{${letter}}`));
  return option ? { type: "select_options", option_indices: [option.index] } : null;
}
