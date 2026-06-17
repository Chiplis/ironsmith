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

const PHASE_KEYS = {
  beginning: "Beginning",
  beginningphase: "Beginning",
  firstmain: "FirstMain",
  firstmainphase: "FirstMain",
  combat: "Combat",
  combatphase: "Combat",
  nextmain: "NextMain",
  nextmainphase: "NextMain",
  secondmainphase: "NextMain",
  ending: "Ending",
  endingphase: "Ending",
};

const STEP_KEYS = {
  untap: "Untap",
  untapstep: "Untap",
  upkeep: "Upkeep",
  upkeepstep: "Upkeep",
  draw: "Draw",
  drawstep: "Draw",
  begincombat: "BeginCombat",
  begincombatstep: "BeginCombat",
  declareattackers: "DeclareAttackers",
  declareattackersstep: "DeclareAttackers",
  declareblockers: "DeclareBlockers",
  declareblockersstep: "DeclareBlockers",
  combatdamage: "CombatDamage",
  combatdamagestep: "CombatDamage",
  endcombat: "EndCombat",
  endcombatstep: "EndCombat",
  end: "End",
  endstep: "End",
  cleanup: "Cleanup",
  cleanupstep: "Cleanup",
};

const PHASE_LABELS = {
  Beginning: "Beginning",
  FirstMain: "Pre-Main",
  Combat: "Combat",
  NextMain: "Post-Main",
  Ending: "Ending",
};

const STEP_LABELS = {
  Untap: "Untap",
  Upkeep: "Upkeep",
  Draw: "Draw",
  BeginCombat: "Begin Combat",
  DeclareAttackers: "Declare Attackers",
  DeclareBlockers: "Declare Blockers",
  CombatDamage: "Combat Damage",
  EndCombat: "End Combat",
  End: "End",
  Cleanup: "Cleanup",
};

function translatedLabel(t, key, fallback) {
  return typeof t === "function" ? t(key, null, fallback) : fallback;
}

function turnTokenKey(value) {
  if (typeof value !== "string") return "";
  return value.trim().toLowerCase().replace(/[\s_-]+/g, "");
}

export function normalizePhaseKey(phase) {
  return PHASE_KEYS[turnTokenKey(phase)] || null;
}

export function normalizeStepKey(step) {
  return STEP_KEYS[turnTokenKey(step)] || null;
}

export function normalizePhaseStep(phase, step) {
  const normalizedStep = normalizeStepKey(step);
  const normalizedPhase = normalizePhaseKey(phase);

  if (normalizedStep === "Untap") return "Untap";
  if (normalizedStep === "Upkeep") return "Upkeep";
  if (normalizedStep === "Draw") return "Draw";
  if (
    normalizedStep === "BeginCombat" ||
    normalizedStep === "DeclareAttackers" ||
    normalizedStep === "DeclareBlockers" ||
    normalizedStep === "CombatDamage" ||
    normalizedStep === "EndCombat"
  )
    return "Combat";
  if (normalizedStep === "End") return "End";
  if (normalizedStep === "Cleanup") return "Cleanup";
  if (normalizedPhase === "FirstMain") return "Main";
  if (normalizedPhase === "NextMain") return "Main2";
  if (normalizedPhase === "Ending") return "End";
  return "Main";
}

export function nextPriorityAdvanceLabel(phase, step, stackSize, t = null) {
  if (stackSize > 0) return translatedLabel(t, "action.resolve", "Resolve");

  switch (normalizeStepKey(step)) {
    case "Untap": return translatedLabel(t, "game.step.Upkeep", "Upkeep");
    case "Upkeep": return translatedLabel(t, "game.step.Draw", "Draw");
    case "Draw": return translatedLabel(t, "game.advance.mainPhase", "Pre-Main");
    case "BeginCombat": return translatedLabel(t, "game.advance.attackers", "Attackers");
    case "DeclareAttackers": return translatedLabel(t, "game.advance.blockers", "Blockers");
    case "DeclareBlockers": return translatedLabel(t, "game.advance.damage", "Damage");
    case "CombatDamage": return translatedLabel(t, "game.step.EndCombat", "End Combat");
    case "EndCombat": return translatedLabel(t, "game.advance.main2", "Post-Main");
    case "End": return translatedLabel(t, "game.step.Cleanup", "Cleanup");
    case "Cleanup": return translatedLabel(t, "game.advance.nextTurn", "Next Turn");
    default: break;
  }

  switch (normalizePhaseKey(phase)) {
    case "FirstMain": return translatedLabel(t, "game.phase.Combat", "Combat");
    case "NextMain": return translatedLabel(t, "game.step.End", "End Step");
    case "Ending": return translatedLabel(t, "game.step.Cleanup", "Cleanup");
    default: return translatedLabel(t, "game.advance.next", "Next");
  }
}

export function currentPriorityPhaseLabel(phase, step, t = null) {
  const normalizedStep = normalizeStepKey(step);
  if (normalizedStep) {
    return translatedLabel(t, `game.step.${normalizedStep}`, STEP_LABELS[normalizedStep]);
  }

  const normalizedPhase = normalizePhaseKey(phase);
  if (normalizedPhase) {
    return translatedLabel(t, `game.phase.${normalizedPhase}`, PHASE_LABELS[normalizedPhase]);
  }

  return phase ? String(phase).replace(/([a-z])([A-Z])/g, "$1 $2") : translatedLabel(t, "action.priority", "Priority");
}

export function priorityPassButtonColor(phase, step, stackSize) {
  if (stackSize > 0) return "yellow";
  if (normalizePhaseKey(phase) === "FirstMain" && !normalizeStepKey(step)) return "red";

  switch (normalizeStepKey(step)) {
    case "BeginCombat":
    case "DeclareAttackers":
      return "blue";
    case "DeclareBlockers":
    case "CombatDamage":
      return "orange";
    default:
      return "yellow";
  }
}

export function isMainPhase(phase) {
  const normalizedPhase = normalizePhaseKey(phase);
  return normalizedPhase === "FirstMain" || normalizedPhase === "NextMain";
}

export function isCombatPhase(phase) {
  return normalizePhaseKey(phase) === "Combat";
}

export function isEndingPhase(phase) {
  return normalizePhaseKey(phase) === "Ending";
}

export function formatPhase(phase, t = null) {
  const normalizedPhase = normalizePhaseKey(phase);
  if (normalizedPhase) {
    return translatedLabel(t, `game.phase.${normalizedPhase}`, PHASE_LABELS[normalizedPhase]);
  }
  return phase ? String(phase).replace(/([a-z])([A-Z])/g, "$1 $2") : "Unknown";
}

export function formatStep(step, t = null) {
  const normalizedStep = normalizeStepKey(step);
  if (normalizedStep) {
    return translatedLabel(t, `game.step.${normalizedStep}`, STEP_LABELS[normalizedStep]);
  }
  return step ? String(step).replace(/([a-z])([A-Z])/g, "$1 $2") : "None";
}

export function parseNames(raw) {
  return raw
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
}
