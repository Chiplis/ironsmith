import { useLayoutEffect, useRef } from "react";
import useScryfallImageUrl from "@/hooks/useScryfallImageUrl";
import { isFaceUpZoneCard } from "@/lib/zone-piles";
import {
  BattlefieldCounterBadge,
  buildCounterBadge,
  resolveCardCounters,
} from "@/components/cards/counter-badges";

// Shared by desktop piles and the touch-sized mobile zone buttons.
export default function ZonePileArt({ card }) {
  const imageRef = useRef(null);
  const faceUp = isFaceUpZoneCard(card);
  const name = faceUp ? card.name : null;
  const url = useScryfallImageUrl(name, "normal");
  const counterBadges = faceUp
    ? resolveCardCounters(card?.counters, card?.counter_signature ?? card?.counterSignature)
      .map(buildCounterBadge)
      .filter(Boolean)
    : [];
  useLayoutEffect(() => {
    const source = imageRef.current?.parentElement;
    if (!source) return undefined;
    source.dataset.cardImageUrl = url;
    return () => { delete source.dataset.cardImageUrl; };
  }, [url]);
  return (
    <>
      {url ? <img ref={imageRef} src={url} alt="" draggable={false} loading="lazy" referrerPolicy="no-referrer" />
        : <span className="zone-pile-placeholder" aria-hidden="true">{card ? "◇" : "—"}</span>}
      {counterBadges.length > 0 && (
        <span className="zone-pile-counter-rail" aria-label="Card counters">
          {counterBadges.map((badge, index) => (
            <BattlefieldCounterBadge key={`${badge.fullLabel}-${index}`} badge={badge} />
          ))}
        </span>
      )}
    </>
  );
}
