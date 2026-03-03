import { useState, useRef, useMemo } from "react";
import { useGame } from "@/context/GameContext";
import { useHover } from "@/context/HoverContext";
import { useDrag } from "@/context/DragContext";
import useNewCards from "@/hooks/useNewCards";
import GameCard from "@/components/cards/GameCard";
import ActionPopover from "@/components/overlays/ActionPopover";

/**
 * Build a map of objectId → actions for all castable/playable cards.
 * Also builds a list of "extra" playable cards from non-hand zones.
 */
function buildPlayableMaps(state, player) {
  const handPlayable = new Map();   // objectId → actions[] (from hand)
  const extraPlayable = new Map();  // objectId → { name, actions[], fromZone }

  if (!state?.decision?.kind === "priority" || !state.decision?.actions) {
    return { handPlayable, extraPlayable };
  }

  const handIds = new Set((player?.hand_cards || []).map((c) => Number(c.id)));

  // Build zone lookup for card names (graveyard, exile)
  const cardNameById = new Map();
  for (const c of player?.graveyard_cards || []) cardNameById.set(Number(c.id), c.name);
  for (const c of player?.exile_cards || []) cardNameById.set(Number(c.id), c.name);
  for (const c of player?.hand_cards || []) cardNameById.set(Number(c.id), c.name);

  for (const action of state.decision.actions) {
    if (
      (action.kind === "cast_spell" || action.kind === "play_land") &&
      action.object_id != null
    ) {
      const objId = Number(action.object_id);

      if (handIds.has(objId)) {
        // Card is in hand
        if (!handPlayable.has(objId)) handPlayable.set(objId, []);
        handPlayable.get(objId).push(action);
      } else {
        // Card from another zone (graveyard flashback, exile, etc.)
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
  }

  return { handPlayable, extraPlayable };
}

export default function HandZone({ player, selectedObjectId, onInspect }) {
  const { state, dispatch } = useGame();
  const { hoveredObjectId, hoverCard, clearHover } = useHover();
  const { dragState, startDrag, updateDrag, endDrag } = useDrag();
  const [popover, setPopover] = useState(null);
  const dragThresholdRef = useRef(null);
  const handCards = (player?.can_view_hand && player?.hand_cards) || [];
  const handCardIds = useMemo(() => handCards.map((c) => c.id), [handCards]);
  const { newIds, bumpedIds } = useNewCards(handCardIds);

  if (!player) return null;

  const isMe = player.id === state?.perspective;

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

  const handlePointerDown = (e, card, plays, glowKind) => {
    if (plays.length === 0) return;
    if (e.button !== 0) return;
    e.preventDefault();
    const sx = e.clientX;
    const sy = e.clientY;
    dragThresholdRef.current = { sx, sy, card, plays, glowKind, dragging: false };

    const onMove = (me) => {
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
      document.removeEventListener("pointermove", onMove);
      document.removeEventListener("pointerup", onUp);
      const dt = dragThresholdRef.current;
      dragThresholdRef.current = null;
      if (dt && !dt.dragging) {
        handleCardClick(ue, card);
      }
    };

    document.addEventListener("pointermove", onMove);
    document.addEventListener("pointerup", onUp);
  };

  if (player.can_view_hand) {
    const hasExtra = extraCards.length > 0;

    return (
      <section className="bg-[#10161f] px-2 pt-3 pb-1 grid gap-1 h-full overflow-visible" style={{ gridTemplateRows: "auto minmax(0,1fr)" }}>
        <h3 className="m-0 text-[#a4bdd7] uppercase tracking-wider text-[14px] font-semibold">
          Hand
        </h3>
        <div className="flex gap-1.5 flex-nowrap pb-0.5 items-end min-h-0 overflow-visible">
          {/* Regular hand cards */}
          {handCards.map((card, i) => {
            const plays = handPlayable.get(Number(card.id)) || [];
            const isPlayable = plays.length > 0;
            const glowKind = isPlayable
              ? plays.some((a) => a.kind === "play_land") ? "land" : "spell"
              : null;
            const cardId = String(card.id);
            const isDragging = dragState?.objectId === card.id;
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
                isHovered={hoveredObjectId === cardId}
                isDragging={isDragging}
                isNew={isNew}
                isBumped={isBumped}
                bumpDirection={bumpDir}
                isInspected={selectedObjectId != null && cardId === String(selectedObjectId)}
                onClick={isPlayable ? undefined : (e) => handleCardClick(e, card)}
                onPointerDown={isPlayable ? (e) => handlePointerDown(e, card, plays, glowKind) : undefined}
                onMouseEnter={() => hoverCard(card.id)}
                onMouseLeave={clearHover}
              />
            );
          })}

          {/* Separator when extra cards present */}
          {hasExtra && handCards.length > 0 && (
            <div className="w-px self-stretch my-2 bg-[rgba(174,118,255,0.3)]" />
          )}

          {/* Extra playable cards from other zones */}
          {extraCards.map((extra) => {
            const cardId = String(extra.id);
            const card = { id: extra.id, name: extra.name };
            const plays = extra.actions;
            const isDragging = dragState?.objectId === extra.id;
            return (
              <GameCard
                key={`extra-${extra.id}`}
                card={card}
                variant="hand"
                isPlayable
                glowKind="extra"
                isHovered={hoveredObjectId === cardId}
                isDragging={isDragging}
                isNew
                isInspected={selectedObjectId != null && cardId === String(selectedObjectId)}
                onClick={plays.length <= 1 ? undefined : (e) => handleCardClick(e, card, plays)}
                onPointerDown={(e) => handlePointerDown(e, card, plays, "extra")}
                onMouseEnter={() => hoverCard(extra.id)}
                onMouseLeave={clearHover}
              />
            );
          })}

          {handCards.length === 0 && extraCards.length === 0 && (
            <div className="text-muted-foreground text-[17px] p-3 italic">Empty hand</div>
          )}
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
