const ORIGINAL_BASIC_LAND_SETS = new Set([
  "Plains",
  "Island",
  "Swamp",
  "Mountain",
  "Forest",
]);

const CUSTOM_CARD_ART_URLS_STORAGE_KEY = "ironsmith-custom-card-art-urls";

function storage() {
  try {
    return globalThis?.localStorage || null;
  } catch {
    return null;
  }
}

function customArtKey(cardName) {
  return String(cardName || "").trim().toLowerCase();
}

function readCustomCardArtUrlMap() {
  const localStorage = storage();
  if (!localStorage) return {};

  try {
    const raw = localStorage.getItem(CUSTOM_CARD_ART_URLS_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed : {};
  } catch {
    return {};
  }
}

function writeCustomCardArtUrlMap(map) {
  const localStorage = storage();
  if (!localStorage) return;

  const entries = Object.entries(map)
    .filter(([, url]) => typeof url === "string" && url.trim())
    .sort(([left], [right]) => left.localeCompare(right));
  if (entries.length === 0) {
    localStorage.removeItem(CUSTOM_CARD_ART_URLS_STORAGE_KEY);
    return;
  }

  localStorage.setItem(CUSTOM_CARD_ART_URLS_STORAGE_KEY, JSON.stringify(Object.fromEntries(entries)));
}

export function customCardArtUrl(cardName) {
  const key = customArtKey(cardName);
  if (!key) return "";
  const url = readCustomCardArtUrlMap()[key];
  return typeof url === "string" ? url.trim() : "";
}

export function setCustomCardArtUrls(entries) {
  const map = readCustomCardArtUrlMap();
  for (const entry of entries || []) {
    const key = customArtKey(entry?.name);
    if (!key) continue;

    const artUrl = String(entry?.artUrl || "").trim();
    if (artUrl) {
      map[key] = artUrl;
    } else {
      delete map[key];
    }
  }
  writeCustomCardArtUrlMap(map);
}

export function scryfallImageUrl(cardName, version = "normal") {
  const query = String(cardName || "").trim();
  if (!query) return "";
  const customUrl = customCardArtUrl(query);
  if (customUrl) return customUrl;

  const params = new URLSearchParams({ format: "image", version });

  if (ORIGINAL_BASIC_LAND_SETS.has(query)) {
    params.set("exact", query);
    params.set("set", "lea");
  } else {
    params.set("fuzzy", query);
  }

  return `https://api.scryfall.com/cards/named?${params.toString()}`;
}
const namedCardMetaCache = new Map();

export async function fetchScryfallCardMeta(cardName) {
  const query = String(cardName || "").trim();
  if (!query) {
    return { mana_cost: null, oracle_text: "", produced_mana: [] };
  }

  if (namedCardMetaCache.has(query)) {
    return namedCardMetaCache.get(query);
  }

  const request = (async () => {
    const params = new URLSearchParams({ exact: query });
    const exactResponse = await fetch(`https://api.scryfall.com/cards/named?${params.toString()}`);
    if (exactResponse.ok) {
      const card = await exactResponse.json();
      return {
        mana_cost: card?.mana_cost || null,
        oracle_text: card?.oracle_text || "",
        produced_mana: Array.isArray(card?.produced_mana) ? card.produced_mana : [],
      };
    }

    const fuzzyParams = new URLSearchParams({ fuzzy: query });
    const fuzzyResponse = await fetch(`https://api.scryfall.com/cards/named?${fuzzyParams.toString()}`);
    if (!fuzzyResponse.ok) {
      throw new Error(`Could not resolve Scryfall metadata for ${query}`);
    }
    const fuzzyCard = await fuzzyResponse.json();
    return {
      mana_cost: fuzzyCard?.mana_cost || null,
      oracle_text: fuzzyCard?.oracle_text || "",
      produced_mana: Array.isArray(fuzzyCard?.produced_mana) ? fuzzyCard.produced_mana : [],
    };
  })()
    .catch((error) => {
      namedCardMetaCache.delete(query);
      throw error;
    });

  namedCardMetaCache.set(query, request);
  return request;
}
