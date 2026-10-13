const key = "ironsmith-custom-card-flavor-v1";
let memory = {};
const normalized = name => String(name || "").trim().toLowerCase();

function read() {
  try { return {...JSON.parse(globalThis.localStorage?.getItem(key) || "{}"), ...memory}; }
  catch { return memory; }
}

export function registerCustomCardFlavor(cards) {
  memory = read();
  for (const card of cards) {
    for (const face of card.draft.faces) memory[normalized(face.name)] = face.flavorText || card.source?.post || "";
  }
  try { globalThis.localStorage?.setItem(key, JSON.stringify(memory)); } catch { /* memory fallback */ }
}

export function customCardFlavorText(name) {
  const value = read()[normalized(name)];
  return typeof value === "string" ? value : "";
}
