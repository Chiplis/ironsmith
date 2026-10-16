import ZonePileLabel from "./ZonePileLabel";
import useUiText from "@/i18n/useUiText";
import { HIDDEN_CARD_BACK_IMAGE_URL } from "@/lib/scryfall";
import { useGame } from "@/context/GameContext";
import { useHoverActions } from "@/context/HoverContext";
import { visibleLibraryCards } from "@/lib/library-pile";
import ZonePileArt from "./ZonePileArt";

export default function LibraryZonePile({ player, onCardClick, renderExpandedPile }) {
  const ui = useUiText();
  const { state } = useGame();
  const { hoverCard, clearHover, showAnchoredCardPreview } = useHoverActions();
  const count = Math.max(0, Number(player.library_size) || 0);
  const cards = visibleLibraryCards(state, player);
  if (cards.length > 1 && renderExpandedPile) return renderExpandedPile(cards);
  const card = cards[0];
  const inspectable = card && !String(card.id).startsWith("look-top-");
  const preview = () => { if (inspectable) hoverCard(card.id); };
  return <div className="zone-pile-slot library-pile-slot">
    <ZonePileLabel label={ui("Deck")} count={count} />
    <button type="button" className="zone-pile library-zone-pile" data-zone-pile="library"
      data-zone-owner={String(player.id ?? player.index)}
      data-object-id={inspectable ? card.id : undefined}
      data-zone-card={card ? "library" : undefined}
      aria-label={ui("{0}'s {1}, {2} cards", { 0: player.name, 1: ui("Deck"), 2: count })}
      onPointerEnter={(event) => { if (event.pointerType !== "touch") preview(); }}
      onPointerLeave={() => clearHover()} onFocus={preview} onBlur={() => clearHover()}
      onPointerDown={(event) => event.stopPropagation()}
      onClick={(event) => {
        event.stopPropagation();
        if (!inspectable) return;
        onCardClick?.(event, card);
        showAnchoredCardPreview(card.id, event.currentTarget, { placement: "zone" });
      }}>
      {card ? <ZonePileArt card={card} /> : <img src={HIDDEN_CARD_BACK_IMAGE_URL} alt="" draggable="false" />}
    </button>
  </div>;
}
