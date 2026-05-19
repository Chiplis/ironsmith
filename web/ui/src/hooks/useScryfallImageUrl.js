import { useEffect, useMemo, useState } from "react";
import { resolveScryfallImageUrl, scryfallImageUrl } from "@/lib/scryfall";

export default function useScryfallImageUrl(cardName, version = "normal") {
  const query = String(cardName || "").trim();
  const imageVersion = String(version || "normal").trim() || "normal";
  const key = useMemo(() => `${query}|${imageVersion}`, [imageVersion, query]);
  const cached = scryfallImageUrl(query, imageVersion);
  const [resolved, setResolved] = useState(() => ({
    key,
    url: cached,
  }));
  const currentUrl = resolved.key === key ? resolved.url : cached;

  useEffect(() => {
    let cancelled = false;
    if (cached || !query) return undefined;

    resolveScryfallImageUrl(query, imageVersion)
      .then((url) => {
        if (!cancelled) {
          setResolved({ key, url: url || "" });
        }
      })
      .catch(() => {
        if (!cancelled) {
          setResolved({ key, url: "" });
        }
      });

    return () => {
      cancelled = true;
    };
  }, [cached, imageVersion, key, query]);

  return currentUrl;
}
