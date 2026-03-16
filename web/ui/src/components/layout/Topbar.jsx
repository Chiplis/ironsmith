import { useGame } from "@/context/GameContext";
import { formatPhase, formatStep } from "@/lib/constants";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import PhaseTrack from "@/components/board/PhaseTrack";
import TopbarMenuSheet from "./TopbarMenuSheet";

const pill = "stone-pill text-[13px] uppercase cursor-pointer hover:brightness-110 transition-all select-none";

export default function Topbar({
  playerNames,
  setPlayerNames,
  startingLife,
  setStartingLife,
  onReset,
  onChangePerspective,
  onRefresh,
  onToggleLog,
  onEnterDeckLoading,
  onOpenLobby,
  deckLoadingMode,
}) {
  const {
    inspectorDebug,
    setInspectorDebug,
    multiplayer,
    state,
  } = useGame();

  const players = state?.players || [];
  const activePlayer = players.find((player) => player.id === state?.active_player) || null;
  const priorityPlayer = players.find((player) => player.id === state?.priority_player) || null;
  const phaseSummary = `${formatPhase(state?.phase)}${state?.step ? ` • ${formatStep(state?.step)}` : ""}`;

  return (
    <header className="table-toolbar table-toolbar--primary topbar-shell rounded-none px-3 py-2">
      <div className="topbar-side-cluster topbar-side-cluster--left min-w-0">
        <h1 className="toolbar-brand topbar-brand m-0 whitespace-nowrap font-bold">
          Ironsmith
        </h1>
        <div className="topbar-phase-caption topbar-phase-caption--inline">
          <span>{phaseSummary}</span>
          <span className="topbar-phase-caption-dot" aria-hidden="true">•</span>
          <span>Turn {state?.turn_number ?? "-"}</span>
          {activePlayer ? (
            <>
              <span className="topbar-phase-caption-dot" aria-hidden="true">•</span>
              <span>Active {activePlayer.name}</span>
            </>
          ) : null}
          {priorityPlayer ? (
            <>
              <span className="topbar-phase-caption-dot" aria-hidden="true">•</span>
              <span>Priority {priorityPlayer.name}</span>
            </>
          ) : null}
        </div>
      </div>

      <div className="topbar-center-lane min-w-0">
        <div className="topbar-phase-shell">
          <PhaseTrack />
        </div>
      </div>

      <div className="topbar-side-cluster topbar-side-cluster--right">
        <div className="topbar-minor-controls topbar-minor-controls--utility">
          <label className="toolbar-checkbox toolbar-debug-toggle topbar-toggle flex items-center gap-1.5 whitespace-nowrap cursor-pointer uppercase">
            <Checkbox
              checked={inspectorDebug}
              onCheckedChange={(value) => setInspectorDebug(!!value)}
              className="h-3.5 w-3.5"
            />
            Debug
          </label>
          <Badge variant="secondary" className={pill} onClick={onToggleLog}>Log</Badge>
          <TopbarMenuSheet
            playerNames={playerNames}
            setPlayerNames={setPlayerNames}
            startingLife={startingLife}
            setStartingLife={setStartingLife}
            onReset={onReset}
            onChangePerspective={onChangePerspective}
            onRefresh={onRefresh}
            onEnterDeckLoading={onEnterDeckLoading}
            onOpenLobby={onOpenLobby}
            deckLoadingMode={deckLoadingMode}
          />
        </div>
      </div>
    </header>
  );
}
