import { useRef, useMemo, useEffect, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import { useHover } from "@/context/HoverContext";
import { useDragActions } from "@/context/DragContext";
import useNewCards from "@/hooks/useNewCards";
import GameCard from "@/components/cards/GameCard";
import { stagger } from "@/lib/motion/anime";
import useLayoutReflow from "@/lib/motion/useLayoutReflow";

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

  // Build zone lookup for card names (graveyard, exile)
  const cardNameById = new Map();
  for (const c of player?.graveyard_cards || []) cardNameById.set(Number(c.id), c.name);
  for (const c of player?.exile_cards || []) cardNameById.set(Number(c.id), c.name);
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
      || action.kind === "activate_mana_ability";

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

  return { handPlayable, extraPlayable };
}

export default function HandZone({ player, selectedObjectId, onInspect }) {
  const { state } = useGame();
  const { hoverCard, clearHover, hoveredObjectId, hoveredLinkedObjectIds } = useHover();
  const { startDrag, updateDrag, endDrag } = useDragActions();
  const dragThresholdRef = useRef(null);
  const activePointerIdRef = useRef(null);
  const dragHandlersRef = useRef(null);
  const hoverClearTimerRef = useRef(null);
  const handListRef = useRef(null);
  const handCards = useMemo(
    () => (player?.can_view_hand && player?.hand_cards) || [],
    [player?.can_view_hand, player?.hand_cards]
  );
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
  const handLayoutSignature = useMemo(
    () => [
      handCards.map((card) => card.id).join("|"),
      extraCards.map((card) => `extra-${card.id}`).join("|"),
    ].join("::"),
    [extraCards, handCards]
  );

  useLayoutReflow(handListRef, handLayoutSignature, {
    children: ".game-card",
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
    const hasExtra = extraCards.length > 0;

    return (
      <section
        className="hand-zone-surface w-max max-w-full bg-transparent px-2 py-1 h-full min-h-0 overflow-hidden"
      >
        <div className="hand-zone-scroll min-h-0 h-full -mx-2 px-2 overflow-x-auto overflow-y-hidden pb-0.5">
          <div ref={handListRef} className="hand-zone-row flex gap-1.5 flex-nowrap items-start h-full w-max pl-1 pr-2">
            {/* Regular hand cards */}
            {handCards.map((card, i) => {
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
              const isNew = newIds.has(card.id);
              const isBumped = bumpedIds.has(card.id);
              let bumpDir = 0;
              if (isBumped) {
                if (i > 0 && newIds.has(handCards[i - 1].id)) bumpDir = 1;
                else if (i < handCards.length - 1 && newIds.has(handCards[i + 1].id)) bumpDir = -1;
              }
              return (
                <GameCard
                  key={card.id}
                  card={card}
                  variant="hand"
                  isPlayable={isPlayable}
                  glowKind={glowKind}
                  isNew={isNew}
                  isBumped={isBumped}
                  bumpDirection={bumpDir}
                  isInspected={isPlayable && selectedObjectId != null && String(card.id) === String(selectedObjectId)}
                  onClick={isPlayable ? undefined : (e) => handleCardClick(e, card)}
                  onPointerDown={isPlayable ? (e) => handlePointerDown(e, card, plays, glowKind) : undefined}
                  onMouseEnter={() => handleHoverEnter(card.id)}
                  onMouseLeave={handleHoverLeave}
                  style={{
                    flex: "0 0 124px",
                    width: "124px",
                    minWidth: "124px",
                    maxWidth: "124px",
                  }}
                />
              );
            })}

            {/* Separator when extra cards present */}
            {hasExtra && handCards.length > 0 && (
              <div className="w-px self-stretch my-2 bg-[rgba(174,118,255,0.3)]" />
            )}

            {/* Extra playable cards from other zones */}
            {extraCards.map((extra) => {
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
                  key={`extra-${extra.id}`}
                  card={card}
                  variant="hand"
                  isPlayable={isPlayable}
                  glowKind={isActionLinkedHover ? "action-link" : (isPlayable ? "extra" : null)}
                  isNew
                  isInspected={selectedObjectId != null && String(extra.id) === String(selectedObjectId)}
                  onClick={plays.length === 0
                    ? (e) => handleCardClick(e, card)
                    : plays.length <= 1 ? undefined : (e) => handleCardClick(e, card)}
                  onPointerDown={plays.length > 0 ? (e) => handlePointerDown(e, card, plays, "extra") : undefined}
                  onMouseEnter={() => handleHoverEnter(extra.id)}
                  onMouseLeave={handleHoverLeave}
                  style={{
                    flex: "0 0 124px",
                    width: "124px",
                    minWidth: "124px",
                    maxWidth: "124px",
                  }}
                />
              );
            })}

            {handCards.length === 0 && extraCards.length === 0 && (
              <div className="text-muted-foreground text-[17px] p-3 italic">Empty hand</div>
            )}
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
