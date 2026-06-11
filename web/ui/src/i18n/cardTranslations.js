import { cardRouteKey, fetchScryfallLocalizedCardTranslation } from "@/lib/scryfall";
import { loadGeneratedTextTranslation } from "./generatedTextTranslations";

const cardI18nBucketCache = new Map();
const officialCardTranslationCache = new Map();
const translatedCardViewCache = new Map();

function baseAssetUrl() {
  const configured = typeof import.meta !== "undefined"
    ? import.meta.env?.BASE_URL
    : null;
  const base = configured || "/";
  return new URL(base, globalThis?.location?.href || "http://localhost/").href;
}

// Prebuilt translations are sharded into bucket files keyed by the first two
// characters of the lookup key, so the asset count stays in the hundreds
// instead of one file per card.
export function cardI18nBucketKey(key) {
  return String(key || "").slice(0, 2) || "_";
}

function cardI18nBucketUrl(locale, kind, key) {
  return new URL(
    `card-i18n/${encodeURIComponent(locale)}/${kind}/${encodeURIComponent(cardI18nBucketKey(key))}.json`,
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

async function lookupCardI18nBucket(locale, kind, key) {
  if (!key) return null;
  const bucketCacheKey = `${locale}:${kind}:${cardI18nBucketKey(key)}`;
  if (!cardI18nBucketCache.has(bucketCacheKey)) {
    cardI18nBucketCache.set(
      bucketCacheKey,
      fetchJsonOrNull(cardI18nBucketUrl(locale, kind, key)).catch(() => null)
    );
  }
  const bucket = await cardI18nBucketCache.get(bucketCacheKey);
  const entry = bucket?.[key];
  return entry && typeof entry === "object" ? entry : null;
}

export async function loadOfficialCardTranslation(locale, cardName, oracleId = null) {
  if (!locale || locale === "en") return null;

  const route = cardRouteKey(cardName);
  const oracleKey = oracleId ? String(oracleId).trim() : "";
  if (!route && !oracleKey) return null;

  const cacheKey = `${locale}:${oracleKey || "-"}:${route || "-"}`;
  if (!officialCardTranslationCache.has(cacheKey)) {
    officialCardTranslationCache.set(cacheKey, (async () => {
      const byOracle = await lookupCardI18nBucket(locale, "by-oracle", oracleKey);
      if (byOracle) return byOracle;
      const byName = await lookupCardI18nBucket(locale, "by-name", route);
      if (byName) return byName;
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
      // Card names and type lines are only ever taken from official Scryfall
      // printed fields; everything else stays English. Machine translation is
      // reserved for rules text.
      const official = await loadOfficialCardTranslation(locale, cardName, oracleId);
      const officialRulesText = String(official?.oracleText || "").trim();
      const generatedRulesText = !officialRulesText && rulesText
        ? await loadGeneratedTextTranslation(locale, rulesText)
        : null;

      if (!official && !generatedRulesText) return null;
      return {
        name: official?.name || cardName || null,
        typeLine: official?.typeLine || typeLine || null,
        rulesText: officialRulesText || generatedRulesText || rulesText || null,
        source: official ? "scryfall" : "generated",
      };
    })());
  }

  return translatedCardViewCache.get(cacheKey);
}
