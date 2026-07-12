import { normalizePlayerId } from "../wasm-test-harness.mjs";

export const PLAYER_NAMES = ["Alice", "Bob", "Charlie", "Dana"];
export const MAX_ADVANCE_STEPS = 600;
export const DEFAULT_LIBRARY_CARD = "Plains";
export const DEFAULT_LIBRARY_SIZE = 60;
export const ALLOW_ENGINE_SHIMS = process.env.MAGE_PORT_ALLOW_ENGINE_SHIMS === "1";

export const CARD_FIXTURES = new Map([
  ["Archetype of Courage", { manaCost: "{1}{W}{W}", typeLine: "Enchantment Creature - Human Soldier", oracleText: "First strike\nCreatures you control have first strike.", power: "2", toughness: "2" }],
  ["Mox Sapphire", { manaCost: "{0}", typeLine: "Artifact", oracleText: "{T}: Add {U}.", power: null, toughness: null }],
  ["Ancestral Recall", { manaCost: "{U}", typeLine: "Instant", oracleText: "", power: null, toughness: null }],
  ["Speedway Fanatic", { manaCost: "{1}{R}", typeLine: "Creature - Human Pilot", oracleText: "Haste", power: "2", toughness: "1" }],
  ["Giant Ox", { manaCost: "{1}{W}", typeLine: "Creature - Ox", oracleText: "", power: "6", toughness: "6" }],
  ["Kotori, Pilot Prodigy", { manaCost: "{1}{W}{U}", typeLine: "Legendary Creature - Moonfolk Pilot", oracleText: "", power: "2", toughness: "4" }],
  ["Irontread Crusher", { manaCost: "{4}", typeLine: "Artifact - Vehicle", oracleText: "Lifelink\nVigilance\nCrew 2", power: "6", toughness: "6" }],
  ["Hotshot Mechanic", { manaCost: "{W}", typeLine: "Artifact Creature - Fox Pilot", oracleText: "", power: "4", toughness: "1" }],
  ["New Perspectives", { manaCost: "{5}{U}", typeLine: "Enchantment", oracleText: "When this enchantment enters, draw three cards.", power: null, toughness: null }],
  ["Moonmist", { manaCost: "{1}{G}", typeLine: "Instant", oracleText: "", power: null, toughness: null }],
  ["Brimstone Vandal", { manaCost: "{2}{R}", typeLine: "Creature - Devil", oracleText: "", power: "2", toughness: "3" }],
]);

export const DAY_NIGHT_TRANSFORMS = new Map([
  ["Tavern Ruffian", { front: "Tavern Ruffian", back: "Tavern Smasher", frontPower: 2, frontToughness: 5, backPower: 6, backToughness: 5 }],
  ["Tavern Smasher", { front: "Tavern Ruffian", back: "Tavern Smasher", frontPower: 2, frontToughness: 5, backPower: 6, backToughness: 5 }],
  ["Curse of Leeches", { front: "Curse of Leeches", back: "Leeching Lurker", frontPower: null, frontToughness: null, backPower: 4, backToughness: 4 }],
  ["Leeching Lurker", { front: "Curse of Leeches", back: "Leeching Lurker", frontPower: null, frontToughness: null, backPower: 4, backToughness: 4 }],
  ["Grizzled Outcasts", { front: "Grizzled Outcasts", back: "Krallenhorde Wantons", frontPower: 4, frontToughness: 4, backPower: 7, backToughness: 7 }],
  ["Krallenhorde Wantons", { front: "Grizzled Outcasts", back: "Krallenhorde Wantons", frontPower: 4, frontToughness: 4, backPower: 7, backToughness: 7 }],
]);

export const PHASE_NAMES = new Set([
  "UNTAP", "UPKEEP", "DRAW", "PRECOMBAT_MAIN", "BEGIN_COMBAT", "DECLARE_ATTACKERS",
  "DECLARE_BLOCKERS", "COMBAT_DAMAGE", "END_COMBAT", "POSTCOMBAT_MAIN", "END_TURN", "CLEANUP",
]);

export function playerIndex(player) {
  return normalizePlayerId(player);
}

export function playerName(index) {
  return PLAYER_NAMES[index] ?? `Player ${index}`;
}

export function cardName(raw) {
  const text = String(raw ?? "");
  if (/EmptyNames\.FACE_DOWN_TOKEN\.getTestCommand\(\)/.test(text)) return "Face-down Token";
  if (/EmptyNames\.FACE_DOWN_CREATURE\.getTestCommand\(\)/.test(text)) return "Face-down Creature";
  if (/EmptyNames\.FULLY_LOCKED_ROOM\.getTestCommand\(\)/.test(text)) return "Fully Locked Room";
  const unquoted = text.replace(/^"(.+)"$/, "$1").replace(/^'(.+)'$/, "$1");
  const aliasIndex = unquoted.indexOf("@");
  return (aliasIndex > 0 ? unquoted.slice(0, aliasIndex) : unquoted)
    .replace(/\s+using\s+.+$/i, "")
    .replace(/\s+with\s+(?:alternative cost.*|awaken|blitz|emerge|escape|freerunning.*|jump-start|kicker|mayhem|no alternative cost|overload|prowl.*|prototype|retrace|spectacle|surge|warp|web-slinging)$/i, "")
    .replace(/^Cast\s+/i, "");
}

export function zoneName(zone) {
  const raw = String(zone).replace(/^Zone\./, "").toLowerCase();
  if (["outside_game", "battlefield", "graveyard", "library", "command"].includes(raw)) return raw;
  if (raw === "exile" || raw === "exiled") return "exile";
  return "hand";
}

export function phaseLabel(phase) {
  return String(phase || "PRECOMBAT_MAIN").replace(/^PhaseStep\./, "");
}

export function magePlayerName(player) {
  return `Player${String.fromCharCode("A".charCodeAt(0) + playerIndex(player))}`;
}

export function normalizePhase(phase, step) {
  const combined = `${phase || ""} ${step || ""}`.toLowerCase();
  if (combined.includes("upkeep")) return "UPKEEP";
  if (combined.includes("draw")) return "DRAW";
  if (combined.includes("first main") || combined.includes("precombat")) return "PRECOMBAT_MAIN";
  if (combined.includes("begin combat")) return "BEGIN_COMBAT";
  if (combined.includes("declare attackers")) return "DECLARE_ATTACKERS";
  if (combined.includes("declare blockers")) return "DECLARE_BLOCKERS";
  if (combined.includes("combat damage")) return "COMBAT_DAMAGE";
  if (combined.includes("end combat")) return "END_COMBAT";
  if (combined.includes("second main") || combined.includes("postcombat")) return "POSTCOMBAT_MAIN";
  if (combined.includes("end step")) return "END_TURN";
  if (combined.includes("cleanup")) return "CLEANUP";
  return combined.toUpperCase();
}

export function phaseOrder(phase) {
  return [...PHASE_NAMES].indexOf(phaseLabel(phase));
}

export function currentPositionIsAfter(state, turn, phase) {
  const currentTurn = Number(state.turn_number || 1);
  const targetTurn = Number(turn || 1);
  if (currentTurn !== targetTurn) return currentTurn > targetTurn;
  return phaseOrder(normalizePhase(state.phase, state.step)) > phaseOrder(phase);
}
