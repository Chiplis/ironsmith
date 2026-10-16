// Hand/zone snapshots carry card_types rather than the inspector's type_line.
// Prefer current engine details, then source metadata, then the typed snapshot.
export function cardDisplayTypeLine(card, fallback = "", engineTypeLine = "") {
  if (card?.face_down || card?.name === "Hidden card") return "";
  if (fallback && engineTypeLine && card?.type_line === engineTypeLine) return fallback;
  const explicit = String(card?.type_line_display || card?.type_line || "").trim();
  if (explicit && explicit !== "Card") return explicit;
  if (fallback) return fallback;
  // Lightweight snapshots use lowercase engine enum names and keep their
  // live subtypes in the characteristic signature, not separate arrays.
  const signature = Object.fromEntries(String(card?.characteristic_signature || '').split('\n')
    .filter(line => /^(supertypes|types|subtypes):/.test(line))
    .map(line => {const colon=line.indexOf(':');return [line.slice(0,colon),line.slice(colon+1).split(',').filter(value=>value && value!=='-')];}));
  const title = value => String(value).replace(/\b[a-z]/g, letter => letter.toUpperCase());
  const main = [...(signature.supertypes ?? card?.supertypes ?? []),
    ...(signature.types ?? card?.card_types ?? card?.cardTypes ?? [])].map(title).join(" ");
  const subtypes = (signature.subtypes ?? card?.subtypes ?? []).map(title).join(" ");
  return main + (main && subtypes ? " — " + subtypes : "");
}
