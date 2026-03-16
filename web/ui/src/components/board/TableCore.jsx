import { useRef } from "react";
import { useGame } from "@/context/GameContext";
import OpponentZone from "./OpponentZone";
import MyZone from "./MyZone";
import DeckLoadingView from "./DeckLoadingView";
import DecisionPopupLayer from "@/components/overlays/DecisionPopupLayer";
export default function TableCore({
  selectedObjectId,
  onInspect,
  zoneViews,
  zoneActivityByPlayer = {},
  deckLoadingMode,
  onLoadDecks,
  onCancelDeckLoading,
  legalTargetPlayerIds = new Set(),
  legalTargetObjectIds = new Set(),
}) {
  const { state } = useGame();
  const tableRef = useRef(null);
  const players = state?.players || [];
  const perspective = state?.perspective;

  if (!players.length) {
    return <main className="table-gradient table-shell rounded-none min-h-0" />;
  }

  if (deckLoadingMode) {
    return <DeckLoadingView onLoad={onLoadDecks} onCancel={onCancelDeckLoading} />;
  }

  const me = players.find((p) => p.id === perspective) || players[0];
  const meIndex = players.findIndex((p) => p.id === me.id);
  const ordered = meIndex >= 0 ? [...players.slice(meIndex), ...players.slice(0, meIndex)] : players;
  const opponents = ordered.filter((p) => p.id !== me.id);
  const decision = state?.decision || null;
  const expandedActionBar = Boolean(
    decision
    && decision.kind !== "priority"
    && decision.kind !== "attackers"
    && decision.kind !== "blockers"
  );
  const actionBarHeight = expandedActionBar ? 124 : 62;

  return (
    <main
      ref={tableRef}
      className="table-gradient table-shell relative rounded-none grid gap-0 p-0 min-h-0 h-full overflow-visible"
      data-drop-zone
      style={{ gridTemplateRows: `minmax(0,1fr) ${actionBarHeight}px minmax(0,1fr)` }}
    >
      <OpponentZone
        opponents={opponents}
        selectedObjectId={selectedObjectId}
        onInspect={onInspect}
        zoneViews={zoneViews}
        zoneActivityByPlayer={zoneActivityByPlayer}
        legalTargetPlayerIds={legalTargetPlayerIds}
        legalTargetObjectIds={legalTargetObjectIds}
      />
      <div className="relative z-20 flex items-center">
        <div
          className="table-action-bar relative h-full w-full rounded-none border border-[#2b3f57]/65 bg-[linear-gradient(90deg,rgba(7,15,23,0.92),rgba(14,28,44,0.86),rgba(7,15,23,0.92))] shadow-[inset_0_1px_0_rgba(170,208,245,0.12),0_8px_18px_rgba(0,0,0,0.32)]"
          data-expanded={expandedActionBar ? "true" : "false"}
        >
          <DecisionPopupLayer priorityInline selectedObjectId={selectedObjectId} />
        </div>
      </div>
      <MyZone
        player={me}
        selectedObjectId={selectedObjectId}
        onInspect={onInspect}
        zoneViews={zoneViews}
        zoneActivity={zoneActivityByPlayer[String(me?.id ?? me?.index ?? "")] || {}}
        legalTargetPlayerIds={legalTargetPlayerIds}
        legalTargetObjectIds={legalTargetObjectIds}
      />
    </main>
  );
}
