import ZonePileLabel from "./ZonePileLabel";
import ZoneIcon, { LookEyeGlyph } from "./ZoneIcon";
import { createPortal } from "react-dom";
import useUiText from "@/i18n/useUiText";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Popover, PopoverTrigger, PopoverContent } from "@/components/ui/popover";
import { useGame } from "@/context/GameContext";
import { useI18n } from "@/i18n/I18nContext";
import { useCastTargeting, useCastZoneHovered } from "@/context/DragContext";
import { useHover } from "@/context/HoverContext";
import ZoneArt from "./ZonePileArt";
import { freshLookInZone, lookedZoneCards } from "@/lib/look-pile";
import { samePlayerId } from "@/lib/player-display";
import { isFaceUpZoneCard, pileZonesFor, zonePileCards } from "@/lib/zone-piles";
import { isObjectChosen, requestObjectSelection } from "@/lib/object-selection";
import { useChosenObjectIds } from "@/context/ObjectSelectionContext";
import LobbyChat from "@/components/right-rail/LobbyChat";
import SelectionCheckBadge from "@/components/cards/SelectionCheckBadge";
import { customCardCounterOverrides } from "@/lib/scryfall";
import {
  aggregateCounterEntries,
  buildCounterBadge,
  resolveCardCounters,
} from "@/components/cards/counter-badges";

// How long a zone takes to grow when it starts holding something to pick.
const ZONE_TARGET_GROW_MS = 220;
// Desktop HUD: the local Graveyard/Exile column rises toward the opponent
// piles above it to leave the decision dock more height underneath.
const LOCAL_PILES_MAX_LIFT = 160;
const LOCAL_PILES_CLEARANCE = 20;
export const ZONE_PILES_MOVED_EVENT = "ironsmith:zone-piles-moved";

// `variant` picks the trigger: "art" shows the top card (the command zone),
// "icon" a zone glyph with its count, and "eye" the cards this perspective
// looked at in `lookZone`. Icon and eye strips open beside their trigger.
function ZonePile({ player, zone, onCardClick, legalTargetObjectIds, cardsOverride, onOpenChange,
  variant = "art", lookZone = null, fresh = false, count: countOverride = null, onTriggerClick = null }) {
  const beside = variant !== "art";
  const ui = useUiText();
  const { state } = useGame();
  const { t } = useI18n();
  const chosenObjectIds = useChosenObjectIds();
  const { hoveredObjectId, hoveredLinkedObjectIds, hoverCard, clearHover, clearAnchoredCardPreview, showAnchoredCardPreview } = useHover();
  const castIntent = useCastTargeting();
  const castZoneHovered = useCastZoneHovered(player.id ?? player.index, zone);
  const [open, setOpen] = useState(false);
  useEffect(() => { onOpenChange?.(open); }, [open, onOpenChange]);
  const triggerRef = useRef(null);
  const menuRef = useRef(null);
  const closeTimerRef = useRef(null);
  const pointerInsideRef = useRef(false);
  // Dismissing the overlay exposes the trigger under the same stationary pointer.
  const dismissedRef = useRef(false);
  const changeOpen = (nextOpen) => {
    dismissedRef.current = !nextOpen;
    setOpen(nextOpen);
  };
  const keepOpen = () => {
    pointerInsideRef.current = true;
    clearTimeout(closeTimerRef.current);
    if (!dismissedRef.current) setOpen(true);
  };
  const closeAfterLeave = () => {
    clearTimeout(closeTimerRef.current);
    closeTimerRef.current = setTimeout(() => {
      if (!pointerInsideRef.current) setOpen(false);
    }, 120);
  };
  useEffect(() => () => clearTimeout(closeTimerRef.current), []);
  const [stripBounds, setStripBounds] = useState({ width: 240, cardWidth: 72 });
  useLayoutEffect(() => {
    if (!open) return undefined;
    const trigger = triggerRef.current;
    const battlefield = trigger?.closest(".has-zone-piles") || trigger?.closest(".table-shell");
    if (!battlefield) return undefined;
    const measure = () => {
      const anchor = trigger.getBoundingClientRect();
      const field = battlefield.getBoundingClientRect();
      // Target scaling animates back to normal as the strip opens. Its
      // transformed width must not become a permanent negative popover offset.
      // Strips beside a small trigger take the battlefield's card size instead.
      const fieldCard = battlefield.querySelector(".battlefield-row-card");
      const cardWidth = beside ? Math.max(52, Math.min(80, fieldCard?.offsetWidth || 64)) : anchor.width;
      const right = beside ? anchor.left - 6 : anchor.right;
      setStripBounds({ width: Math.max(0, (right - Math.max(field.left, 8)) + 6), cardWidth });
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(battlefield);
    observer.observe(trigger);
    window.addEventListener("resize", measure);
    return () => { observer.disconnect(); window.removeEventListener("resize", measure); };
  }, [open, zone, beside]);
  const cards = cardsOverride ?? zonePileCards(player, zone);
  // The inspector and its portaled glossary are part of this pile's surface.
  const isPreviewTarget = (target) => {
    if (!(target instanceof Element)) return false;
    const preview = [...document.querySelectorAll('[data-card-hover-preview="true"][data-visible="true"]')]
      .find(element => cards.some(card => String(card.id) === element.dataset.previewObjectId));
    if (!preview || !cards.some(card => String(card.id) === preview.dataset.previewObjectId)) return false;
    if (preview.contains(target)) return true;
    const tooltip = target.closest('[data-ui-layer="tooltip"]');
    return Boolean(tooltip?.id && Array.from(preview.querySelectorAll('[aria-controls]'))
      .some(trigger => trigger.getAttribute("aria-controls") === tooltip.id));
  };
  useEffect(() => {
    if (!open) return undefined;
    const trackPointer = (event) => {
      const inside = Boolean(menuRef.current?.contains(event.target)
        || triggerRef.current?.contains(event.target) || isPreviewTarget(event.target));
      if (inside) {
        pointerInsideRef.current = true;
        clearTimeout(closeTimerRef.current);
      } else if (pointerInsideRef.current) {
        pointerInsideRef.current = false;
        closeAfterLeave();
      }
    };
    document.addEventListener("pointerover", trackPointer);
    return () => document.removeEventListener("pointerover", trackPointer);
  });
  const topCard = variant === "eye" ? cards[0] : cards.find(isFaceUpZoneCard) || cards[0];
  // Looked-at library cards read deepest first, so the top card ends the
  // strip right beside the deck.
  const remainingCards = cards.filter((card) => card !== topCard);
  if (variant === "eye") remainingCards.reverse();
  const labelZone = lookZone ?? zone;
  const zoneLabel = labelZone === "library" ? "Deck" : labelZone === "command" ? "Command"
    : t(labelZone === "graveyard" ? "zone.graveyardFull" : "zone.exileFull");
  const label = variant === "eye" ? ui("Looked at in {0}", { 0: ui(zoneLabel) }) : zoneLabel;
  const count = countOverride ?? (zone === "graveyard" ? (player.graveyard_size ?? cards.length) : cards.length);
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
  // Stack target hovers may open their linked zones. Decision choices instead
  // preview their card on the collapsed pile without moving the options.
  const isStackTargeted = (card) => card != null && hoveredLinkedObjectIds.has(String(card.id));
  // The card a hover elsewhere stands for -- a dies trigger on the stack whose
  // source is already in here -- is marked the way an aimed-at card is. A card
  // the pointer is on itself is not: growing it would move it out from under
  // the pointer that opened it.
  const [rowHoveredId, setRowHoveredId] = useState(null);
  const isHoverSource = (card) => card != null
    && hoveredObjectId != null
    && String(card.id) === String(hoveredObjectId)
    && String(rowHoveredId) !== String(card.id);
  const optionHoveredCard = canChoose && (choosingOption || choosingObject || choosingTarget)
    ? cards.find(card => isHoverSource(card) && isFaceUpZoneCard(card)) : null;
  const holdsHoveredCard = !optionHoveredCard && ((hoveredObjectId != null
    && cards.some((card) => String(card?.id) === String(hoveredObjectId)))
    || cards.some(isStackTargeted));
  const openedByHoverRef = useRef(false);
  useEffect(() => {
    if (holdsHoveredCard) {
      openedByHoverRef.current = true;
      const timer = setTimeout(() => setOpen(true), 160);
      return () => clearTimeout(timer);
    }
    if (!openedByHoverRef.current) return undefined;
    openedByHoverRef.current = false;
    const timer = setTimeout(() => {
      if (!pointerInsideRef.current) setOpen(false);
    }, 160);
    return () => clearTimeout(timer);
  }, [holdsHoveredCard]);
  // The strip scrolls sideways; a target deep in a long graveyard would open
  // out of view, so once the strip is up it is scrolled to the marked card.
  const stackTargetKey = cards.filter((card) => isStackTargeted(card) || isHoverSource(card)).map((card) => card.id).join("|");
  useEffect(() => {
    if (!open || !stackTargetKey) return undefined;
    const frame = requestAnimationFrame(() => {
      menuRef.current?.querySelector('.zone-pile-card-list [data-stack-target="true"], .zone-pile-card-list [data-hover-source="true"]')
        ?.scrollIntoView({ block: "nearest", inline: "nearest" });
    });
    return () => cancelAnimationFrame(frame);
  }, [open, stackTargetKey]);
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
  const collapsedCard = optionHoveredCard || latestChosenCard || topCard;
  const chosenInPile = !open && choosingObject
    ? latestChosenCard || cards.find((card) => isObjectChosen(chosenObjectIds, card.id))
    : null;

  const cardHoverLabel = (card) => {
    const name = String(card?.name || ui("Face-down card"));
    if (!isFaceUpZoneCard(card)) return name;
    const counters = aggregateCounterEntries(
      customCardCounterOverrides(card.name)
        || resolveCardCounters(card?.counters, card?.counter_signature ?? card?.counterSignature)
    )
      .map(buildCounterBadge)
      .filter(Boolean)
      .map((counter) => counter.fullLabel);
    return counters.length > 0 ? `${name}\n${ui("Counters")}:\n${counters.join("\n")}` : name;
  };

  const renderCard = (card) => {
    const legal = canChoose && isLegal(card);
    const disabled = (choosingTarget || choosingObject) && !legal;
    const chosen = choosingObject && isObjectChosen(chosenObjectIds, card.id);
    const stackTargeted = isStackTargeted(card);
    const hoverSource = isHoverSource(card);
    const hoverLabel = cardHoverLabel(card);
    // A face-up card opens the preview, which names it already; a bare-name
    // native tooltip would sit on top of that preview.
    const previews = isFaceUpZoneCard(card) && !String(card.id).startsWith("look-top-");
    const rowLabel = card.position != null && variant === "eye"
      ? ui("{0}, {1} from the top", { 0: card.name, 1: card.position + 1 }) : card.name;
    // The check has to sit outside the row button to stay clickable, so the
    // row gets a wrapper of its own strip width.
    return <span key={card.id} className="zone-pile-card-slot" data-stack-target={stackTargeted ? "true" : undefined} data-hover-source={hoverSource ? "true" : undefined}>
      <button type="button" className={`zone-pile-card-row${chosen ? " is-chosen" : ""}`}
        aria-label={rowLabel || ui("Face-down card")}
        title={previews && hoverLabel === card.name ? undefined : hoverLabel}
        data-object-id={String(card.id).startsWith("look-top-") ? undefined : card.id} data-zone-card={zone}
        data-target-legal={legal ? "true" : undefined} data-stack-target={stackTargeted ? "true" : undefined}
        data-hover-source={hoverSource ? "true" : undefined} aria-disabled={disabled || undefined}
        onPointerEnter={(event) => {
          setRowHoveredId(card.id);
          if (event.pointerType === "touch" || !isFaceUpZoneCard(card) || String(card.id).startsWith("look-top-")) return;
          hoverCard(card.id);
        }}
        onPointerLeave={(event) => {
          setRowHoveredId((current) => (String(current) === String(card.id) ? null : current));
          if (event.pointerType !== "touch") clearHover();
        }}
        onFocus={() => {
          if (isFaceUpZoneCard(card) && !String(card.id).startsWith("look-top-")) hoverCard(card.id);
        }}
        onBlur={() => clearHover()}
        onClick={(event) => {
          if (disabled || (castIntent && state?.decision?.kind !== "targets")) return;
          if (choosingObject && legal) {
            // Searches take several picks: leave the zone open, add the card,
            // and let its check be the only way back out.
            requestObjectSelection(card.id, "add");
            return;
          }
          if (choosingTarget && legal) {
            clearAnchoredCardPreview();
            clearHover();
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
        {variant === "eye" && card.position != null ? (
          <span className="zone-pile-position" aria-hidden="true">{card.position === 0 ? ui("Top") : card.position + 1}</span>
        ) : null}
        {card.known_face_down ? <span className="zone-pile-known-face-down" aria-hidden="true"><LookEyeGlyph /></span> : null}
      </button>
      {chosen && <SelectionCheckBadge objectId={card.id} />}
    </span>;
  };

  const triggerContent = variant === "icon"
    ? <><ZoneIcon zone={zone} /><span className="zone-pile-count">{count}</span></>
    : variant === "eye"
      ? <><LookEyeGlyph /><span className="zone-pile-count">{cards.length}</span></>
      : <ZoneArt card={open ? topCard : collapsedCard} />;
  return (
    <Popover open={open && cards.length > 0} onOpenChange={changeOpen}>
      {/* The slot carries its transitions inline, so the growth a legal target
          brings has to be listed here too or the stylesheet's is overridden. */}
      <div className={`zone-pile-slot zone-pile-slot--${variant}`} style={{
        transition: `opacity 120ms ease, transform ${ZONE_TARGET_GROW_MS}ms ease`,
      }}>
      {variant === "art" && <ZonePileLabel label={ui(label)} count={count} />}
      <PopoverTrigger asChild>
        <button ref={triggerRef} type="button" className={`zone-pile zone-pile--${variant}`} data-zone-pile={zone}
          data-look-zone={lookZone ?? undefined}
          data-fresh-look={fresh ? "true" : undefined}
          title={beside ? `${ui(label)}: ${count}` : undefined}
          data-zone-owner={String(player.id ?? player.index)}
          data-option-hovered={optionHoveredCard ? "true" : undefined}
          data-zone-card={!open && optionHoveredCard ? zone : undefined}
          data-object-id={!open && optionHoveredCard ? optionHoveredCard.id : undefined}
          data-has-targets={hasLegalCards ? "true" : undefined}
          aria-label={ui("{0}'s {1}, {2} cards. Open zone", { 0: player.name, 1: ui(label), 2: variant === "eye" ? cards.length : count })}
          onPointerEnter={(event) => {
            if (zone === "look") dismissedRef.current = false;
            if (event.pointerType !== "touch") keepOpen();
          }}
          onPointerLeave={(event) => {
            if (!(event.relatedTarget instanceof Node) || !menuRef.current?.contains(event.relatedTarget)) dismissedRef.current = false;
            pointerInsideRef.current = event.relatedTarget instanceof Node && Boolean(menuRef.current?.contains(event.relatedTarget));
            closeAfterLeave();
          }}
          onPointerDown={(event) => event.stopPropagation()}
          onClick={(event) => {
            event.stopPropagation();
            if (event.detail > 0 && event.pointerType !== "touch") event.preventDefault();
            onTriggerClick?.(event);
          }}>
          {triggerContent}
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
      {/* Keep the top card over its pile and extend the strip to the left. */}
      <PopoverContent ref={menuRef} className={`zone-pile-menu zone-pile-menu--${variant}`} side="left" align={beside ? "center" : "start"}
        sideOffset={beside ? 6 : -(stripBounds.cardWidth + 6)} alignOffset={beside ? 0 : -6} avoidCollisions={beside} collisionPadding={8}
        updatePositionStrategy={zone === "look" ? "always" : "optimized"}
        data-local-zone-strip={samePlayerId(player.id ?? player.index, state?.perspective) ? "true" : undefined}
        style={{ "--zone-strip-width": `${stripBounds.width}px`, "--zone-strip-card-width": `${stripBounds.cardWidth}px` }}
        aria-label={ui("{0}'s {1}", { 0: player.name, 1: ui(label) })}
        onInteractOutside={(event) => {
          if (isPreviewTarget(event.target)) event.preventDefault();
        }}
        onOpenAutoFocus={(event) => event.preventDefault()}
        onCloseAutoFocus={(event) => event.preventDefault()}
        onPointerEnter={() => {
          pointerInsideRef.current = true;
          clearTimeout(closeTimerRef.current);
        }}
        onPointerLeave={(event) => {
          if (open && event.relatedTarget instanceof Node && !triggerRef.current?.contains(event.relatedTarget)) dismissedRef.current = false;
          pointerInsideRef.current = event.relatedTarget instanceof Node && Boolean(triggerRef.current?.contains(event.relatedTarget) || isPreviewTarget(event.relatedTarget));
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

// Face-down exiled cards this perspective knows come first, wearing their
// faces; the rest keep the engine's order.
function exileCardsWithKnownFaces(player, known) {
  const names = new Map(known.map((card) => [String(card.id), card]));
  const cards = zonePileCards(player, "exile");
  const knownCards = cards.filter((card) => names.has(String(card.id)))
    .map((card) => ({ ...card, ...names.get(String(card.id)), face_down: false, known_face_down: true }));
  return [...knownCards, ...cards.filter((card) => !names.has(String(card.id)))];
}

// One zone: its glyph and count, with an eye beside it while this perspective
// knows cards inside it that are hidden from the table.
function ZoneIconRow({ player, zone, onCardClick, legalTargetObjectIds, onOpenDecklist }) {
  const { state } = useGame();
  const looked = zone === "library" || zone === "exile" ? lookedZoneCards(state, player, zone) : [];
  const fresh = looked.length > 0 && freshLookInZone(state, player, zone);
  const count = zone === "library" ? Math.max(0, Number(player.library_size) || 0) : undefined;
  return <div className="zone-icon-row" data-zone-row={zone} data-zone-owner={String(player.id ?? player.index)}>
    {looked.length > 0 && <ZonePile key={`look-${zone}`} player={player} zone="look" lookZone={zone} variant="eye"
      cardsOverride={looked} fresh={fresh}
      onCardClick={onCardClick} legalTargetObjectIds={legalTargetObjectIds} />}
    <ZonePile player={player} zone={zone} variant="icon" count={count}
      cardsOverride={zone === "library" ? [] : zone === "exile" ? exileCardsWithKnownFaces(player, looked) : undefined}
      onTriggerClick={zone === "library" && onOpenDecklist ? () => onOpenDecklist(player) : null}
      onCardClick={onCardClick} legalTargetObjectIds={legalTargetObjectIds} />
  </div>;
}

export default function PlayerZonePiles({ player, onCardClick, legalTargetObjectIds, leadingRail = null, onOpenDecklist = null }) {
  const { state } = useGame();
  const ref = useRef(null);
  const [commandDock, setCommandDock] = useState(null);
  useLayoutEffect(() => {
    const piles = ref.current;
    const container = piles?.parentElement;
    if (!container) return undefined;
    const row = container.querySelector(".battlefield-row");
    let frame;
    const measure = () => {
      setCommandDock(piles.dataset.localZonePiles === "true"
        ? container.closest(".table-shell")?.querySelector(".local-command-zone-dock") ?? null
        : null);
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
      const containerClientTop = container.clientTop;
      let pileWidth = Math.min(56, cardWidth * 0.7);
      if (piles.dataset.opponentZonePiles === "true"
        && window.innerWidth >= 1024
        && piles.closest('.table-shell[data-focused-hud="true"]:not([data-multiplayer-board="true"])')) {
        // This rail starts at the top of the lane. Fit every pile and label
        // inside the opponent panel, whose overflow clips the last card.
        const panelBottom = container.closest(".battlefield-subpanel")?.getBoundingClientRect().bottom ?? bounds.bottom;
        const slots = piles.querySelectorAll(":scope > .zone-pile-slot, :scope > .zone-icon-row").length;
        const available = Math.max(0, Math.min(bounds.bottom, panelBottom) - bounds.top - 16);
        if (slots > 0) {
          // Reserve the label plus border/rounding space for each card.
          const cardHeight = Math.max(0, (available - slots * 22 - (slots - 1) * 6) / slots);
          pileWidth = Math.min(pileWidth, cardHeight * 63 / 88);
        }
      }
      piles.style.setProperty("--zone-pile-width", `${pileWidth}px`);
      if (board) {
        board.style.setProperty("--battlefield-objects-top", `${Math.max(0, top - boardBounds.top)}px`);
        const zoneTop = Math.max(0, top - boardBounds.top);
        const pilesTop = Math.max(0, boardBounds.top + zoneTop - bounds.top - containerClientTop);
        const nextPilesTop = bounds.top + containerClientTop + pilesTop;
        piles.style.setProperty("--zone-piles-top", `${pilesTop}px`);
        if (piles.dataset.localZonePiles === "true") {
          let lift = 0;
          const column = piles.querySelector(":scope > .zone-icon-row, :scope > .zone-pile-slot")?.getBoundingClientRect();
          if (column && piles.closest('.table-shell[data-focused-hud="true"]')) {
            const ceiling = [...document.querySelectorAll('[data-opponent-zone-piles="true"] :is(.zone-icon-row, .zone-pile-slot)')]
              .map((slot) => slot.getBoundingClientRect())
              .filter((rect) => rect.width > 0 && rect.left < column.right && rect.right > column.left
                && rect.bottom <= nextPilesTop)
              .reduce((bottom, rect) => Math.max(bottom, rect.bottom + LOCAL_PILES_CLEARANCE), -Infinity);
            lift = Math.max(0, Math.min(LOCAL_PILES_MAX_LIFT, nextPilesTop - ceiling));
          }
          const nextLift = `${Math.round(lift)}px`;
          if (piles.style.getPropertyValue("--local-piles-lift") !== nextLift) {
            piles.style.setProperty("--local-piles-lift", nextLift);
            window.dispatchEvent(new Event(ZONE_PILES_MOVED_EVENT));
          }
        }
        // Chat hangs 6px under Exile; give it whatever the board has left.
        piles.style.setProperty("--exile-chat-room", `${Math.max(0, boardBounds.bottom - nextPilesTop - pilesBounds.height - 12)}px`);
        // The stack anchor is independent of the zone column.
        const stackTop = 12;
        board.style.setProperty("--stack-area-top", `${stackTop}px`);
        // Overlay the unused left edge without removing a battlefield column.
        const fieldCards = [...(row?.querySelectorAll('.game-card') || [])]
          .map(card => card.getBoundingClientRect()).filter(rect => rect.width > 0 && rect.height > 0);
        const firstColumnLeft = fieldCards.length ? Math.min(...fieldCards.map(rect => rect.left)) : Math.min(360, window.innerWidth * .24);
        const railWidth = `${Math.max(0, firstColumnLeft - 8)}px`;
        if (board.style.getPropertyValue('--stack-frame-rail-width') !== railWidth) board.style.setProperty('--stack-frame-rail-width', railWidth);
        board.style.setProperty('--stack-frame-rail-left', `${-boardBounds.left}px`);
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
  const isLocal = samePlayerId(player.id ?? player.index, state?.perspective);
  const zones = ["exile", ...pileZonesFor(state).map(zone => zone === "exile" ? "library" : zone)];
  const dockCommand = isLocal && commandDock && zones.includes("command");
  if (!isLocal) zones.reverse();
  return <>
    <div ref={ref} className="player-zone-piles" data-player-zone-piles
      data-local-zone-piles={isLocal ? "true" : undefined}
      data-opponent-zone-piles={!isLocal ? "true" : undefined}>
      {/* Spans the piles' own height, so anything here lines up with them. */}
      {leadingRail}
      {zones.filter((zone) => !(dockCommand && zone === "command")).map((zone) => zone === "command"
        ? <ZonePile key={zone} player={player} zone={zone}
          onCardClick={onCardClick} legalTargetObjectIds={legalTargetObjectIds} />
        : <ZoneIconRow key={zone} player={player} zone={zone} onOpenDecklist={onOpenDecklist}
          onCardClick={onCardClick} legalTargetObjectIds={legalTargetObjectIds} />)}
    </div>
    {dockCommand && createPortal(
      <ZonePile player={player} zone="command" onCardClick={onCardClick} legalTargetObjectIds={legalTargetObjectIds} />,
      commandDock
    )}
    {isLocal && <div className="player-zone-chat-dock"><LobbyChat /></div>}
  </>;
}
