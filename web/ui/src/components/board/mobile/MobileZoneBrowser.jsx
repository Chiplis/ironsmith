import { X } from "lucide-react";
import useUiText from "@/i18n/useUiText";
import useModalFocus from "@/hooks/useModalFocus";
import { isFaceUpZoneCard } from "@/lib/zone-piles";
import ZonePileArt from "@/components/board/ZonePileArt";

const LABELS = { graveyard: "Graveyard", exile: "Exile", command: "Command", ante: "Ante" };

export default function MobileZoneBrowser({ player, zone, legalIds, choosing, onCardClick, onZoneChange, onClose }) {
  const ui = useUiText();
  const dialogRef = useModalFocus(onClose);
  const cards = player?.[`${zone}_cards`] || [];
  const title = ui(LABELS[zone] || zone);
  return (
    <section ref={dialogRef} tabIndex={-1} className="mobile-mtga-stack-browser mobile-zone-browser"
      role="dialog" aria-modal="true" aria-label={`${player?.name} · ${title}`}>
      <header className="mobile-mtga-stack-browser-header">
        <span>{player?.name}</span>
        <select className="mobile-zone-browser-select" aria-label={ui("Zone")} value={zone}
          onChange={(event) => onZoneChange(event.target.value, player)}>
          {Object.entries({ ...LABELS, library: "Library" }).filter(([key]) =>
            !["command", "ante"].includes(key) || player?.[`${key}_cards`]?.length
          ).map(([key, label]) => <option key={key} value={key}>{ui(label)}</option>)}
        </select>
        <span className="mobile-mtga-stack-browser-count">{cards.length}</span>
        <button type="button" className="mobile-mtga-stack-browser-close" aria-label={ui("Close")}
          onClick={onClose}><X className="size-4" aria-hidden="true" /></button>
      </header>
      <div className="mobile-zone-browser-list">
        {cards.length ? cards.map((card, index) => {
          const visible = isFaceUpZoneCard(card);
          const legal = legalIds.has(Number(card.id));
          return (
            <button key={card.id ?? index} type="button" className="mobile-mtga-stack-browser-row"
              data-object-id={card.id} data-zone-card={zone} data-target-legal={legal || undefined}
              disabled={!visible || (choosing && !legal)}
              onClick={(event) => onCardClick(event, card)}>
              <span className="mobile-zone-browser-art"><ZonePileArt card={card} /></span>
              <span className="mobile-mtga-stack-browser-name">{visible ? ui(card.name) : ui("Face-down card")}</span>
              {visible && card.type_line ? <span className="mobile-mtga-stack-browser-kind">{ui(card.type_line)}</span> : null}
            </button>
          );
        }) : <p className="mobile-zone-browser-empty">{ui("Empty")}</p>}
      </div>
    </section>
  );
}
