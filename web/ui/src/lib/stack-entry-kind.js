// What kind of thing waits on the stack, in one word: the stack tiles' sub-
// line and the card pile's pager both say it (Spell, Trigger, Activation...).
export function stackEntryKindLabel(entry) {
  const abilityKind = String(entry?.ability_kind || "").trim();
  const normalized = abilityKind.toLowerCase();
  if (!abilityKind) return "Spell";
  if (normalized === "triggered") return "Trigger";
  if (normalized === "activated") return "Activation";
  if (normalized === "replacement") return "Replacement";
  return `${abilityKind} ability`;
}
