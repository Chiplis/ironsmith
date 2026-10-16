import useUiText from "@/i18n/useUiText";
import { HIDDEN_CARD_BACK_IMAGE_URL } from "@/lib/scryfall";

export default function LibraryZonePile({ player }) {
  const ui = useUiText();
  const count = Math.max(0, Number(player.library_size) || 0);
  return <div className="zone-pile-slot library-pile-slot">
    <span className="zone-pile-label">{ui("Deck")} <strong>{count}</strong></span>
    <div className="zone-pile library-zone-pile" data-zone-pile="library"
      data-zone-owner={String(player.id ?? player.index)} role="img"
      aria-label={ui("{0}'s {1}, {2} cards", { 0: player.name, 1: ui("Deck"), 2: count })}>
      <img src={HIDDEN_CARD_BACK_IMAGE_URL} alt="" draggable="false" />
    </div>
  </div>;
}
