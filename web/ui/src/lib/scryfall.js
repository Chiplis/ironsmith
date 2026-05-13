const ORIGINAL_BASIC_LAND_SETS = new Set([
  "Plains",
  "Island",
  "Swamp",
  "Mountain",
  "Forest",
]);

const CUSTOM_CARD_ART_URLS_STORAGE_KEY = "ironsmith-custom-card-art-urls";
const HIDDEN_CARD_NAMES = new Set(["hidden card"]);
const HIDDEN_CARD_BACK_SVG = `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 488 680" role="img" aria-label="Hidden card">
  <defs>
    <linearGradient id="bg" x1="0" x2="1" y1="0" y2="1">
      <stop offset="0" stop-color="#21150f"/>
      <stop offset="0.42" stop-color="#12151d"/>
      <stop offset="1" stop-color="#08090d"/>
    </linearGradient>
    <radialGradient id="core" cx="50%" cy="42%" r="58%">
      <stop offset="0" stop-color="#8e6be8" stop-opacity="0.85"/>
      <stop offset="0.46" stop-color="#7b4ed6" stop-opacity="0.32"/>
      <stop offset="1" stop-color="#04050a" stop-opacity="0"/>
    </radialGradient>
    <linearGradient id="rim" x1="0" x2="1" y1="0" y2="1">
      <stop offset="0" stop-color="#f2d492"/>
      <stop offset="0.52" stop-color="#7f5bca"/>
      <stop offset="1" stop-color="#e9b15f"/>
    </linearGradient>
  </defs>
  <rect x="8" y="8" width="472" height="664" rx="28" fill="#050507"/>
  <rect x="19" y="19" width="450" height="642" rx="22" fill="url(#bg)" stroke="url(#rim)" stroke-width="5"/>
  <rect x="37" y="37" width="414" height="606" rx="14" fill="none" stroke="#d7b66c" stroke-opacity="0.48" stroke-width="2"/>
  <circle cx="244" cy="290" r="210" fill="url(#core)"/>
  <g fill="none" stroke="#e7d09a" stroke-opacity="0.72" stroke-width="8">
    <path d="M244 128c54 72 112 117 178 136-66 19-124 64-178 136-54-72-112-117-178-136 66-19 124-64 178-136Z"/>
    <path d="M244 186c27 36 56 59 89 69-33 10-62 33-89 69-27-36-56-59-89-69 33-10 62-33 89-69Z"/>
  </g>
  <g fill="#f2dcaa" font-family="Alegreya Sans SC, Georgia, serif" text-anchor="middle">
    <text x="244" y="492" font-size="40" font-weight="800" letter-spacing="4">IRONSMITH</text>
    <text x="244" y="535" font-size="20" font-weight="700" letter-spacing="5" opacity="0.78">HIDDEN CARD</text>
  </g>
</svg>`.trim();

export const HIDDEN_CARD_BACK_IMAGE_URL =
  `data:image/svg+xml;charset=utf-8,${encodeURIComponent(HIDDEN_CARD_BACK_SVG)}`;

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

export function isHiddenCardName(cardName) {
  return HIDDEN_CARD_NAMES.has(customArtKey(cardName));
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
  if (isHiddenCardName(query)) return HIDDEN_CARD_BACK_IMAGE_URL;
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
  if (isHiddenCardName(query)) {
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
