import { useEffect, useState } from "react";
import { resolveScryfallLocalizedFlavorText } from "@/lib/scryfall";

export default function useScryfallFlavorText(imageUrl, locale = "en") {
  const [resolved, setResolved] = useState(null);
  useEffect(() => {
    let cancelled = false;
    if (!imageUrl) return undefined;
    resolveScryfallLocalizedFlavorText(imageUrl, locale).then((text) => {
      if (!cancelled) setResolved({ imageUrl, locale, text });
    }).catch(() => {
      if (!cancelled) setResolved({ imageUrl, locale, text: "" });
    });
    return () => { cancelled = true; };
  }, [imageUrl, locale]);
  return resolved?.imageUrl === imageUrl && resolved?.locale === locale ? resolved.text : "";
}
