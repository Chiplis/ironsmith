import { cardRouteKey, fetchScryfallLocalizedCardTranslation } from "@/lib/scryfall";
import { loadGeneratedTextTranslation } from "./generatedTextTranslations";

const officialCardTranslationCache = new Map();
const translatedCardViewCache = new Map();

const ES_TYPE_LINE_REPLACEMENTS = [
  ["Legendary", "Legendario"],
  ["Basic", "Basico"],
  ["Snow", "Nevado"],
  ["Artifact", "Artefacto"],
  ["Creature", "Criatura"],
  ["Enchantment", "Encantamiento"],
  ["Instant", "Instantaneo"],
  ["Sorcery", "Conjuro"],
  ["Land", "Tierra"],
  ["Battle", "Batalla"],
  ["Planeswalker", "Planeswalker"],
  ["Kindred", "Kindred"],
  ["Plains", "Llanura"],
  ["Island", "Isla"],
  ["Swamp", "Pantano"],
  ["Mountain", "Montana"],
  ["Forest", "Bosque"],
  ["Human", "Humano"],
  ["Bear", "Oso"],
  ["Elf", "Elfo"],
  ["Druid", "Druida"],
];

function baseAssetUrl() {
  const configured = typeof import.meta !== "undefined"
    ? import.meta.env?.BASE_URL
    : null;
  const base = configured || "/";
  return new URL(base, globalThis?.location?.href || "http://localhost/").href;
}

function cardI18nUrl(locale, kind, key) {
  return new URL(
    `card-i18n/${encodeURIComponent(locale)}/${kind}/${encodeURIComponent(key)}.json`,
    baseAssetUrl()
  ).href;
}

async function fetchJsonOrNull(url) {
  const response = await fetch(url, { cache: "force-cache" });
  if (response.status === 404) return null;
  if (!response.ok) throw new Error(`Card translation fetch failed: HTTP ${response.status}`);
  const payload = await response.json();
  return payload && typeof payload === "object" ? payload : null;
}

function translateTypeLineFallback(locale, typeLine) {
  if (locale !== "es") return typeLine || "";
  let translated = String(typeLine || "");
  for (const [english, spanish] of ES_TYPE_LINE_REPLACEMENTS) {
    translated = translated.replace(new RegExp(`\\b${english}\\b`, "g"), spanish);
  }
  return translated;
}

export async function loadOfficialCardTranslation(locale, cardName, oracleId = null) {
  if (!locale || locale === "en") return null;

  const route = cardRouteKey(cardName);
  const oracleKey = oracleId ? String(oracleId).trim() : "";
  if (!route && !oracleKey) return null;

  const cacheKey = `${locale}:${oracleKey || "-"}:${route || "-"}`;
  if (!officialCardTranslationCache.has(cacheKey)) {
    officialCardTranslationCache.set(cacheKey, (async () => {
      if (oracleKey) {
        const byOracle = await fetchJsonOrNull(cardI18nUrl(locale, "by-oracle", oracleKey)).catch(() => null);
        if (byOracle) return byOracle;
      }
      if (route) {
        const byName = await fetchJsonOrNull(cardI18nUrl(locale, "by-name", route)).catch(() => null);
        if (byName) return byName;
      }
      return fetchScryfallLocalizedCardTranslation(cardName, locale).catch(() => null);
    })());
  }

  return officialCardTranslationCache.get(cacheKey);
}

export async function loadTranslatedCardView(locale, cardView) {
  if (!locale || locale === "en" || !cardView) return null;

  const cardName = String(cardView.name || "").trim();
  const typeLine = String(cardView.typeLine || "").trim();
  const rulesText = String(cardView.rulesText || "").trim();
  const oracleId = String(cardView.oracleId || "").trim();

  const cacheKey = [
    locale,
    oracleId || "-",
    cardRouteKey(cardName) || "-",
    typeLine,
    rulesText,
  ].join("|");

  if (!translatedCardViewCache.has(cacheKey)) {
    translatedCardViewCache.set(cacheKey, (async () => {
      const official = await loadOfficialCardTranslation(locale, cardName, oracleId);
      if (official) {
        const officialTypeLine = official.typeLine || official.printedTypeLine || "";
        const officialRulesText = String(official.oracleText || official.rulesText || "").trim();
        // Official payloads never carry English rules text; when no localized
        // printing was digitized, fall back to the generated translation.
        const generatedRulesText = !officialRulesText && rulesText
          ? await loadGeneratedTextTranslation(locale, rulesText)
          : null;
        return {
          name: official.name || cardName || null,
          typeLine: translateTypeLineFallback(
            locale,
            officialTypeLine && officialTypeLine !== typeLine ? officialTypeLine : typeLine
          ) || null,
          rulesText: officialRulesText || generatedRulesText || rulesText || null,
          source: "scryfall",
        };
      }

      const [translatedName, translatedTypeLine, translatedRulesText] = await Promise.all([
        cardName ? loadGeneratedTextTranslation(locale, cardName) : null,
        typeLine ? loadGeneratedTextTranslation(locale, typeLine) : null,
        rulesText ? loadGeneratedTextTranslation(locale, rulesText) : null,
      ]);

      if (!translatedName && !translatedTypeLine && !translatedRulesText) return null;
      return {
        name: translatedName || cardName || null,
        typeLine: translatedTypeLine || typeLine || null,
        rulesText: translatedRulesText || rulesText || null,
        source: "generated",
      };
    })());
  }

  return translatedCardViewCache.get(cacheKey);
}
