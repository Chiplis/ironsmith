import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import { messages } from "./messages";

const I18N_STORAGE_KEY = "ironsmith.locale";
const DEFAULT_LOCALE = "en";

export const LOCALES = [
  { id: "en", label: "English" },
  { id: "es", label: "Espanol" },
];

const I18nContext = createContext(null);

function readStoredLocale() {
  if (typeof window === "undefined") return DEFAULT_LOCALE;
  const stored = String(window.localStorage.getItem(I18N_STORAGE_KEY) || "").trim();
  return messages[stored] ? stored : DEFAULT_LOCALE;
}

function interpolate(template, params) {
  if (!params || typeof params !== "object") return template;
  return String(template).replace(/\{([a-zA-Z0-9_]+)\}/g, (match, key) => (
    params[key] == null ? match : String(params[key])
  ));
}

export function I18nProvider({ children }) {
  const [locale, setLocaleRaw] = useState(readStoredLocale);

  useEffect(() => {
    if (typeof window === "undefined") return;
    window.localStorage.setItem(I18N_STORAGE_KEY, locale);
    document.documentElement.lang = locale;
  }, [locale]);

  const setLocale = useCallback((nextLocale) => {
    const normalized = String(nextLocale || "").trim();
    setLocaleRaw(messages[normalized] ? normalized : DEFAULT_LOCALE);
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

export function useI18n() {
  const value = useContext(I18nContext);
  if (!value) {
    throw new Error("useI18n must be used inside I18nProvider");
  }
  return value;
}
