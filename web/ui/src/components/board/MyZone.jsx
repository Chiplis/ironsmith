import { useState } from "react";
import { useGame } from "@/context/GameContext";
import BattlefieldRow from "./BattlefieldRow";
import ActionPopover from "@/components/overlays/ActionPopover";

function getZoneCards(player, zoneView) {
  switch (zoneView) {
    case "hand": return player.hand_cards || [];
    case "graveyard": return player.graveyard_cards || [];
    case "exile": return player.exile_cards || [];
    default: return player.battlefield || [];
  }
}

export default function MyZone({ player, selectedObjectId, onInspect, zoneView = "battlefield" }) {
  const { state, dispatch } = useGame();
  const [popover, setPopover] = useState(null);

  const cards = getZoneCards(player, zoneView);
  const zoneName = zoneView === "battlefield" ? "" : ` — ${zoneView.charAt(0).toUpperCase() + zoneView.slice(1)}`;

  // Build activatable map from decision actions (activate_ability + activate_mana_ability)
  const activatableMap = new Map();
  if (state?.decision?.kind === "priority" && state.decision.actions) {
    for (const action of state.decision.actions) {
      if (
        (action.kind === "activate_ability" || action.kind === "activate_mana_ability") &&
        action.object_id != null
      ) {
        const objId = Number(action.object_id);
        if (!activatableMap.has(objId)) activatableMap.set(objId, []);
        activatableMap.get(objId).push(action);
      }
    }
  }

  const handleCardClick = (e, card) => {
    // Always inspect
    onInspect?.(card.id);

    // Show popover if activatable
    const actions = activatableMap.get(Number(card.id)) || [];
    if (actions.length > 0) {
      const rect = e.currentTarget.getBoundingClientRect();
      setPopover({ anchorRect: rect, actions, objectId: card.id });
    }
  };

  const handlePopoverAction = (action) => {
    setPopover(null);
    dispatch(
      { type: "priority_action", action_index: action.index },
      action.label
    );
  };

  return (
    <section className="board-zone-bg p-2 min-h-[120px] overflow-hidden grid gap-1" style={{ gridTemplateRows: "auto minmax(0,1fr)", alignContent: "stretch" }}>
      <div className="flex items-center gap-2">
        <span className="text-[23px] font-bold leading-none text-[#f5d08b] tabular-nums">{player.life}</span>
        <span className="text-[16px] text-[#a4bdd7] uppercase tracking-wider font-bold">
          {player.name}
          {zoneName && <span className="text-muted-foreground">{zoneName}</span>}
        </span>
        <span className="text-[14px] text-muted-foreground ml-auto">{cards.length} cards</span>
      </div>
      <BattlefieldRow
        cards={cards}
        selectedObjectId={selectedObjectId}
        onCardClick={handleCardClick}
        activatableMap={activatableMap}
      />

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
