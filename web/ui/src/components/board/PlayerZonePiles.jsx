import useUiText from "@/i18n/useUiText";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Popover, PopoverTrigger, PopoverContent } from "@/components/ui/popover";
import { useGame } from "@/context/GameContext";
import { useI18n } from "@/i18n/I18nContext";
import { useCastTargeting, useCastZoneHovered } from "@/context/DragContext";
import { useHover } from "@/context/HoverContext";
import useScryfallImageUrl from "@/hooks/useScryfallImageUrl";
import { LOOK_DONE_EVENT, LOOK_FADE_MS, lookViewKey, temporaryLookView, persistentLookCards, mergeLookCards } from "@/lib/look-pile";
import { samePlayerId } from "@/lib/player-display";
import { isFaceUpZoneCard, PILE_ZONES, zonePileCards } from "@/lib/zone-piles";
import { isObjectChosen, requestObjectSelection } from "@/lib/object-selection";
import { useChosenObjectIds } from "@/context/ObjectSelectionContext";
import SelectionCheckBadge from "@/components/cards/SelectionCheckBadge";

// How long a zone takes to grow when it starts holding something to pick.
const ZONE_TARGET_GROW_MS = 220;

function ZoneArt({ card }) {
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

function ZonePile({ player, zone, onCardClick, legalTargetObjectIds, cardsOverride, fading = false, onOpenChange }) {
  const ui = useUiText();
  const { state } = useGame();
  const { t } = useI18n();
  const chosenObjectIds = useChosenObjectIds();
  const { hoverCard, clearHover, showAnchoredCardPreview } = useHover();
  const castIntent = useCastTargeting();
  const castZoneHovered = useCastZoneHovered(player.id ?? player.index, zone);
  const [open, setOpen] = useState(false);
  useEffect(() => { onOpenChange?.(open); }, [open, onOpenChange]);
  const triggerRef = useRef(null);
  const menuRef = useRef(null);
  const closeTimerRef = useRef(null);
  // Dismissing the overlay exposes the trigger under the same stationary pointer.
  const dismissedRef = useRef(false);
  const changeOpen = (nextOpen) => {
    dismissedRef.current = !nextOpen;
    setOpen(nextOpen);
  };
  const keepOpen = () => {
    clearTimeout(closeTimerRef.current);
    if (!dismissedRef.current) setOpen(true);
  };
  const closeAfterLeave = () => {
    clearTimeout(closeTimerRef.current);
    closeTimerRef.current = setTimeout(() => setOpen(false), 120);
  };
  useEffect(() => () => clearTimeout(closeTimerRef.current), []);
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
      setStripBounds({ width: Math.max(0, (zone === "look" ? Math.min(field.right, window.innerWidth - 8) - anchor.left : anchor.right - Math.max(field.left, 8)) + 6), cardWidth });
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(battlefield);
    observer.observe(trigger);
    window.addEventListener("resize", measure);
    return () => { observer.disconnect(); window.removeEventListener("resize", measure); };
  }, [open, zone]);
  const cards = cardsOverride ?? zonePileCards(player, zone);
  const topCard = cards.find(isFaceUpZoneCard) || cards[0];
  const remainingCards = cards.filter((card) => card !== topCard);
  const label = t(zone === "graveyard" ? "zone.graveyardFull" : zone === "look" ? "zone.look" : "zone.exileFull");
  const count = zone === "graveyard" ? (player.graveyard_size ?? cards.length) : cards.length;
  const decision = state?.decision?.kind === "targets" ? state.decision
    : castIntent?.targetDecision || state?.decision;
  const choosingTarget = decision?.kind === "targets";
  const choosingObject = decision?.kind === "select_objects";
  const choosingOption = decision?.kind === "select_options";
  const canChoose = samePlayerId(decision?.player, state?.perspective);
  const isLegal = (card) => choosingObject
    ? (decision.candidates || []).some((candidate) => String(candidate.id) === String(card.id) && candidate.legal !== false)
    : legalTargetObjectIds?.has(Number(card.id)) || (decision?.requirements || []).some((req) =>
      (req.legal_targets || []).some((target) => target.kind === "object" && String(target.object) === String(card.id))
    );
  const hasLegalCards = canChoose && (choosingTarget || choosingObject) && cards.some(isLegal);
  const hoverOpensZone = Boolean(castIntent && hasLegalCards && castZoneHovered);
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

  // A card chosen inside a closed pile is otherwise invisible: the pile shows
  // its most recent Look choice (or the normal top card). It wears the same check,
  // and clicking it unchooses that card without opening the strip. Once the
  // strip is open the cards carry their own checks, so this one steps aside.
  const latestChosenCard = zone === "look" && choosingObject
    ? chosenObjectIds.slice().reverse()
      .map((id) => cards.find((card) => String(card.id) === String(id)))
      .find(Boolean)
    : null;
  const collapsedCard = latestChosenCard || topCard;
  const chosenInPile = !open && choosingObject
    ? latestChosenCard || cards.find((card) => isObjectChosen(chosenObjectIds, card.id))
    : null;

  const renderCard = (card) => {
    const legal = canChoose && isLegal(card);
    const disabled = (choosingTarget || choosingObject) && !legal;
    const chosen = choosingObject && isObjectChosen(chosenObjectIds, card.id);
    // The check has to sit outside the row button to stay clickable, so the
    // row gets a wrapper of its own strip width.
    return <span key={card.id} className="zone-pile-card-slot">
      <button type="button" className={`zone-pile-card-row${chosen ? " is-chosen" : ""}`}
        aria-label={card.name || ui("Face-down card")}
        data-object-id={String(card.id).startsWith("look-top-") ? undefined : card.id} data-zone-card={zone}
        data-target-legal={legal ? "true" : undefined} disabled={disabled}
        onPointerEnter={(event) => {
          if (event.pointerType === "touch" || disabled || !isFaceUpZoneCard(card) || String(card.id).startsWith("look-top-")) return;
          hoverCard(card.id);
        }}
        onPointerLeave={(event) => {
          if (event.pointerType !== "touch") clearHover();
        }}
        onFocus={() => {
          if (!disabled && isFaceUpZoneCard(card) && !String(card.id).startsWith("look-top-")) hoverCard(card.id);
        }}
        onBlur={() => clearHover()}
        onClick={(event) => {
          if (castIntent && state?.decision?.kind !== "targets") return;
          if (choosingObject && legal) {
            // Searches take several picks: leave the zone open, add the card,
            // and let its check be the only way back out.
            requestObjectSelection(card.id, "add");
            return;
          }
          if (choosingTarget && legal) {
            window.dispatchEvent(new CustomEvent("ironsmith:target-choice", {
              detail: { target: { kind: "object", object: Number(card.id) } },
            }));
            changeOpen(false);
            return;
          }
          if (choosingOption && legal) {
            const option = (decision.options || []).find((candidate) =>
              candidate.object_id != null && String(candidate.object_id) === String(card.id)
            );
            if (option) {
              window.dispatchEvent(new CustomEvent("ironsmith:select-option-choice", {
                detail: { optionIndex: option.index },
              }));
              changeOpen(false);
              return;
            }
          }
          const anchor = event.currentTarget;
          onCardClick?.(event, card);
          // A zone card is an explicit selection, so show its full frame
          // immediately while keeping the normal inspector selection in sync.
          // Do this after onCardClick because that callback clears any previous
          // anchored preview as part of changing the selected object.
          if (isFaceUpZoneCard(card) && !String(card.id).startsWith("look-top-")) {
            showAnchoredCardPreview(card.id, anchor, { placement: "zone" });
          }
          if (choosingTarget || choosingObject) changeOpen(false);
        }}>
        <ZoneArt card={card} />
      </button>
      {chosen && <SelectionCheckBadge objectId={card.id} />}
    </span>;
  };

  return (
    <Popover open={open} onOpenChange={changeOpen}>
      {/* The slot carries its transitions inline, so the growth a legal target
          brings has to be listed here too or the stylesheet's is overridden. */}
      <div className="zone-pile-slot" style={{
        opacity: fading ? 0 : 1,
        transition: `${fading ? `opacity ${LOOK_FADE_MS}ms linear` : "opacity 120ms ease"}, transform ${ZONE_TARGET_GROW_MS}ms ease`,
      }}>
      <span className="zone-pile-label">{ui(label)} <strong>{count}</strong></span>
      <PopoverTrigger asChild>
        <button ref={triggerRef} type="button" className="zone-pile" data-zone-pile={zone}
          data-zone-owner={String(player.id ?? player.index)}
          data-has-targets={hasLegalCards ? "true" : undefined}
          aria-label={ui("{0}'s {1}, {2} cards. Open zone", { 0: player.name, 1: ui(label), 2: count })}
          onPointerEnter={(event) => {
            if (zone === "look") dismissedRef.current = false;
            if (event.pointerType !== "touch") keepOpen();
          }}
          onPointerLeave={(event) => {
            if (!(event.relatedTarget instanceof Node) || !menuRef.current?.contains(event.relatedTarget)) dismissedRef.current = false;
            closeAfterLeave();
          }}
          onPointerDown={(event) => event.stopPropagation()}
          onClick={(event) => {
            event.stopPropagation();
            if (event.detail > 0 && event.pointerType !== "touch") event.preventDefault();
          }}>
          <ZoneArt card={open ? topCard : collapsedCard} />
        </button>
      </PopoverTrigger>
      {chosenInPile ? (
        <SelectionCheckBadge
          objectId={chosenInPile.id}
          className="zone-pile-check"
          label={ui("Deselect {0}", { 0: chosenInPile.name || "card" })}
        />
      ) : null}
      </div>
      <PopoverContent ref={menuRef} className={`zone-pile-menu${zone === "look" ? " zone-pile-menu--look" : ""}`} side={zone === "look" ? "right" : "left"} align="start" sideOffset={-(stripBounds.cardWidth + 6)} alignOffset={-6} avoidCollisions={false}
        data-local-zone-strip={samePlayerId(player.id ?? player.index, state?.perspective) ? "true" : undefined}
        style={{ "--zone-strip-width": `${stripBounds.width}px`, "--zone-strip-card-width": `${stripBounds.cardWidth}px` }}
        aria-label={ui("{0}'s {1}", { 0: player.name, 1: ui(label) })}
        onOpenAutoFocus={(event) => event.preventDefault()}
        onCloseAutoFocus={(event) => event.preventDefault()}
        onPointerEnter={() => clearTimeout(closeTimerRef.current)}
        onPointerLeave={(event) => {
          if (open && event.relatedTarget instanceof Node && !triggerRef.current?.contains(event.relatedTarget)) dismissedRef.current = false;
          closeAfterLeave();
        }}
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

function LookPile({ player, onCardClick, legalTargetObjectIds }) {
  const { state } = useGame();
  const view = temporaryLookView(state);
  const key = lookViewKey(view, state?.decision);
  const [completedKey, setCompletedKey] = useState("");
  const [retained, setRetained] = useState(null);
  const [open, setOpen] = useState(false);
  const active = Boolean(key) && key !== completedKey;
  const persistent = persistentLookCards(state);
  useEffect(() => {
    const done = () => { setCompletedKey(key); setRetained(view); };
    window.addEventListener(LOOK_DONE_EVENT, done);
    return () => window.removeEventListener(LOOK_DONE_EVENT, done);
  }, [key, view]);
  useEffect(() => {
    if (active || open || !retained) return undefined;
    const timer = setTimeout(() => setRetained(null), LOOK_FADE_MS);
    return () => clearTimeout(timer);
  }, [active, open, retained]);
  const cards = mergeLookCards(persistent, (active ? view : retained)?.cards || []);
  if (!cards.length) return null;
  return <>
    {active && <div key={key} className="look-eye-effect" aria-hidden="true">
      <svg viewBox="0 0 96 54" role="presentation">
        <path className="look-eye-glow" d="M8 27 C27 4 69 4 88 27 C69 50 27 50 8 27 Z" />
        <path className="look-eye-lid" d="M8 27 C27 4 69 4 88 27" />
        <ellipse className="look-eye-iris" cx="48" cy="27" rx="12" ry="15" />
        <circle className="look-eye-pupil" cx="48" cy="27" r="5" />
      </svg>
    </div>}
    <ZonePile player={player} zone="look" cardsOverride={cards}
      fading={!active && !open && persistent.length === 0}
      onOpenChange={setOpen} onCardClick={onCardClick} legalTargetObjectIds={legalTargetObjectIds} />
  </>;
}

export default function PlayerZonePiles({ player, onCardClick, legalTargetObjectIds }) {
  const { state } = useGame();
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
      const board = container.closest(".my-zone-board-shell");
      // Read geometry before writing styles; target highlights must not force
      // another synchronous layout of the entire battlefield.
      const boardBounds = board?.getBoundingClientRect();
      const pilesBounds = board ? piles.getBoundingClientRect() : null;
      piles.style.setProperty("--zone-pile-width", `${Math.min(56, cardWidth * 0.7)}px`);
      if (board) {
        board.style.setProperty("--battlefield-objects-top", `${Math.max(0, top - boardBounds.top)}px`);
        const lookTop = Math.max(0, top - boardBounds.top);
        const pileWidth = Math.min(56, cardWidth * 0.7);
        // Keep Look above the stack, reserving its label and card height even
        // between temporary reveals so the stack never jumps or overlaps it.
        piles.style.setProperty("--look-area-top", `${boardBounds.top + lookTop - pilesBounds.top}px`);
        piles.style.setProperty("--look-area-left", `${boardBounds.left + 70 - pilesBounds.left}px`);
        board.style.setProperty("--stack-area-top", `${lookTop + 20 + pileWidth * 88 / 63 + 8}px`);
      }
    };
    const schedule = () => { cancelAnimationFrame(frame); frame = requestAnimationFrame(measure); };
    measure();
    const observer = new ResizeObserver(schedule);
    observer.observe(container);
    observer.observe(piles);
    if (row) observer.observe(row);
    const mutations = new MutationObserver((records) => {
      // Card internals change for hover, targeting and animation. Only the
      // grid and its positioned wrappers determine the piles' placement.
      const layoutClasses = (value) => (value || "").split(/\s+/)
        .filter(name => name.startsWith("battlefield-row-card--") || name === "tapped").join(" ");
      if (records.some(({ target, type, attributeName, oldValue }) => {
        if (target === row) return true;
        if (!(target instanceof Element) || !target.matches(".battlefield-row-card")) return false;
        if (type !== "attributes") return false;
        if (attributeName === "class") return layoutClasses(oldValue) !== layoutClasses(target.className);
        return oldValue !== target.getAttribute(attributeName);
      })) schedule();
    });
    if (row) mutations.observe(row, { attributes: true, attributeOldValue: true, childList: true, subtree: true, attributeFilter: ["style", "class"] });
    window.addEventListener("resize", schedule);
    return () => { cancelAnimationFrame(frame); observer.disconnect(); mutations.disconnect(); window.removeEventListener("resize", schedule); };
  }, [player]);
  return <div ref={ref} className="player-zone-piles" data-player-zone-piles
    data-local-zone-piles={samePlayerId(player.id ?? player.index, state?.perspective) ? "true" : undefined}>
    {PILE_ZONES.map((zone) => <ZonePile key={zone} player={player} zone={zone}
      onCardClick={onCardClick} legalTargetObjectIds={legalTargetObjectIds} />)}
    {samePlayerId(player.id ?? player.index, state?.perspective) &&
      <div className="player-look-pile"><LookPile key={state?.perspective} player={player} onCardClick={onCardClick} legalTargetObjectIds={legalTargetObjectIds} /></div>}
  </div>;
}
