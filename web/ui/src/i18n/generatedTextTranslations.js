const generatedTextCache = new Map();

const encoder = new TextEncoder();

export function normalizeGeneratedEnglishText(text) {
  return String(text || "")
    .replace(/\r\n?/g, "\n")
    .split("\n")
    .map((line) => line.trim().replace(/\s+/g, " "))
    .join("\n")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}

function hexFromBytes(buffer) {
  return [...new Uint8Array(buffer)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

export async function generatedTextHash(sourceText) {
  const normalized = normalizeGeneratedEnglishText(sourceText);
  if (!normalized) return "";
  const digest = await crypto.subtle.digest("SHA-256", encoder.encode(normalized));
  return hexFromBytes(digest);
}

export function generatedTextTranslationUrl(locale, sourceHash) {
  return `/generated-i18n/${encodeURIComponent(locale)}/${encodeURIComponent(sourceHash)}.json`;
}

export async function loadGeneratedTextTranslation(locale, sourceText) {
  if (!locale || locale === "en") return null;
  const sourceHash = await generatedTextHash(sourceText);
  if (!sourceHash) return null;

  const cacheKey = `${locale}:${sourceHash}`;
  if (!generatedTextCache.has(cacheKey)) {
    generatedTextCache.set(cacheKey, (async () => {
      const response = await fetch(generatedTextTranslationUrl(locale, sourceHash), {
        cache: "force-cache",
      });
      if (!response.ok) return null;
      const payload = await response.json();
      if (payload?.sourceHash !== sourceHash || payload?.targetLang !== locale) return null;
      return payload?.translatedText || null;
    })().catch(() => null));
  }

  return generatedTextCache.get(cacheKey);
}
