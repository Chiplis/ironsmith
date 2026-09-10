import { PUZZLE_VERSION } from "./puzzles.js";

/**
 * Zones a generated table can fill, in the order the puzzle payload lists them.
 * The ante zone is deliberately absent: the engine's zone lookup has no name
 * for it, and one ante card rejects the whole batch of card placements.
 */
export const RANDOM_GAME_ZONES = ["battlefield", "hand", "library", "graveyard", "exile", "command"];

// Engine card types, as `definition.card.card_types` spells them.
export const PERMANENT_TYPES = ["Artifact", "Battle", "Creature", "Enchantment", "Land", "Planeswalker"];
export const SPELL_TYPES = ["Instant", "Sorcery"];
// Kindred (once Tribal) only ever rides along with another type.
const RIDER_TYPES = ["Kindred", "Tribal"];
const NORMAL_TYPES = new Set([...PERMANENT_TYPES, ...SPELL_TYPES, ...RIDER_TYPES]);

export const CARD_COLORS = ["White", "Blue", "Black", "Red", "Green"];
export const BASIC_LAND_BY_COLOR = {
  White: "Plains",
  Blue: "Island",
  Black: "Swamp",
  Red: "Mountain",
  Green: "Forest",
  Colorless: "Wastes",
};

/** Only a permanent can sit on the battlefield, and only these can be a commander. */
const ZONE_RULES = {
  battlefield: (card) => card.permanent,
  command: (card) => card.permanent && card.legendary
    && (card.types.includes("Creature") || card.types.includes("Planeswalker")),
};

function hashSeed(seed) {
  const text = String(seed ?? "");
  let hash = 0x811c9dc5;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= text.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash || 0x9e3779b9;
}

/**
 * A small seeded generator, so a table can be described by its seed alone and
 * the same seed rebuilds it card for card.
 */
export function createSeededRng(seed) {
  let state = hashSeed(seed);
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let next = state;
    next = Math.imul(next ^ (next >>> 15), next | 1);
    next ^= next + Math.imul(next ^ (next >>> 7), next | 61);
    return ((next ^ (next >>> 14)) >>> 0) / 4294967296;
  };
}

function pipManaValue(pip) {
  if (Array.isArray(pip)) return pip.some((entry) => typeof entry === "object" && entry !== null && "Generic" in entry)
    ? Math.max(...pip.map((entry) => (typeof entry === "object" && entry !== null && "Generic" in entry ? Number(entry.Generic) || 0 : 1)))
    : 1;
  if (typeof pip === "object" && pip !== null && "Generic" in pip) return Number(pip.Generic) || 0;
  return 1;
}

function pipColors(pip) {
  const entries = Array.isArray(pip) ? pip : [pip];
  return entries.filter((entry) => typeof entry === "string" && CARD_COLORS.includes(entry));
}

/**
 * Describe a card from its compiled frontend asset. The engine's own card types
 * decide where a card may legally sit, so they are read rather than the printed
 * type line. Anything outside a normal game — tokens, planes, schemes — has no
 * place in a generated table and is dropped here.
 */
export function classifyCard(source) {
  const name = String(source?.canonicalName || source?.group?.name || "").trim();
  const artifact = Array.isArray(source?.artifacts) ? source.artifacts[0] : null;
  const card = artifact?.payload?.definition?.card;
  if (!name || !card || card.is_token) return null;
  const types = Array.isArray(card.card_types) ? card.card_types.map((type) => String(type)) : [];
  if (types.length === 0 || types.some((type) => !NORMAL_TYPES.has(type))) return null;
  const supertypes = Array.isArray(card.supertypes) ? card.supertypes.map((type) => String(type)) : [];
  const pips = Array.isArray(card.mana_cost?.pips) ? card.mana_cost.pips : [];
  const colors = [...new Set(pips.flatMap(pipColors))];
  const score = Number(source?.group?.score);
  return {
    name,
    types,
    supertypes,
    colors,
    manaValue: pips.reduce((total, pip) => total + pipManaValue(pip), 0),
    permanent: types.some((type) => PERMANENT_TYPES.includes(type))
      && !types.some((type) => SPELL_TYPES.includes(type)),
    legendary: supertypes.includes("Legendary"),
    basic: supertypes.includes("Basic"),
    singleFaced: String(card.linked_face_layout || "None") === "None",
    score: Number.isFinite(score) ? score : null,
  };
}

/** Whether a zone can hold this card at all, before any of the player's own filters. */
export function zoneAcceptsCard(zone, card) {
  if (!card || !RANDOM_GAME_ZONES.includes(zone)) return false;
  const rule = ZONE_RULES[zone];
  return rule ? rule(card) : true;
}

/** Whether the card is one the player asked for. */
export function cardMatchesFilters(card, config) {
  if (!card) return false;
  if (config.singleFacedOnly && !card.singleFaced) return false;
  const minScore = Number(config.minScore);
  if (Number.isFinite(minScore) && minScore > 0 && !(Number(card.score) >= minScore)) return false;
  if (!card.types.some((type) => config.types?.[type])) return false;
  const wanted = card.colors.length === 0
    ? Boolean(config.colors?.Colorless)
    : card.colors.some((color) => config.colors?.[color]);
  if (!wanted) return false;
  const { min, max } = config.manaValue || {};
  if (Number.isFinite(Number(min)) && card.manaValue < Number(min)) return false;
  if (Number.isFinite(Number(max)) && card.manaValue > Number(max)) return false;
  return true;
}

export function randomGameDefaults() {
  return {
    seed: "",
    playerCount: 2,
    startingLife: 20,
    zones: {
      battlefield: { count: 6, basics: 3 },
      hand: { count: 7, basics: 1 },
      library: { count: 30, basics: 12 },
      graveyard: { count: 3, basics: 0 },
      exile: { count: 0, basics: 0 },
      command: { count: 0, basics: 0 },
    },
    types: {
      Creature: true,
      Artifact: true,
      Enchantment: true,
      Planeswalker: true,
      Battle: false,
      Land: true,
      Instant: true,
      Sorcery: true,
    },
    colors: { White: true, Blue: true, Black: true, Red: true, Green: true, Colorless: true },
    manaValue: { min: 0, max: 7 },
    minScore: 1,
    // Cards the local player's battlefield always starts with, whatever the
    // filters say. Omniscience makes a generated table immediately playable.
    alwaysOnMyBattlefield: ["Omniscience"],
    singleFacedOnly: true,
    allowDuplicates: false,
    allowDuplicateLegends: false,
  };
}

function normalizeZoneCount(raw) {
  const parsed = Math.trunc(Number(raw));
  return Number.isFinite(parsed) && parsed > 0 ? Math.min(parsed, 250) : 0;
}

/** The basics the player left switched on, so a generated table can still make mana. */
export function basicLandCycle(config) {
  const names = CARD_COLORS
    .filter((color) => config.colors?.[color])
    .map((color) => BASIC_LAND_BY_COLOR[color]);
  if (names.length > 0) return names;
  return config.colors?.Colorless ? [BASIC_LAND_BY_COLOR.Colorless] : [];
}

const nameKey = (name) => String(name || "").trim().toLocaleLowerCase("en-US");

/**
 * The named cards the local battlefield must start with, and the names that
 * cannot be honoured. A name is refused when the catalogue has no such card or
 * when it could not legally sit on a battlefield — the promise is a playable
 * table, so a spell named here is reported rather than placed.
 */
export function resolveGuaranteedBattlefield(names, cards = []) {
  const byName = new Map(cards.filter(Boolean).map((card) => [nameKey(card.name), card]));
  const accepted = [];
  const rejected = [];
  for (const name of Array.isArray(names) ? names : []) {
    const trimmed = String(name || "").trim();
    if (!trimmed) continue;
    const card = byName.get(nameKey(trimmed));
    if (card && zoneAcceptsCard("battlefield", card)) accepted.push(card);
    else rejected.push(trimmed);
  }
  return { accepted, rejected };
}

function drawCard(pool, taken, config, zone, used) {
  for (let attempt = 0; attempt < pool.length; attempt += 1) {
    const card = pool[(taken.cursor + attempt) % pool.length];
    const seen = used.get(card.name) || 0;
    if (!config.allowDuplicates && seen > 0) continue;
    // Two of one legend on the same battlefield would kill one another as the
    // table settles, which is a state no one asked to be generated.
    if (!config.allowDuplicateLegends && card.legendary && zone === "battlefield" && seen > 0) continue;
    taken.cursor = (taken.cursor + attempt + 1) % pool.length;
    used.set(card.name, seen + 1);
    return card.name;
  }
  return null;
}

function shuffled(items, rng) {
  const copy = [...items];
  for (let index = copy.length - 1; index > 0; index -= 1) {
    const swap = Math.floor(rng() * (index + 1));
    [copy[index], copy[swap]] = [copy[swap], copy[index]];
  }
  return copy;
}

/**
 * Build a puzzle payload describing a random table.
 *
 * `cards` are classified descriptors; every zone draws only from the cards it
 * can legally hold, so an instant can never be generated onto the battlefield.
 * Basic lands are drawn from the chosen colours rather than the catalogue, so a
 * table can always produce mana even when the filters are narrow.
 */
export function generateRandomGamePayload({ config, cards = [], guaranteedCards = [], rng = Math.random } = {}) {
  const settings = { ...randomGameDefaults(), ...(config || {}) };
  const eligible = cards.filter((card) => cardMatchesFilters(card, settings));
  const basics = basicLandCycle(settings);
  // The local player is always the first: a loaded table is viewed from
  // perspective 0.
  const guaranteed = resolveGuaranteedBattlefield(settings.alwaysOnMyBattlefield, [...guaranteedCards, ...cards]);
  const playerCount = Math.max(1, Math.min(8, Math.trunc(Number(settings.playerCount)) || 1));
  const life = Math.trunc(Number(settings.startingLife));
  const shortfalls = new Set();

  const players = Array.from({ length: playerCount }, (_, playerIndex) => {
    const used = new Map();
    const zones = {};
    for (const zone of RANDOM_GAME_ZONES) {
      // Cards promised to the local battlefield are placed before anything
      // else and take slots from the random draw, never from each other.
      const promised = zone === "battlefield" && playerIndex === 0
        ? guaranteed.accepted.map((card) => card.name)
        : [];
      const requested = Math.max(promised.length, normalizeZoneCount(settings.zones?.[zone]?.count));
      if (requested === 0) {
        zones[zone] = [];
        continue;
      }
      const wantedBasics = zone === "command"
        ? 0
        : Math.min(Math.max(0, requested - promised.length), normalizeZoneCount(settings.zones?.[zone]?.basics));
      const names = [...promised];
      for (const name of promised) used.set(name, (used.get(name) || 0) + 1);
      for (let index = 0; index < wantedBasics; index += 1) {
        if (basics.length === 0) break;
        names.push(basics[index % basics.length]);
      }
      const pool = shuffled(eligible.filter((card) => zoneAcceptsCard(zone, card)), rng);
      const taken = { cursor: 0 };
      while (names.length < requested) {
        const drawn = pool.length > 0 ? drawCard(pool, taken, settings, zone, used) : null;
        if (!drawn) break;
        names.push(drawn);
      }
      if (names.length < requested) shortfalls.add(zone);
      zones[zone] = shuffled(names, rng);
    }
    return {
      name: `Player ${playerIndex + 1}`,
      life: Number.isFinite(life) ? life : 20,
      zones,
    };
  });

  return {
    payload: { version: PUZZLE_VERSION, players },
    shortfalls: [...shortfalls],
    eligibleCount: eligible.length,
    unavailableGuaranteed: guaranteed.rejected,
  };
}
