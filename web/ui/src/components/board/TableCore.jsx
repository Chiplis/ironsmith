import { useGame } from "@/context/GameContext";
import OpponentZone from "./OpponentZone";
import MyZone from "./MyZone";
import PhaseTrack from "./PhaseTrack";
import HandZone from "./HandZone";
import DeckLoadingView from "./DeckLoadingView";

export default function TableCore({ selectedObjectId, onInspect, zoneView, deckLoadingMode, onLoadDecks, onCancelDeckLoading }) {
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

  return (
    <main className="table-gradient rounded grid gap-1.5 p-1.5 min-h-0 overflow-hidden" style={{ gridTemplateRows: "1.7fr 1fr auto auto" }}>
      <OpponentZone opponents={opponents} selectedObjectId={selectedObjectId} onInspect={onInspect} zoneView={zoneView} />
      <MyZone player={me} selectedObjectId={selectedObjectId} onInspect={onInspect} zoneView={zoneView} />
      <PhaseTrack />
      <HandZone player={me} selectedObjectId={selectedObjectId} onInspect={onInspect} />
    </main>
  );
}
