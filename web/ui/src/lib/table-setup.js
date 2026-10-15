// The local table the player sets up in the menu (seat names and starting
// life). Reset Match and the startup board both read it, and it survives
// reloads. Puzzle links and goldfish loads change the live names without
// touching this saved setup.
const KEY = "ironsmith.tableSetup";

export const MIN_TABLE_SEATS = 2;
export const MAX_TABLE_SEATS = 4;
export const DEFAULT_TABLE_SETUP = Object.freeze({ playerNames: "Alice,Bob", startingLife: 20 });

export function defaultSeatName(index) {
  return `Player ${index + 1}`;
}

// One name per seat; a blank entry is still a seat and gets a default name.
export function tableSeatNames(raw) {
  const names = String(raw ?? "").split(",").map(name => name.trim());
  while (names.length < MIN_TABLE_SEATS) names.push("");
  return names.slice(0, MAX_TABLE_SEATS).map((name, index) => name || defaultSeatName(index));
}

export function readTableSetup(storage = globalThis.localStorage) {
  try {
    const saved = JSON.parse(storage?.getItem(KEY) || "null");
    if (!saved || typeof saved !== "object") return { ...DEFAULT_TABLE_SETUP };
    const life = Math.floor(Number(saved.startingLife));
    return {
      playerNames: typeof saved.playerNames === "string" && saved.playerNames.trim()
        ? tableSeatNames(saved.playerNames).join(",")
        : DEFAULT_TABLE_SETUP.playerNames,
      startingLife: Number.isFinite(life) && life > 0 ? life : DEFAULT_TABLE_SETUP.startingLife,
    };
  } catch {
    return { ...DEFAULT_TABLE_SETUP };
  }
}

export function storeTableSetup(setup, storage = globalThis.localStorage) {
  try {
    storage?.setItem(KEY, JSON.stringify({
      playerNames: String(setup.playerNames ?? DEFAULT_TABLE_SETUP.playerNames),
      startingLife: Number(setup.startingLife) || DEFAULT_TABLE_SETUP.startingLife,
    }));
  } catch { /* The setup still applies for this session. */ }
}
