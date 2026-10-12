// Hand/zone snapshots carry card_types rather than the inspector's type_line.
// Prefer current engine details, then source metadata, then the typed snapshot.
export function cardDisplayTypeLine(card, fallback = "", engineTypeLine = "") {
  if (card?.face_down || card?.name === "Hidden card") return "";
  if (fallback && engineTypeLine && card?.type_line === engineTypeLine) return fallback;
  const explicit = String(card?.type_line_display || card?.type_line || "").trim();
  if (explicit && explicit !== "Card") return explicit;
  if (fallback) return fallback;
  const main = [...(card?.supertypes || []), ...(card?.card_types || card?.cardTypes || [])].join(" ");
  const subtypes = (card?.subtypes || []).join(" ");
  return main + (main && subtypes ? " — " + subtypes : "");
}
