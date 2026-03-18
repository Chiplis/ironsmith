import { useGame } from "@/context/GameContext";
import useViewportLayout from "@/hooks/useViewportLayout";
import { formatPhase, formatStep } from "@/lib/constants";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import PhaseTrack from "@/components/board/PhaseTrack";
import { Github } from "lucide-react";
import AddCardSheet from "./AddCardSheet";
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
  onAddCardFailure,
}) {
  const {
    inspectorDebug,
    setInspectorDebug,
    state,
  } = useGame();
  const { nonDesktopViewport } = useViewportLayout();

  const players = state?.players || [];
  const activePlayer = players.find((player) => player.id === state?.active_player) || null;
  const priorityPlayer = players.find((player) => player.id === state?.priority_player) || null;
  const phaseSummary = `${formatPhase(state?.phase)}${state?.step ? ` • ${formatStep(state?.step)}` : ""}`;
  const compactPhaseLabel = formatStep(state?.step) || formatPhase(state?.phase) || "Phase";

  return (
    <header className="table-toolbar table-toolbar--primary topbar-shell rounded-none px-3 py-2">
      <div className="topbar-side-cluster topbar-side-cluster--left min-w-0">
        <h1 className="toolbar-brand topbar-brand m-0 whitespace-nowrap font-bold">
          Ironsmith
        </h1>
        {nonDesktopViewport ? (
          <div className="topbar-phase-chip" aria-label={phaseSummary}>
            <span className="topbar-phase-chip-label">{compactPhaseLabel}</span>
            <span className="topbar-phase-chip-turn">T{state?.turn_number ?? "-"}</span>
          </div>
        ) : null}
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
          <Button
            variant="secondary"
            size="icon-xs"
            className="stone-pill topbar-github-trigger rounded-none text-[#d8c8a7] hover:text-[#fff1cd]"
            asChild
          >
            <a
              href="https://github.com/Chiplis/ironsmith"
              target="_blank"
              rel="noopener noreferrer"
              aria-label="Open Ironsmith GitHub repository"
            >
              <Github className="size-3.5" />
            </a>
          </Button>
          <AddCardSheet
            onAddCardFailure={onAddCardFailure}
            trigger={(
              <Button
                variant="secondary"
                size="sm"
                className="stone-pill topbar-add-card-trigger rounded-none px-2.5 text-[#d8c8a7] hover:text-[#fff1cd]"
              >
                Add Card
              </Button>
            )}
          />
          <TopbarMenuSheet
            playerNames={playerNames}
            setPlayerNames={setPlayerNames}
            startingLife={startingLife}
            setStartingLife={setStartingLife}
            onReset={onReset}
            onChangePerspective={onChangePerspective}
            onRefresh={onRefresh}
            onToggleLog={onToggleLog}
            onEnterDeckLoading={onEnterDeckLoading}
            onOpenLobby={onOpenLobby}
            deckLoadingMode={deckLoadingMode}
          />
        </div>
      </div>
    </header>
  );
}
