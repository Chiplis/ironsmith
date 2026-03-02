import { useState, useRef, useEffect, useCallback } from "react";
import { useGame } from "@/context/GameContext";
import GameCard from "@/components/cards/GameCard";
import ActionPopover from "@/components/overlays/ActionPopover";

export default function HandZone({ player, onInspect }) {
  const { state, dispatch } = useGame();
  const [popover, setPopover] = useState(null); // { anchorRect, actions, objectId }

  if (!player) return null;

  const isMe = player.id === state?.perspective;

  // Build playable map from decision actions
  const playableMap = new Map();
  if (isMe && state?.decision?.kind === "priority" && state.decision.actions) {
    for (const action of state.decision.actions) {
      if (
        (action.kind === "cast_spell" || action.kind === "play_land") &&
        action.object_id != null &&
        action.to_zone
      ) {
        const objId = Number(action.object_id);
        if (!playableMap.has(objId)) playableMap.set(objId, []);
        playableMap.get(objId).push(action);
      }
    }
  }

  const handleCardClick = (e, card) => {
    const plays = playableMap.get(Number(card.id)) || [];
    if (plays.length > 0) {
      const rect = e.currentTarget.getBoundingClientRect();
      setPopover({ anchorRect: rect, actions: plays, objectId: card.id });
    } else {
      onInspect?.(card.id);
    }
  };

  const handlePopoverAction = (action) => {
    setPopover(null);
    dispatch(
      { type: "priority_action", action_index: action.index },
      action.label
    );
  };

  if (player.can_view_hand) {
    const handCards = player.hand_cards || [];
    return (
      <section className="border border-[#41566f] bg-[#10161f] p-2 grid gap-1.5">
        <h3 className="m-0 text-[#a4bdd7] uppercase tracking-wider text-[12px] font-semibold">
          Your Hand
        </h3>
        <div className="flex gap-1.5 flex-nowrap overflow-x-auto pb-0.5 min-h-[176px] items-end">
          {handCards.map((card) => {
            const plays = playableMap.get(Number(card.id)) || [];
            const isPlayable = plays.length > 0;
            return (
              <GameCard
                key={card.id}
                card={card}
                variant="hand"
                isPlayable={isPlayable}
                onClick={(e) => handleCardClick(e, card)}
              />
            );
          })}
          {handCards.length === 0 && (
            <div className="text-muted-foreground text-[13px] p-3 italic">Empty hand</div>
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
    <section className="border border-[#41566f] bg-[#10161f] p-2 grid gap-1.5">
      <h3 className="m-0 text-[#a4bdd7] uppercase tracking-wider text-[12px] font-semibold">
        Hand ({player.hand_size})
      </h3>
      <div className="flex gap-1.5 flex-nowrap overflow-x-auto pb-0.5 min-h-[176px] items-end">
        {backs > 0
          ? Array.from({ length: backs }, (_, i) => (
              <div key={i} className="game-card w-[92px] min-w-[92px] min-h-[126px] p-1 text-[11px] grid content-end">
                <span className="card-label text-muted-foreground">Card</span>
              </div>
            ))
          : <div className="text-muted-foreground text-[13px] p-3 italic">Empty hand</div>
        }
      </div>
    </section>
  );
}
