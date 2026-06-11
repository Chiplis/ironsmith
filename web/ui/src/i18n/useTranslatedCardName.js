import { useEffect, useState } from "react";
import { useI18n } from "./I18nContext";
import { loadOfficialCardTranslation } from "./cardTranslations";

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
