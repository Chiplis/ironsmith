import { useMemo, useRef } from "react";
import { useGame } from "@/context/GameContext";
import OpponentZone from "./OpponentZone";
import MyZone from "./MyZone";
import DeckLoadingView from "./DeckLoadingView";
import DecisionPopupLayer from "@/components/overlays/DecisionPopupLayer";
import StackTimelineRail from "@/components/right-rail/StackTimelineRail";

export default function TableCore({
  selectedObjectId,
  onInspect,
  onExpandInspector,
  zoneViews,
  deckLoadingMode,
  onLoadDecks,
  onCancelDeckLoading,
}) {
  const { state } = useGame();
  const tableRef = useRef(null);
  if (!state?.players?.length) return <main className="table-gradient rail-gradient rounded min-h-0" />;

  if (deckLoadingMode) {
    return <DeckLoadingView onLoad={onLoadDecks} onCancel={onCancelDeckLoading} />;
  }

  const players = state.players;
  const perspective = state.perspective;
  const me = players.find((p) => p.id === perspective) || players[0];
  const meIndex = players.findIndex((p) => p.id === me.id);
  const ordered = meIndex >= 0 ? [...players.slice(meIndex), ...players.slice(0, meIndex)] : players;
  const opponents = ordered.filter((p) => p.id !== me.id);
  const legalTargetPlayerIds = useMemo(() => {
    const ids = new Set();
    const decision = state?.decision;
    if (!decision || decision.kind !== "targets") return ids;
    for (const req of decision.requirements || []) {
      for (const target of req.legal_targets || []) {
        if (target.kind === "player" && target.player != null) {
          ids.add(Number(target.player));
        }
      }
    }
    return ids;
  }, [state?.decision]);
  const legalTargetObjectIds = useMemo(() => {
    const ids = new Set();
    const decision = state?.decision;
    if (!decision || decision.kind !== "targets") return ids;
    for (const req of decision.requirements || []) {
      for (const target of req.legal_targets || []) {
        if (target.kind === "object" && target.object != null) {
          ids.add(Number(target.object));
        }
      }
    }
    return ids;
  }, [state?.decision]);

  return (
    <main
      ref={tableRef}
      className="table-gradient relative rounded grid gap-1.5 p-1.5 min-h-0 h-full overflow-hidden"
      data-drop-zone
      style={{ gridTemplateRows: "minmax(0,1.7fr) 62px minmax(0,1fr)" }}
    >
      <OpponentZone
        opponents={opponents}
        selectedObjectId={selectedObjectId}
        onInspect={onInspect}
        onExpandInspector={onExpandInspector}
        zoneViews={zoneViews}
        legalTargetPlayerIds={legalTargetPlayerIds}
        legalTargetObjectIds={legalTargetObjectIds}
      />
      <div className="relative z-20 flex items-center">
        <div className="relative h-full w-full rounded border border-[#2b3f57]/65 bg-[linear-gradient(90deg,rgba(7,15,23,0.92),rgba(14,28,44,0.86),rgba(7,15,23,0.92))] shadow-[inset_0_1px_0_rgba(170,208,245,0.12),0_8px_18px_rgba(0,0,0,0.32)]">
          <DecisionPopupLayer priorityInline selectedObjectId={selectedObjectId} />
        </div>
      </div>
      <MyZone
        player={me}
        selectedObjectId={selectedObjectId}
        onInspect={onInspect}
        onExpandInspector={onExpandInspector}
        zoneViews={zoneViews}
        legalTargetPlayerIds={legalTargetPlayerIds}
        legalTargetObjectIds={legalTargetObjectIds}
      />
      <StackTimelineRail
        floating
        anchorRef={tableRef}
        selectedObjectId={selectedObjectId}
        onInspectObject={onInspect}
      />
    </main>
  );
}
