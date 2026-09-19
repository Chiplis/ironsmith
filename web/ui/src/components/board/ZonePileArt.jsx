import { useLayoutEffect, useRef } from "react";
import useScryfallImageUrl from "@/hooks/useScryfallImageUrl";
import { isFaceUpZoneCard } from "@/lib/zone-piles";

// Shared by desktop piles and the touch-sized mobile zone buttons.
export default function ZonePileArt({ card }) {
  const imageRef = useRef(null);
  const name = isFaceUpZoneCard(card) ? card.name : null;
  const url = useScryfallImageUrl(name, "normal");
  useLayoutEffect(() => {
    const source = imageRef.current?.parentElement;
    if (!source) return undefined;
    source.dataset.cardImageUrl = url;
    return () => { delete source.dataset.cardImageUrl; };
  }, [url]);
  return url ? <img ref={imageRef} src={url} alt="" draggable={false} loading="lazy" referrerPolicy="no-referrer" />
    : <span className="zone-pile-placeholder" aria-hidden="true">{card ? "◇" : "—"}</span>;
}
