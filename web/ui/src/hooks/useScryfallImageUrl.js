import { useEffect, useMemo, useState } from "react";
import { useI18n } from "@/i18n/I18nContext";
import {
  resolveScryfallImageUrl,
  resolveScryfallLocalizedImageUrl,
  scryfallImageUrl,
} from "@/lib/scryfall";

export function useScryfallImage(cardName, version = "normal") {
  const { locale } = useI18n();
  const query = String(cardName || "").trim();
  const imageVersion = String(version || "normal").trim() || "normal";
  const key = useMemo(() => `${locale}|${query}|${imageVersion}`, [imageVersion, locale, query]);
  const cached = scryfallImageUrl(query, imageVersion);
  const [localized, setLocalized] = useState(() => ({ key, url: "", settled: locale === "en" || !query }));
  const [resolved, setResolved] = useState(() => ({
    key,
    url: cached,
    settled: Boolean(cached) || !query,
  }));
  const localizedSettled = localized.key === key && localized.settled;
  const useEnglishFallback = locale === "en" || (localizedSettled && !localized.url);
  const localizedUrl = localized.key === key ? localized.url : "";
  const currentUrl = localizedUrl || (useEnglishFallback
    ? ((resolved.key === key && resolved.url) ? resolved.url : cached)
    : "");

  useEffect(() => {
    let cancelled = false;
    if (locale === "en" || !query) {
      return undefined;
    }

    resolveScryfallLocalizedImageUrl(query, locale, imageVersion)
      .then((url) => {
        if (!cancelled) setLocalized({ key, url: url || "", settled: true });
      })
      .catch(() => {
        if (!cancelled) setLocalized({ key, url: "", settled: true });
      });

    return () => { cancelled = true; };
  }, [imageVersion, key, locale, query]);

  useEffect(() => {
    let cancelled = false;
    if (!useEnglishFallback || cached || !query) return undefined;

    resolveScryfallImageUrl(query, imageVersion)
      .then((url) => {
        if (!cancelled) {
          setResolved({ key, url: url || "", settled: true });
        }
      })
      .catch(() => {
        if (!cancelled) {
          setResolved({ key, url: "", settled: true });
        }
      });

    return () => {
      cancelled = true;
    };
  }, [cached, imageVersion, key, query, useEnglishFallback]);

  return {
    url: currentUrl,
    ready: Boolean(currentUrl) || !query
      || (useEnglishFallback && resolved.key === key && resolved.settled),
  };
}

export default function useScryfallImageUrl(cardName, version = "normal") {
  return useScryfallImage(cardName, version).url;
}
