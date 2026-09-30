import { useLayoutEffect, useRef } from "react";
import useScryfallImageUrl from "@/hooks/useScryfallImageUrl";
import { isFaceUpZoneCard } from "@/lib/zone-piles";
import { customCardCounterOverrides } from "@/lib/scryfall";
import {
  aggregateCounterEntries,
  BattlefieldCounterBadge,
  buildCounterBadge,
  resolveCardCounters,
} from "@/components/cards/counter-badges";

function ZonePilePlaceholder({ hasCard }) {
  if (!hasCard) return <span aria-hidden="true">—</span>;
  return (
    <svg
      viewBox="0 0 24 24"
      width="22"
      height="22"
      aria-hidden="true"
      focusable="false"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <rect x="5" y="4" width="12" height="16" rx="1.5" />
      <path d="M8 7h6M8 10h6M8 13h4" />
      <path d="M17 7.5 20 9v10a1 1 0 0 1-1 1h-2" />
    </svg>
  );
}

// Shared by desktop piles and the touch-sized mobile zone buttons.
export default function ZonePileArt({ card }) {
  const imageRef = useRef(null);
  const faceUp = isFaceUpZoneCard(card);
  const name = faceUp ? card.name : null;
  const url = useScryfallImageUrl(name, "normal");
  const counterOverride = faceUp ? customCardCounterOverrides(name) : null;
  const counterBadges = faceUp
    ? aggregateCounterEntries(
      counterOverride
        || resolveCardCounters(card?.counters, card?.counter_signature ?? card?.counterSignature)
    )
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
        : <span className="zone-pile-placeholder"><ZonePilePlaceholder hasCard={Boolean(card)} /></span>}
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
