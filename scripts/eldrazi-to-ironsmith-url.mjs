#!/usr/bin/env node

import { pathToFileURL } from "node:url";

const DEFAULT_IRONSMITH_URL = "https://chiplis.com/ironsmith/";
const PUZZLE_ZONES = [
  "battlefield",
  "hand",
  "graveyard",
  "exile",
  "library",
  "command",
  "ante",
];

function usage() {
  return `Usage: node scripts/eldrazi-to-ironsmith-url.mjs <eldrazi-url> [options]

Options:
  --base-url <url>    Ironsmith installation to link to
                      (default: ${DEFAULT_IRONSMITH_URL})
  --save-slot <1-4>  Use this Eldrazi save-state slot instead of the URL/default
  --omit-library     Leave the library out of the generated URL
  --help             Show this help`;
}

function parsePositiveInteger(raw, label) {
  const value = Number(raw);
  if (!Number.isInteger(value) || value < 1) {
    throw new Error(`${label} must be a positive integer`);
  }
  return value;
}

export function parseCliArgs(argv) {
  const options = {
    baseUrl: DEFAULT_IRONSMITH_URL,
    omitLibrary: false,
    saveSlot: null,
    sourceUrl: "",
  };

  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--help" || argument === "-h") {
      options.help = true;
    } else if (argument === "--omit-library") {
      options.omitLibrary = true;
    } else if (argument === "--base-url") {
      const value = argv[index + 1];
      if (!value) throw new Error("--base-url requires a URL");
      options.baseUrl = value;
      index += 1;
    } else if (argument === "--save-slot") {
      const value = argv[index + 1];
      if (!value) throw new Error("--save-slot requires a slot number");
      options.saveSlot = parsePositiveInteger(value, "--save-slot");
      index += 1;
    } else if (argument.startsWith("-")) {
      throw new Error(`Unknown option: ${argument}`);
    } else if (!options.sourceUrl) {
      options.sourceUrl = argument;
    } else {
      throw new Error(`Unexpected argument: ${argument}`);
    }
  }

  return options;
}

export function parseEldraziUrl(rawUrl) {
  let url;
  try {
    url = new URL(rawUrl);
  } catch {
    throw new Error("The Eldrazi URL is not valid");
  }

  const hostname = url.hostname.toLowerCase();
  if (hostname !== "eldrazi.gg" && hostname !== "www.eldrazi.gg") {
    throw new Error(`Expected an eldrazi.gg URL, received ${url.hostname}`);
  }
  if (url.pathname !== "/playtest") {
    throw new Error("Expected an Eldrazi /playtest URL");
  }

  const handId = String(url.searchParams.get("handId") || "").trim();
  if (!handId) {
    throw new Error("The Eldrazi URL must contain a handId parameter");
  }

  const rawSaveState = String(url.searchParams.get("saveState") || "").trim();
  return {
    handId,
    origin: url.origin,
    saveSlot: rawSaveState ? parsePositiveInteger(rawSaveState, "saveState") : null,
    url,
  };
}

function cardNames(cards) {
  if (!Array.isArray(cards)) return [];
  return cards
    .map((card) => typeof card === "string" ? card : card?.name)
    .map((name) => String(name || "").trim())
    .filter(Boolean);
}

function emptyZones() {
  return Object.fromEntries(PUZZLE_ZONES.map((zone) => [zone, []]));
}

function zonesFromSavedState(savedState, omitLibrary) {
  const zones = emptyZones();
  for (const zone of PUZZLE_ZONES) {
    if (zone === "library" && omitLibrary) continue;
    zones[zone] = cardNames(savedState?.zones?.[zone]);
  }
  return zones;
}

function zonesFromInitialState(state, omitLibrary) {
  const zones = emptyZones();
  zones.battlefield = cardNames(state?.battlefieldCards || state?.zones?.battlefield);
  zones.hand = cardNames(state?.handCards || state?.zones?.hand);
  zones.graveyard = cardNames(state?.graveyardCards || state?.zones?.graveyard);
  zones.exile = cardNames(state?.exileCards || state?.zones?.exile);
  zones.command = cardNames(state?.commandCards || state?.zones?.command);
  zones.ante = cardNames(state?.anteCards || state?.zones?.ante);
  if (!omitLibrary) {
    zones.library = cardNames(state?.libraryCards || state?.zones?.library);
  }
  return zones;
}

function commanderStartingLife(state) {
  const hasCommander = cardNames(state?.commandCards).length > 0
    || cardNames(state?.commanders).length > 0;
  return hasCommander ? 40 : 20;
}

export function selectEldraziState(response, requestedSaveSlot = null) {
  const slots = Array.isArray(response?.saveStateSlots) ? response.saveStateSlots : [];
  let slotIndex = null;

  if (requestedSaveSlot != null) {
    slotIndex = requestedSaveSlot - 1;
    if (slotIndex < 0 || slotIndex >= slots.length || !slots[slotIndex]) {
      throw new Error(`Eldrazi save-state slot ${requestedSaveSlot} is empty or does not exist`);
    }
  } else if (Number.isInteger(response?.defaultSaveSlotIndex)) {
    const defaultIndex = response.defaultSaveSlotIndex;
    if (defaultIndex >= 0 && defaultIndex < slots.length && slots[defaultIndex]) {
      slotIndex = defaultIndex;
    }
  }

  return {
    savedState: slotIndex == null ? null : slots[slotIndex],
    slotIndex,
    state: response?.state || {},
  };
}

export function eldraziResponseToPuzzle(response, options = {}) {
  const selected = selectEldraziState(response, options.saveSlot ?? null);
  const sourceState = selected.savedState || selected.state;
  const life = Number(sourceState?.lifeTotal);
  const name = String(selected.state?.seats?.[0] || selected.state?.deckName || "Player 1").trim();

  return {
    version: 1,
    players: [{
      name,
      life: Number.isFinite(life) ? Math.trunc(life) : commanderStartingLife(selected.state),
      zones: selected.savedState
        ? zonesFromSavedState(selected.savedState, options.omitLibrary)
        : zonesFromInitialState(selected.state, options.omitLibrary),
    }],
  };
}

export function buildIronsmithUrl(puzzle, baseUrl = DEFAULT_IRONSMITH_URL) {
  let url;
  try {
    url = new URL(baseUrl);
  } catch {
    throw new Error("The Ironsmith base URL is not valid");
  }

  const encoded = Buffer.from(JSON.stringify(puzzle), "utf8").toString("base64url");
  url.searchParams.set("puzzle", encoded);
  return url.toString();
}

export function conversionWarnings(response, options = {}) {
  const selected = selectEldraziState(response, options.saveSlot ?? null);
  if (!selected.savedState) {
    return ["No saved board slot was selected; only zones explicitly present in the hand state were converted."];
  }

  const warnings = [];
  if (cardNames(selected.savedState?.zones?.stack).length > 0) {
    warnings.push("Ironsmith puzzle URLs cannot represent the stack, so stack cards were omitted.");
  }
  const battlefield = Array.isArray(selected.savedState?.zones?.battlefield)
    ? selected.savedState.zones.battlefield
    : [];
  if (battlefield.some((card) => card?.tapped || card?.isFlipped || card?.isFaceDown)) {
    warnings.push("Tapped, flipped, and face-down status is not encoded by Ironsmith puzzle URLs.");
  }
  const hasCounters = [
    selected.savedState?.cardCounters,
    selected.savedState?.cardTypedCounters,
    selected.savedState?.cardPowerToughness,
    selected.savedState?.tempPTBoosts,
  ].some((value) => value && typeof value === "object" && Object.keys(value).length > 0);
  if (hasCounters) {
    warnings.push("Counters and temporary power/toughness changes are not encoded by Ironsmith puzzle URLs.");
  }
  return warnings;
}

export async function convertEldraziUrl(rawUrl, options = {}) {
  const parsed = parseEldraziUrl(rawUrl);
  const fetchImpl = options.fetchImpl || globalThis.fetch;
  if (typeof fetchImpl !== "function") throw new Error("This Node version does not provide fetch");

  const endpoint = new URL("/api/playtest-from-hand", parsed.origin);
  endpoint.searchParams.set("handId", parsed.handId);
  const response = await fetchImpl(endpoint, {
    headers: { accept: "application/json" },
  });
  if (!response.ok) {
    throw new Error(`Eldrazi returned HTTP ${response.status} while loading hand ${parsed.handId}`);
  }

  const data = await response.json();
  const saveSlot = options.saveSlot ?? parsed.saveSlot;
  const puzzle = eldraziResponseToPuzzle(data, {
    omitLibrary: options.omitLibrary,
    saveSlot,
  });
  return {
    puzzle,
    url: buildIronsmithUrl(puzzle, options.baseUrl || DEFAULT_IRONSMITH_URL),
    warnings: conversionWarnings(data, { saveSlot }),
  };
}

async function main() {
  try {
    const options = parseCliArgs(process.argv.slice(2));
    if (options.help) {
      console.log(usage());
      return;
    }
    if (!options.sourceUrl) throw new Error("Missing Eldrazi URL");

    const result = await convertEldraziUrl(options.sourceUrl, options);
    for (const warning of result.warnings) console.error(`Warning: ${warning}`);
    console.log(result.url);
  } catch (error) {
    console.error(`Error: ${error.message}\n\n${usage()}`);
    process.exitCode = 1;
  }
}

if (import.meta.url === pathToFileURL(process.argv[1] || "").href) {
  await main();
}
