import { useMemo } from "react";
import { useGame } from "@/context/GameContext";
import OpponentZone from "./OpponentZone";
import MyZone from "./MyZone";
import ZoneViewer from "./ZoneViewer";
import DeckLoadingView from "./DeckLoadingView";
import PhaseTrack from "./PhaseTrack";

export default function TableCore({
  selectedObjectId,
  onInspect,
  zoneViews,
  setZoneViews,
  deckLoadingMode,
  onLoadDecks,
  onCancelDeckLoading,
}) {
  const { state } = useGame();
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
      className="table-gradient rounded grid gap-1.5 p-1.5 min-h-0 h-full overflow-hidden"
      data-drop-zone
      style={{ gridTemplateRows: "minmax(0,1.7fr) auto minmax(0,1fr) auto" }}
    >
      <OpponentZone
        opponents={opponents}
        selectedObjectId={selectedObjectId}
        onInspect={onInspect}
        zoneViews={zoneViews}
        legalTargetPlayerIds={legalTargetPlayerIds}
        legalTargetObjectIds={legalTargetObjectIds}
      />
      <PhaseTrack />
      <MyZone
        player={me}
        selectedObjectId={selectedObjectId}
        onInspect={onInspect}
        zoneViews={zoneViews}
        legalTargetPlayerIds={legalTargetPlayerIds}
        legalTargetObjectIds={legalTargetObjectIds}
      />
      <ZoneViewer zoneViews={zoneViews} setZoneViews={setZoneViews} />
    </main>
  );
}
