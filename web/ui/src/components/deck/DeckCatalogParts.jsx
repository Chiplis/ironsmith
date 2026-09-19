import { memo } from "react";
import useUiText from "@/i18n/useUiText";
import { deckArtName, useCardArt } from "@/lib/deck-catalog-view";
import { ManaSymbol } from "@/lib/mana-symbols";

// Not every card in the local corpus has art, so the tile is a card back
// rather than an empty hole.
export const DeckArt = memo(function DeckArt({ entry, className }) {
  const artUrl = useCardArt(deckArtName(entry));
  return (
    <div className={`shrink-0 overflow-hidden bg-[#131418] ${className}`} aria-hidden="true" data-deck-art={artUrl ? "loaded" : "placeholder"}>
      {artUrl
        ? <img className="h-full w-full object-cover" src={artUrl} alt="" loading="lazy" onError={(event) => { event.currentTarget.style.display = "none"; }} />
        : (
          <svg viewBox="0 0 24 24" className="h-full w-full text-[#3a3226]" fill="none" stroke="currentColor" strokeWidth="1.2">
            <rect x="7" y="4.5" width="10" height="15" rx="1.5" />
            <path d="M9.5 8.5h5M9.5 12h5M9.5 15.5h3" strokeLinecap="round" />
          </svg>
        )}
    </div>
  );
});

export const ManaPips = memo(function ManaPips({ colors, size = 13 }) {
  const ui = useUiText();
  if (!colors.length) return null;
  return (
    <span className="inline-flex shrink-0 items-center gap-0.5" aria-label={ui("Colors: {0}", { 0: colors.join(", ") })}>
      {colors.map((color) => <ManaSymbol key={color} sym={color} size={size} />)}
    </span>
  );
});
