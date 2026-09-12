import { useCallback, useEffect, useMemo, useState } from "react";
import { useI18n } from "./I18nContext";
import { loadOfficialCardTranslation } from "./cardTranslations";
import { translationForFace } from "./cardTranslationFace";

// Returns the official Scryfall printed name for the active locale, or the
// English name while loading / when no localized printing exists. Names are
// never machine-translated.
export function useTranslatedCardName(cardName, oracleId = null) {
  const { locale } = useI18n();
  const name = String(cardName || "").trim();
  const translationKey = `${locale}|${name}|${oracleId || ""}`;
  const [translated, setTranslated] = useState(null);

  useEffect(() => {
    if (locale === "en" || !name) return undefined;

    let cancelled = false;
    loadOfficialCardTranslation(locale, name, oracleId).then((official) => {
      if (cancelled || !official?.name) return;
      setTranslated({ key: translationKey, name: official.name });
    });

    return () => {
      cancelled = true;
    };
  }, [locale, name, oracleId, translationKey]);

  return (translated?.key === translationKey && translated.name) || name;
}

const NO_TRANSLATIONS = new Map();

// The batch form, for lists that cannot call a hook per row (decision option
// pills, candidate rows). Returns a lookup that falls back to the English name
// while loading and for cards with no localized printing.
export function useTranslatedCardNames(names) {
  const { locale } = useI18n();
  const key = (names || [])
    .map((name) => String(name || "").trim())
    .filter(Boolean)
    .sort()
    .join("|");
  const unique = useMemo(() => (key ? [...new Set(key.split("|"))] : []), [key]);
  const cacheKey = `${locale}|${key}`;
  const [loaded, setLoaded] = useState(null);

  useEffect(() => {
    if (locale === "en" || unique.length === 0) return undefined;

    let cancelled = false;
    Promise.all(
      unique.map(async (name) => {
        const official = translationForFace(await loadOfficialCardTranslation(locale, name), name);
        return [name, String(official?.name || "").trim()];
      })
    ).then((pairs) => {
      if (cancelled) return;
      setLoaded({ key: cacheKey, map: new Map(pairs.filter(([, translated]) => translated)) });
    });

    return () => {
      cancelled = true;
    };
  }, [cacheKey, locale, unique]);

  const translations = loaded?.key === cacheKey ? loaded.map : NO_TRANSLATIONS;
  return useCallback(
    (name) => translations.get(String(name || "").trim()) || name,
    [translations]
  );
}
