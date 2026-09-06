import { useEffect, useState } from "react";
import { resolveScryfallFlavorText } from "@/lib/scryfall";

export default function useScryfallFlavorText(imageUrl) {
  const [resolved, setResolved] = useState(null);
  useEffect(() => {
    let cancelled = false;
    if (!imageUrl) return undefined;
    resolveScryfallFlavorText(imageUrl).then((text) => {
      if (!cancelled) setResolved({ imageUrl, text });
    }).catch(() => {
      if (!cancelled) setResolved({ imageUrl, text: "" });
    });
    return () => { cancelled = true; };
  }, [imageUrl]);
  return resolved?.imageUrl === imageUrl ? resolved.text : "";
}
