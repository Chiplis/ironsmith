export const PHASE_TRACK = [
  "Untap",
  "Upkeep",
  "Draw",
  "Main",
  "Combat",
  "Main2",
  "End",
  "Cleanup",
];

export const MANA_SYMBOLS = [
  { key: "white", symbol: "W", label: "White", svg: "https://svgs.scryfall.io/card-symbols/W.svg" },
  { key: "blue", symbol: "U", label: "Blue", svg: "https://svgs.scryfall.io/card-symbols/U.svg" },
  { key: "black", symbol: "B", label: "Black", svg: "https://svgs.scryfall.io/card-symbols/B.svg" },
  { key: "red", symbol: "R", label: "Red", svg: "https://svgs.scryfall.io/card-symbols/R.svg" },
  { key: "green", symbol: "G", label: "Green", svg: "https://svgs.scryfall.io/card-symbols/G.svg" },
  { key: "colorless", symbol: "C", label: "Colorless", svg: "https://svgs.scryfall.io/card-symbols/C.svg" },
];

export function normalizePhaseStep(phase, step) {
  if (step === "Untap") return "Untap";
  if (step === "Upkeep") return "Upkeep";
  if (step === "Draw") return "Draw";
  if (
    step === "BeginCombat" ||
    step === "DeclareAttackers" ||
    step === "DeclareBlockers" ||
    step === "CombatDamage" ||
    step === "EndCombat"
  )
    return "Combat";
  if (step === "End") return "End";
  if (step === "Cleanup") return "Cleanup";
  if (phase === "FirstMain") return "Main";
  if (phase === "NextMain") return "Main2";
  if (phase === "Ending") return "End";
  return "Main";
}

export function formatStep(step) {
  return step ? step.replace(/([a-z])([A-Z])/g, "$1 $2") : "None";
}

export function parseNames(raw) {
  return raw
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
}
