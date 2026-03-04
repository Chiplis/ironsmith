import { useState, useRef, useMemo, useEffect, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import { useHoverActions } from "@/context/HoverContext";
import { useDragActions } from "@/context/DragContext";
import useNewCards from "@/hooks/useNewCards";
import GameCard from "@/components/cards/GameCard";
import ActionPopover from "@/components/overlays/ActionPopover";

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
  const { state, dispatch } = useGame();
  const { hoverCard, clearHover } = useHoverActions();
  const { startDrag, updateDrag, endDrag } = useDragActions();
  const [popover, setPopover] = useState(null);
  const dragThresholdRef = useRef(null);
  const activePointerIdRef = useRef(null);
  const dragHandlersRef = useRef(null);
  const hoverClearTimerRef = useRef(null);
  const handCards = (player?.can_view_hand && player?.hand_cards) || [];
  const handCardIds = handCards.map((c) => c.id);
  const { newIds, bumpedIds } = useNewCards(handCardIds);

  const isMe = player?.id === state?.perspective;

  const { handPlayable, extraPlayable } = useMemo(
    () => isMe ? buildPlayableMaps(state, player) : { handPlayable: new Map(), extraPlayable: new Map() },
    [isMe, state, player]
  );

  // Extra playable cards as array for rendering
  const extraCards = useMemo(() => {
    const cards = [];
    for (const [objId, data] of extraPlayable) {
      cards.push({ id: objId, name: data.name, fromZone: data.fromZone, actions: data.actions });
    }
    return cards;
  }, [extraPlayable]);

  const handleCardClick = (e, card, actionsOverride) => {
    onInspect?.(card.id);
    const plays = actionsOverride || handPlayable.get(Number(card.id)) || [];
    if (plays.length > 0) {
      // e.currentTarget may be document (from pointerup handler), so find the card element
      const el = e.currentTarget?.closest?.(".game-card")
        || e.target?.closest?.(".game-card")
        || document.querySelector(`[data-object-id="${card.id}"]`);
      if (!el) return;
      const rect = el.getBoundingClientRect();
      setPopover({ anchorRect: rect, actions: plays, objectId: card.id });
    }
  };

  const handlePopoverAction = (action) => {
    setPopover(null);
    dispatch(
      { type: "priority_action", action_index: action.index },
      action.label
    );
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
    dragThresholdRef.current = { sx, sy, card, plays, glowKind, dragging: false };

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
        startDrag(card.id, card.name, plays, glowKind, me.clientX, me.clientY);
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
        className="bg-[#10161f] px-2 pt-1 pb-0.5 h-full min-h-0 overflow-hidden"
      >
        <div className="min-h-0 h-full -mx-2 px-2 overflow-x-auto overflow-y-hidden pb-0.5">
          <div className="flex gap-1.5 flex-nowrap items-center h-full w-max pl-1 pr-2">
            {/* Regular hand cards */}
            {handCards.map((card, i) => {
              const plays = handPlayable.get(Number(card.id)) || [];
              const isPlayable = plays.length > 0;
              const glowKind = isPlayable ? handGlowFromTypes(card.card_types) : null;
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
              return (
                <GameCard
                  key={`extra-${extra.id}`}
                  card={card}
                  variant="hand"
                  isPlayable={isPlayable}
                  glowKind={isPlayable ? "extra" : null}
                  isNew
                  isInspected={selectedObjectId != null && String(extra.id) === String(selectedObjectId)}
                  onClick={plays.length === 0
                    ? (e) => handleCardClick(e, card, plays)
                    : plays.length <= 1 ? undefined : (e) => handleCardClick(e, card, plays)}
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

        {popover && (
          <ActionPopover
            anchorRect={popover.anchorRect}
            actions={popover.actions}
            onAction={handlePopoverAction}
            onClose={() => setPopover(null)}
          />
        )}
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
