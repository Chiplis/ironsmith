import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Popover, PopoverTrigger, PopoverContent } from "@/components/ui/popover";
import { useGame } from "@/context/GameContext";
import { useCastTargeting, useCastTargetHover } from "@/context/DragContext";
import useScryfallImageUrl from "@/hooks/useScryfallImageUrl";
import { samePlayerId } from "@/lib/player-display";
import { isFaceUpZoneCard, PILE_ZONES, zonePileCards } from "@/lib/zone-piles";

function ZoneArt({ card }) {
  const name = isFaceUpZoneCard(card) ? card.name : null;
  const url = useScryfallImageUrl(name, "normal");
  return url ? <img src={url} alt="" draggable={false} loading="lazy" referrerPolicy="no-referrer" />
    : <span className="zone-pile-placeholder" aria-hidden="true">{card ? "◇" : "—"}</span>;
}

function ZonePile({ player, zone, onCardClick, legalTargetObjectIds }) {
  const { state } = useGame();
  const castIntent = useCastTargeting();
  const castHover = useCastTargetHover();
  const [open, setOpen] = useState(false);
  const triggerRef = useRef(null);
  const [stripBounds, setStripBounds] = useState({ width: 240, cardWidth: 72 });
  useLayoutEffect(() => {
    if (!open) return undefined;
    const trigger = triggerRef.current;
    const battlefield = trigger?.closest(".has-zone-piles");
    if (!battlefield) return undefined;
    const measure = () => {
      const anchor = trigger.getBoundingClientRect();
      const field = battlefield.getBoundingClientRect();
      const cardWidth = anchor.width;
      setStripBounds({ width: Math.max(0, anchor.right - Math.max(field.left, 8) + 6), cardWidth });
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(battlefield);
    observer.observe(trigger);
    window.addEventListener("resize", measure);
    return () => { observer.disconnect(); window.removeEventListener("resize", measure); };
  }, [open]);
  const cards = zonePileCards(player, zone);
  const topCard = cards.find(isFaceUpZoneCard) || cards[0];
  const remainingCards = cards.filter((card) => card !== topCard);
  const label = zone === "graveyard" ? "Graveyard" : "Exile";
  const count = zone === "graveyard" ? (player.graveyard_size ?? cards.length) : cards.length;
  const decision = state?.decision?.kind === "targets" ? state.decision
    : castIntent?.targetDecision || state?.decision;
  const choosingTarget = decision?.kind === "targets";
  const choosingObject = decision?.kind === "select_objects";
  const canChoose = samePlayerId(decision?.player, state?.perspective);
  const isLegal = (card) => choosingObject
    ? (decision.candidates || []).some((candidate) => String(candidate.id) === String(card.id) && candidate.legal !== false)
    : legalTargetObjectIds?.has(Number(card.id)) || (decision?.requirements || []).some((req) =>
      (req.legal_targets || []).some((target) => target.kind === "object" && String(target.object) === String(card.id))
    );
  const hasLegalCards = canChoose && (choosingTarget || choosingObject) && cards.some(isLegal);
  const hoverOpensZone = Boolean(castIntent && hasLegalCards && castHover?.kind === "zone"
    && castHover.zone === zone && String(castHover.playerId) === String(player.id ?? player.index));
  useEffect(() => {
    if (!hoverOpensZone) return undefined;
    const timer = setTimeout(() => setOpen(true), 160);
    return () => clearTimeout(timer);
  }, [hoverOpensZone]);
  useEffect(() => {
    const openTargetZone = (event) => {
      if (event.detail?.zone === zone && String(event.detail?.playerId) === String(player.id ?? player.index)) setOpen(true);
    };
    window.addEventListener("ironsmith:open-target-zone", openTargetZone);
    return () => window.removeEventListener("ironsmith:open-target-zone", openTargetZone);
  }, [player.id, player.index, zone]);

  const renderCard = (card) => {
    const legal = canChoose && isLegal(card);
    const disabled = (choosingTarget || choosingObject) && !legal;
    return <button type="button" key={card.id} className="zone-pile-card-row"
      aria-label={card.name || "Face-down card"}
      data-object-id={card.id} data-zone-card={zone}
      data-target-legal={legal ? "true" : undefined} disabled={disabled}
      onClick={(event) => {
        if (castIntent && state?.decision?.kind !== "targets") return;
        onCardClick?.(event, card);
        if (choosingTarget || choosingObject) setOpen(false);
      }}>
      <ZoneArt card={card} />
    </button>;
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <div className="zone-pile-slot">
      <span className="zone-pile-label">{label} <strong>{count}</strong></span>
      <PopoverTrigger asChild>
        <button ref={triggerRef} type="button" className="zone-pile" data-zone-pile={zone}
          data-zone-owner={String(player.id ?? player.index)}
          data-has-targets={hasLegalCards ? "true" : undefined}
          aria-label={`${player.name}'s ${label}, ${count} cards. Open zone`}
          onPointerDown={(event) => event.stopPropagation()}
          onClick={(event) => event.stopPropagation()}>
          <ZoneArt card={topCard} />
        </button>
      </PopoverTrigger>
      </div>
      <PopoverContent className="zone-pile-menu" side="left" align="start" sideOffset={-(stripBounds.cardWidth + 6)} alignOffset={-6} avoidCollisions={false}
        style={{ "--zone-strip-width": `${stripBounds.width}px`, "--zone-strip-card-width": `${stripBounds.cardWidth}px` }}
        aria-label={`${player.name}'s ${label}`}
        onOpenAutoFocus={(event) => { if (castIntent) event.preventDefault(); }}
        onClick={(event) => event.stopPropagation()}
        onPointerDown={(event) => event.stopPropagation()}>
        <div className="zone-pile-card-list" onWheel={(event) => {
          if (Math.abs(event.deltaY) > Math.abs(event.deltaX)) {
            event.currentTarget.scrollLeft += event.deltaY;
          }
        }}>
          {remainingCards.map(renderCard)}
        </div>
        {topCard ? renderCard(topCard) : <div className="zone-pile-card-row"><ZoneArt /></div>}
      </PopoverContent>
    </Popover>
  );
}

export default function PlayerZonePiles({ player, onCardClick, legalTargetObjectIds }) {
  const ref = useRef(null);
  useLayoutEffect(() => {
    const piles = ref.current;
    const container = piles?.parentElement;
    if (!container) return undefined;
    const row = container.querySelector(".battlefield-row");
    let frame;
    const measure = () => {
      const bounds = container.getBoundingClientRect();
      const cards = Array.from(container.querySelectorAll(".battlefield-row-card"))
        .map((card) => card.getBoundingClientRect()).filter((rect) => rect.width > 0 && rect.height > 0);
      const rowBounds = row?.getBoundingClientRect();
      const top = cards.length ? Math.min(...cards.map((card) => card.top)) : (rowBounds?.top ?? bounds.top) + 12;
      const cardWidth = cards[0]?.width || (row ? parseFloat(getComputedStyle(row).getPropertyValue("--bf-card-width")) : 72) || 72;
      piles.style.top = `${Math.max(0, top - bounds.top)}px`;
      piles.style.setProperty("--zone-pile-width", `${Math.min(56, cardWidth * 0.7)}px`);
      const board = container.closest(".my-zone-board-shell");
      if (board) board.style.setProperty("--battlefield-objects-top", `${Math.max(0, top - board.getBoundingClientRect().top)}px`);
    };
    const schedule = () => { cancelAnimationFrame(frame); frame = requestAnimationFrame(measure); };
    measure();
    const observer = new ResizeObserver(schedule);
    observer.observe(container);
    if (row) observer.observe(row);
    const mutations = new MutationObserver(schedule);
    if (row) mutations.observe(row, { attributes: true, childList: true, subtree: true, attributeFilter: ["style", "class"] });
    window.addEventListener("resize", schedule);
    return () => { cancelAnimationFrame(frame); observer.disconnect(); mutations.disconnect(); window.removeEventListener("resize", schedule); };
  }, [player]);
  return <div ref={ref} className="player-zone-piles" data-player-zone-piles>
    {PILE_ZONES.map((zone) => <ZonePile key={zone} player={player} zone={zone}
      onCardClick={onCardClick} legalTargetObjectIds={legalTargetObjectIds} />)}
  </div>;
}
