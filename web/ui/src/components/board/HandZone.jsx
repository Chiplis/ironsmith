import { Fragment, useRef, useMemo, useEffect, useLayoutEffect, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import { useHover } from "@/context/HoverContext";
import { useDragActions } from "@/context/DragContext";
import useNewCards from "@/hooks/useNewCards";
import GameCard from "@/components/cards/GameCard";
import { stagger } from "@/lib/motion/anime";
import useLayoutReflow from "@/lib/motion/useLayoutReflow";

const HAND_CARD_WIDTH = 124;
const HAND_CARD_BASE_OVERLAP = 20;
const HAND_ROULETTE_THRESHOLD = 10;
const HAND_ROULETTE_VISIBLE_CARDS = 7;
const HAND_ROULETTE_EDGE_PADDING = 12;
const HAND_ROULETTE_WRAP_GAP = 20;
const HAND_ROULETTE_CYCLE_COUNT = 3;
const HAND_ROULETTE_CENTER_CYCLE = 1;

/** Map card_types array to a glow kind for hand display. */
function handGlowFromTypes(cardTypes) {
  if (!cardTypes || cardTypes.length === 0) return "spell";
  // Priority order: more specific types win
  if (cardTypes.includes("instant")) return "instant";
  if (cardTypes.includes("sorcery")) return "sorcery";
  if (cardTypes.includes("creature")) return "creature";
  if (cardTypes.includes("enchantment")) return "enchantment";
  if (cardTypes.includes("battle")) return "battle";
  if (cardTypes.includes("planeswalker")) return "planeswalker";
  if (cardTypes.includes("artifact")) return "artifact";
  if (cardTypes.includes("land")) return "land";
  return "spell";
}

/**
 * Build a map of objectId → actions for all interactable hand cards.
 * Also builds a list of "extra" pseudo-hand cards from non-hand zones.
 */
function buildPlayableMaps(state, player) {
  const handPlayable = new Map();   // objectId → actions[] (from hand)
  const extraPlayable = new Map();  // objectId → { name, actions[], fromZone }

  const actions =
    state?.decision?.kind === "priority" && Array.isArray(state?.decision?.actions)
      ? state.decision.actions
      : [];

  const handIds = new Set((player?.hand_cards || []).map((c) => Number(c.id)));

  // Build zone lookup for card names (graveyard, exile, command)
  const cardNameById = new Map();
  for (const c of player?.graveyard_cards || []) cardNameById.set(Number(c.id), c.name);
  for (const c of player?.exile_cards || []) cardNameById.set(Number(c.id), c.name);
  for (const c of player?.command_cards || []) cardNameById.set(Number(c.id), c.name);
  for (const c of player?.hand_cards || []) cardNameById.set(Number(c.id), c.name);

  for (const action of actions) {
    if (action.object_id == null) {
      continue;
    }

    const objId = Number(action.object_id);
    const isHandCard = handIds.has(objId);
    const isHandInteraction =
      action.kind === "cast_spell"
      || action.kind === "play_land"
      || action.kind === "activate_ability"
      || action.kind === "activate_mana_ability"
      || action.kind === "serum_powder_mulligan"
      || action.kind === "begin_with_gemstone_caverns";

    if (isHandCard && isHandInteraction) {
      if (!handPlayable.has(objId)) handPlayable.set(objId, []);
      handPlayable.get(objId).push(action);
      continue;
    }

    // Card from another zone (graveyard flashback, exile, etc.)
    // Keep this list focused on cast/play actions so battlefield activations
    // don't show up as extra pseudo-hand cards.
    if (action.kind === "cast_spell" || action.kind === "play_land") {
      if (!extraPlayable.has(objId)) {
        extraPlayable.set(objId, {
          name: cardNameById.get(objId) || action.label?.replace(/^(Cast|Play)\s+/i, "") || `Card ${objId}`,
          actions: [],
          fromZone: action.from_zone || "other",
        });
      }
      extraPlayable.get(objId).actions.push(action);
    }
  }

  // Surface non-hand cards that currently have permission to be played/cast
  // from their zone, even if they aren't payable right now.
  const addPseudoHandCandidates = (cards, fromZone) => {
    for (const card of cards || []) {
      if (!card?.show_in_pseudo_hand) continue;
      const objId = Number(card.id);
      if (!Number.isFinite(objId) || handIds.has(objId)) continue;
      if (!extraPlayable.has(objId)) {
        extraPlayable.set(objId, {
          name: card.name || cardNameById.get(objId) || `Card ${objId}`,
          actions: [],
          fromZone,
        });
      }
    }
  };

  addPseudoHandCandidates(player?.graveyard_cards, "graveyard");
  addPseudoHandCandidates(player?.exile_cards, "exile");
  addPseudoHandCandidates(player?.command_cards, "command");

  return { handPlayable, extraPlayable };
}

function computeHandOverlap(total) {
  return Math.min(30, HAND_CARD_BASE_OVERLAP + Math.max(0, total - 5) * 1.5);
}

function computeRouletteWidth(total) {
  const visibleCards = Math.min(HAND_ROULETTE_VISIBLE_CARDS, total);
  const overlap = computeHandOverlap(total);
  const stride = HAND_CARD_WIDTH - overlap;
  return Math.round(
    HAND_CARD_WIDTH
    + Math.max(0, visibleCards - 1) * stride
    + (HAND_ROULETTE_EDGE_PADDING * 2)
  );
}

function buildHandCardRowStyle(index, total) {
  const overlap = computeHandOverlap(total);

  return {
    flex: `0 0 ${HAND_CARD_WIDTH}px`,
    width: `${HAND_CARD_WIDTH}px`,
    minWidth: `${HAND_CARD_WIDTH}px`,
    maxWidth: `${HAND_CARD_WIDTH}px`,
    marginLeft: index === 0 ? "0px" : `-${overlap.toFixed(1)}px`,
    zIndex: index + 2,
    "--card-rotate": "0deg",
    "--card-translate-x": "0px",
    "--card-translate-y": "0px",
  };
}

export default function HandZone({ player, selectedObjectId, onInspect, isExpanded = false }) {
  const { state } = useGame();
  const { hoverCard, clearHover, hoveredObjectId, hoveredLinkedObjectIds } = useHover();
  const { startDrag, updateDrag, endDrag } = useDragActions();
  const dragThresholdRef = useRef(null);
  const activePointerIdRef = useRef(null);
  const dragHandlersRef = useRef(null);
  const hoverClearTimerRef = useRef(null);
  const handListRef = useRef(null);
  const handScrollRef = useRef(null);
  const centerCycleRef = useRef(null);
  const rouletteCycleSpanRef = useRef(0);
  const rouletteRecenteringRef = useRef(false);
  const handCards = useMemo(
    () => (player?.can_view_hand && player?.hand_cards) || [],
    [player?.can_view_hand, player?.hand_cards]
  );
  const previousExpandedRef = useRef(isExpanded);
  const handCardIds = handCards.map((c) => c.id);
  const { newIds, bumpedIds } = useNewCards(handCardIds);

  const isMe = player?.id === state?.perspective;

  const { handPlayable, extraPlayable } = useMemo(
    () => isMe ? buildPlayableMaps(state, player) : { handPlayable: new Map(), extraPlayable: new Map() },
    [isMe, state, player]
  );
  const priorityActionObjectIds = useMemo(() => {
    const ids = new Set();
    const decision = state?.decision;
    if (!decision || decision.kind !== "priority" || decision.player !== state?.perspective) {
      return ids;
    }
    for (const action of decision.actions || []) {
      if (action.kind === "pass_priority" || action.object_id == null) continue;
      ids.add(String(action.object_id));
    }
    return ids;
  }, [state?.decision, state?.perspective]);

  // Extra playable cards as array for rendering
  const extraCards = useMemo(() => {
    const cards = [];
    for (const [objId, data] of extraPlayable) {
      cards.push({ id: objId, name: data.name, fromZone: data.fromZone, actions: data.actions });
    }
    return cards;
  }, [extraPlayable]);
  const hoverableHandObjectIds = useMemo(() => {
    const ids = new Set();
    for (const card of handCards) ids.add(String(card.id));
    for (const extra of extraCards) ids.add(String(extra.id));
    return ids;
  }, [extraCards, handCards]);
  const handLayoutSignature = useMemo(
    () => [
      handCards.map((card) => card.id).join("|"),
      extraCards.map((card) => `extra-${card.id}`).join("|"),
    ].join("::"),
    [extraCards, handCards]
  );
  const renderedHandCardCount = handCards.length + extraCards.length;
  const hasExtra = extraCards.length > 0;
  const isRoulette = renderedHandCardCount >= HAND_ROULETTE_THRESHOLD;
  const rouletteWidth = useMemo(
    () => computeRouletteWidth(renderedHandCardCount),
    [renderedHandCardCount]
  );
  const surfaceWidth = isRoulette
    ? `min(${rouletteWidth}px, calc(100vw - 290px))`
    : "fit-content";
  const rouletteCycleIndexes = isRoulette
    ? Array.from({ length: HAND_ROULETTE_CYCLE_COUNT }, (_, index) => index)
    : [HAND_ROULETTE_CENTER_CYCLE];
  const handEntries = useMemo(() => {
    const entries = handCards.map((card, visualIndex) => ({
      kind: "hand",
      key: `hand-${card.id}`,
      card,
      visualIndex,
    }));
    if (hasExtra && handCards.length > 0) {
      entries.push({ kind: "separator", key: "separator" });
    }
    for (let extraIndex = 0; extraIndex < extraCards.length; extraIndex += 1) {
      const extra = extraCards[extraIndex];
      entries.push({
        kind: "extra",
        key: `extra-${extra.id}`,
        extra,
        visualIndex: handCards.length + extraIndex,
      });
    }
    return entries;
  }, [extraCards, handCards, hasExtra]);

  useLayoutReflow(handListRef, handLayoutSignature, {
    children: ".game-card",
    disabled: isRoulette,
    delay: stagger(24),
    duration: 360,
    bounce: 0.14,
    enterFrom: { opacity: 0, x: 22, scale: 0.95 },
    leaveTo: { opacity: 0, x: -20, scale: 0.94 },
  });

  const handleCardClick = (_e, card) => {
    onInspect?.(card.id);
  };

  const clearPendingDragListeners = () => {
    const handlers = dragHandlersRef.current;
    if (!handlers) return;
    document.removeEventListener("pointermove", handlers.onMove);
    document.removeEventListener("pointerup", handlers.onUp);
    document.removeEventListener("pointercancel", handlers.onCancel);
    dragHandlersRef.current = null;
    activePointerIdRef.current = null;
  };

  useEffect(() => {
    return () => {
      if (hoverClearTimerRef.current) {
        clearTimeout(hoverClearTimerRef.current);
        hoverClearTimerRef.current = null;
      }
      const handlers = dragHandlersRef.current;
      if (!handlers) return;
      document.removeEventListener("pointermove", handlers.onMove);
      document.removeEventListener("pointerup", handlers.onUp);
      document.removeEventListener("pointercancel", handlers.onCancel);
      dragHandlersRef.current = null;
      activePointerIdRef.current = null;
    };
  }, []);

  useEffect(() => {
    const wasExpanded = previousExpandedRef.current;
    previousExpandedRef.current = isExpanded;
    if (wasExpanded && !isExpanded && hoveredObjectId != null) {
      if (hoverableHandObjectIds.has(String(hoveredObjectId))) {
        clearHover();
      }
    }
  }, [clearHover, hoverableHandObjectIds, hoveredObjectId, isExpanded]);

  const handleHoverEnter = useCallback((objectId) => {
    if (hoverClearTimerRef.current) {
      clearTimeout(hoverClearTimerRef.current);
      hoverClearTimerRef.current = null;
    }
    hoverCard(objectId);
  }, [hoverCard]);

  const handleHoverLeave = useCallback(() => {
    if (hoverClearTimerRef.current) {
      clearTimeout(hoverClearTimerRef.current);
    }
    // Small delay smooths hover-out when moving across dense hand cards.
    hoverClearTimerRef.current = setTimeout(() => {
      clearHover();
      hoverClearTimerRef.current = null;
    }, 110);
  }, [clearHover]);

  const recenterRouletteIfNeeded = useCallback(() => {
    if (!isRoulette) return;
    const scrollEl = handScrollRef.current;
    const cycleSpan = rouletteCycleSpanRef.current;
    if (!scrollEl || cycleSpan <= 0 || rouletteRecenteringRef.current) return;

    const minScrollLeft = cycleSpan * 0.5;
    const maxScrollLeft = cycleSpan * 1.5;
    let nextScrollLeft = scrollEl.scrollLeft;

    while (nextScrollLeft < minScrollLeft) {
      nextScrollLeft += cycleSpan;
    }
    while (nextScrollLeft > maxScrollLeft) {
      nextScrollLeft -= cycleSpan;
    }

    if (Math.abs(nextScrollLeft - scrollEl.scrollLeft) < 0.5) return;

    rouletteRecenteringRef.current = true;
    scrollEl.scrollLeft = nextScrollLeft;
    requestAnimationFrame(() => {
      rouletteRecenteringRef.current = false;
    });
  }, [isRoulette]);

  useLayoutEffect(() => {
    if (!isRoulette) {
      rouletteCycleSpanRef.current = 0;
      rouletteRecenteringRef.current = false;
      return;
    }

    const scrollEl = handScrollRef.current;
    const centerCycleEl = centerCycleRef.current;
    if (!scrollEl || !centerCycleEl) return;

    const cycleSpan = centerCycleEl.offsetWidth + HAND_ROULETTE_WRAP_GAP;
    rouletteCycleSpanRef.current = cycleSpan;
    rouletteRecenteringRef.current = true;
    scrollEl.scrollLeft = cycleSpan;
    requestAnimationFrame(() => {
      rouletteRecenteringRef.current = false;
    });
  }, [handLayoutSignature, isRoulette]);

  const handleRouletteWheel = useCallback((event) => {
    if (!isRoulette) return;
    const scrollEl = handScrollRef.current;
    if (!scrollEl) return;
    const primaryDelta = Math.abs(event.deltaX) > Math.abs(event.deltaY)
      ? event.deltaX
      : event.deltaY;
    if (primaryDelta === 0) return;
    event.preventDefault();
    scrollEl.scrollBy({
      left: primaryDelta * 1.1,
      behavior: "auto",
    });
  }, [isRoulette]);

  const handleRouletteScroll = useCallback(() => {
    recenterRouletteIfNeeded();
  }, [recenterRouletteIfNeeded]);

  const handlePointerDown = (e, card, plays, glowKind) => {
    if (plays.length === 0) return;
    if (e.button !== 0) return;
    e.preventDefault();
    clearPendingDragListeners();
    activePointerIdRef.current = e.pointerId;
    const sx = e.clientX;
    const sy = e.clientY;
    const sourceRect = e.currentTarget?.closest?.(".game-card")?.getBoundingClientRect?.() || null;
    dragThresholdRef.current = { sx, sy, card, plays, glowKind, sourceRect, dragging: false };

    const onMove = (me) => {
      if (activePointerIdRef.current != null && me.pointerId !== activePointerIdRef.current) {
        return;
      }
      const dt = dragThresholdRef.current;
      if (!dt) return;
      const dx = me.clientX - dt.sx;
      const dy = me.clientY - dt.sy;
      if (!dt.dragging && (dx * dx + dy * dy) > 64) {
        dt.dragging = true;
        startDrag(card.id, card.name, plays, glowKind, me.clientX, me.clientY, dt.sourceRect || null);
      }
      if (dt.dragging) {
        updateDrag(me.clientX, me.clientY);
      }
    };

    const onUp = (ue) => {
      if (activePointerIdRef.current != null && ue.pointerId !== activePointerIdRef.current) {
        return;
      }
      clearPendingDragListeners();
      const dt = dragThresholdRef.current;
      dragThresholdRef.current = null;
      if (dt && !dt.dragging) {
        handleCardClick(ue, card);
      }
    };

    const onCancel = (ce) => {
      if (activePointerIdRef.current != null && ce.pointerId !== activePointerIdRef.current) {
        return;
      }
      clearPendingDragListeners();
      dragThresholdRef.current = null;
      endDrag();
    };

    dragHandlersRef.current = { onMove, onUp, onCancel };
    document.addEventListener("pointermove", onMove);
    document.addEventListener("pointerup", onUp);
    document.addEventListener("pointercancel", onCancel);
  };

  if (!player) return null;

  if (player.can_view_hand) {
    const renderHandEntry = (entry, cycleIndex) => {
      const isPrimaryCycle = !isRoulette || cycleIndex === HAND_ROULETTE_CENTER_CYCLE;

      if (entry.kind === "separator") {
        return (
          <div
            key={`${cycleIndex}-${entry.key}`}
            className="mx-3 w-px self-stretch my-2 bg-[rgba(174,118,255,0.3)]"
          />
        );
      }

      if (entry.kind === "hand") {
        const { card, visualIndex } = entry;
        const plays = handPlayable.get(Number(card.id)) || [];
        const isPlayable = plays.length > 0;
        const baseGlowKind = isPlayable ? handGlowFromTypes(card.card_types) : null;
        const isActionLinkedHover = (
          hoveredLinkedObjectIds.has(String(card.id))
          || (
            hoveredObjectId != null
            && String(hoveredObjectId) === String(card.id)
            && priorityActionObjectIds.has(String(card.id))
          )
        );
        const glowKind = isActionLinkedHover ? "action-link" : baseGlowKind;
        const isNew = isPrimaryCycle && newIds.has(card.id);
        const isBumped = isPrimaryCycle && bumpedIds.has(card.id);
        let bumpDir = 0;
        if (isBumped) {
          if (visualIndex > 0 && newIds.has(handCards[visualIndex - 1].id)) bumpDir = 1;
          else if (visualIndex < handCards.length - 1 && newIds.has(handCards[visualIndex + 1].id)) bumpDir = -1;
        }
        return (
          <GameCard
            key={`${cycleIndex}-${entry.key}`}
            card={card}
            variant="hand"
            isPlayable={isPlayable}
            glowKind={glowKind}
            isNew={isNew}
            isBumped={isBumped}
            bumpDirection={bumpDir}
            handCircuitMode={isExpanded ? "full" : "top"}
            isInspected={selectedObjectId != null && String(card.id) === String(selectedObjectId)}
            onClick={isPlayable ? undefined : (e) => handleCardClick(e, card)}
            onPointerDown={isPlayable ? (e) => handlePointerDown(e, card, plays, glowKind) : undefined}
            onMouseEnter={() => handleHoverEnter(card.id)}
            onMouseLeave={handleHoverLeave}
            style={{
              ...buildHandCardRowStyle(visualIndex, renderedHandCardCount),
              scrollSnapAlign: isRoulette ? "start" : undefined,
            }}
          />
        );
      }

      const { extra, visualIndex } = entry;
      const card = { id: extra.id, name: extra.name };
      const plays = extra.actions;
      const isPlayable = plays.length > 0;
      const isActionLinkedHover = (
        hoveredLinkedObjectIds.has(String(extra.id))
        || (
          hoveredObjectId != null
          && String(hoveredObjectId) === String(extra.id)
          && priorityActionObjectIds.has(String(extra.id))
        )
      );
      return (
        <GameCard
          key={`${cycleIndex}-${entry.key}`}
          card={card}
          variant="hand"
          isPlayable={isPlayable}
          glowKind={isActionLinkedHover ? "action-link" : (isPlayable ? "extra" : null)}
          isNew={isPrimaryCycle}
          handCircuitMode={isExpanded ? "full" : "top"}
          isInspected={selectedObjectId != null && String(extra.id) === String(selectedObjectId)}
          onClick={plays.length === 0
            ? (e) => handleCardClick(e, card)
            : plays.length <= 1 ? undefined : (e) => handleCardClick(e, card)}
          onPointerDown={plays.length > 0 ? (e) => handlePointerDown(e, card, plays, "extra") : undefined}
          onMouseEnter={() => handleHoverEnter(extra.id)}
          onMouseLeave={handleHoverLeave}
          style={{
            ...buildHandCardRowStyle(visualIndex, renderedHandCardCount),
            scrollSnapAlign: isRoulette ? "start" : undefined,
          }}
        />
      );
    };

    return (
      <section
        className={`hand-zone-surface min-w-0 bg-transparent px-2 py-1 h-full min-h-0 overflow-visible ${isRoulette ? "hand-zone-surface-roulette" : "max-w-full"}`}
        style={{ width: surfaceWidth, maxWidth: isRoulette ? surfaceWidth : "100%" }}
      >
        <div className={`hand-zone-viewport min-h-0 h-full w-full min-w-0 overflow-visible ${isRoulette ? "hand-zone-viewport-roulette" : ""}`}>
          <div
            ref={handScrollRef}
            className={`hand-zone-scroll min-h-0 h-full w-full min-w-0 -mx-2 px-2 overflow-x-auto overflow-y-hidden overscroll-x-contain ${isRoulette ? "hand-zone-scroll-roulette" : ""}`}
            onScroll={handleRouletteScroll}
            onWheel={handleRouletteWheel}
          >
            <div
              ref={handListRef}
              className={`hand-zone-row flex min-h-full w-max flex-nowrap items-end pt-1 pb-2 overflow-visible ${isRoulette ? "hand-zone-row-roulette justify-start px-1.5" : "mx-auto min-w-full justify-center pl-4 pr-4"}`}
            >
              {rouletteCycleIndexes.map((cycleIndex) => (
                <Fragment key={`cycle-${cycleIndex}`}>
                  <div
                    ref={cycleIndex === HAND_ROULETTE_CENTER_CYCLE ? centerCycleRef : null}
                    className="hand-zone-cycle flex min-h-full flex-nowrap items-end overflow-visible"
                  >
                    {handEntries.map((entry) => renderHandEntry(entry, cycleIndex))}
                  </div>
                  {isRoulette && cycleIndex < HAND_ROULETTE_CYCLE_COUNT - 1 && (
                    <div
                      aria-hidden="true"
                      className="hand-zone-cycle-gap shrink-0"
                      style={{ width: `${HAND_ROULETTE_WRAP_GAP}px` }}
                    />
                  )}
                </Fragment>
              ))}

              {handCards.length === 0 && extraCards.length === 0 && (
                <div className="text-muted-foreground text-[17px] p-3 italic">Empty hand</div>
              )}
            </div>
          </div>
        </div>
      </section>
    );
  }

  // Opponent hand - show card backs
  const backs = Math.min(player.hand_size, 8);
  return (
    <section className="border border-[#41566f] bg-[#10161f] p-2 grid gap-1.5 h-full overflow-hidden" style={{ gridTemplateRows: "auto minmax(0,1fr)" }}>
      <h3 className="m-0 text-[#a4bdd7] uppercase tracking-wider text-[16px] font-semibold">
        Hand ({player.hand_size})
      </h3>
      <div className="flex gap-1.5 flex-nowrap pb-0.5 items-end min-h-0 overflow-hidden">
        {backs > 0
          ? Array.from({ length: backs }, (_, i) => (
              <div key={i} className="game-card w-[92px] min-w-[92px] min-h-[126px] p-1 text-[14px] grid content-end">
                <span className="card-label text-muted-foreground">Card</span>
              </div>
            ))
          : <div className="text-muted-foreground text-[17px] p-3 italic">Empty hand</div>
        }
      </div>
    </section>
  );
}
