import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import { messages } from "./messages";

import { DEFAULT_LOCALE, I18N_STORAGE_KEY, LOCALES, getActiveLocale, setActiveLocale, interpolate } from "./catalog.js";
export { LOCALES } from "./catalog.js";

const I18nContext = createContext(null);

function readStoredLocale() {
  try {
    const stored = globalThis.localStorage?.getItem(I18N_STORAGE_KEY);
    if (stored) setActiveLocale(stored);
  } catch { /* Storage is optional. */ }
  return getActiveLocale();
}

export function I18nProvider({ children }) {
  const [locale, setLocaleRaw] = useState(readStoredLocale);

  useEffect(() => {
    if (typeof window === "undefined") return;
    try { window.localStorage.setItem(I18N_STORAGE_KEY, locale); } catch { /* Optional persistence. */ }
    document.documentElement.lang = locale;
  }, [locale]);

  const setLocale = useCallback((nextLocale) => {
    const normalized = String(nextLocale || "").trim();
    const next = messages[normalized] ? normalized : DEFAULT_LOCALE;
    setActiveLocale(next);
    setLocaleRaw(next);
  }, []);

  const t = useCallback((key, params = null, fallback = null) => {
    const localized = messages[locale]?.[key];
    const english = messages[DEFAULT_LOCALE]?.[key];
    const template = localized ?? english ?? fallback ?? key;
    return interpolate(template, params);
  }, [locale]);

  const value = useMemo(() => ({
    locale,
    locales: LOCALES,
    setLocale,
    t,
  }), [locale, setLocale, t]);

  return (
    <I18nContext.Provider value={value}>
      {children}
    </I18nContext.Provider>
  );
}

export function useI18n({ optional = false } = {}) {
  const value = useContext(I18nContext);
  if (!value && optional) return { locale: getActiveLocale() };
  if (!value) {
    throw new Error("useI18n must be used inside I18nProvider");
  }
  return value;
}
