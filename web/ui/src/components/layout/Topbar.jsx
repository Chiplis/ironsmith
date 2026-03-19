import { useGame } from "@/context/GameContext";
import useViewportLayout from "@/hooks/useViewportLayout";
import { formatPhase, formatStep } from "@/lib/constants";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import PhaseTrack from "@/components/board/PhaseTrack";
import { ChevronLeft, ChevronRight, Github, Plus } from "lucide-react";
import AddCardSheet from "./AddCardSheet";
import TopbarMenuSheet from "./TopbarMenuSheet";

const pill = "stone-pill text-[13px] uppercase cursor-pointer hover:brightness-110 transition-all select-none";

function dispatchPlayerTargetChoice(player, legalTargetPlayerIds) {
  const directId = Number(player?.id);
  const fallbackId = Number(player?.index);
  const targetPlayer = legalTargetPlayerIds.has(directId) ? directId : fallbackId;
  if (!Number.isFinite(targetPlayer)) return;

  window.dispatchEvent(
    new CustomEvent("ironsmith:target-choice", {
      detail: { target: { kind: "player", player: targetPlayer } },
    })
  );
}

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
  mobileOpponentIndex = 0,
  setMobileOpponentIndex,
  mobileOverlay = false,
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
  const me = players.find((player) => player.id === state?.perspective) || players[0];
  const meIndex = players.findIndex((player) => player.id === me?.id);
  const orderedPlayers = meIndex >= 0
    ? [...players.slice(meIndex), ...players.slice(0, meIndex)]
    : players;
  const opponents = orderedPlayers.filter((player) => player.id !== me?.id);
  const hasMobileOpponent = nonDesktopViewport && opponents.length > 0;
  const resolvedOpponentIndex = opponents.length > 0
    ? Math.min(mobileOpponentIndex, opponents.length - 1)
    : 0;
  const activeMobileOpponent = hasMobileOpponent
    ? opponents[resolvedOpponentIndex] || opponents[0]
    : null;
  const cycleMobileOpponent = (direction) => {
    if (!setMobileOpponentIndex || opponents.length <= 1) return;
    setMobileOpponentIndex((currentIndex) => {
      const nextIndex = Number(currentIndex || 0) + direction;
      if (nextIndex < 0) return opponents.length - 1;
      if (nextIndex >= opponents.length) return 0;
      return nextIndex;
    });
  };
  const phaseSummary = `${formatPhase(state?.phase)}${state?.step ? ` • ${formatStep(state?.step)}` : ""}`;
  const compactPhaseLabel = formatStep(state?.step) || formatPhase(state?.phase) || "Phase";
  const legalTargetPlayerIds = new Set();
  if (state?.decision?.kind === "targets") {
    for (const req of state.decision.requirements || []) {
      for (const target of req.legal_targets || []) {
        if (target.kind === "player" && target.player != null) {
          legalTargetPlayerIds.add(Number(target.player));
        }
      }
    }
  }
  const canPickTargets = state?.decision?.kind === "targets"
    && state?.decision?.player === state?.perspective;
  const activeMobileOpponentIsTargetable = activeMobileOpponent != null && (
    legalTargetPlayerIds.has(Number(activeMobileOpponent.id))
    || legalTargetPlayerIds.has(Number(activeMobileOpponent.index))
  );
  const handleMobileOpponentTarget = () => {
    if (!canPickTargets || !activeMobileOpponentIsTargetable || !activeMobileOpponent) return;
    dispatchPlayerTargetChoice(activeMobileOpponent, legalTargetPlayerIds);
  };

  if (mobileOverlay) {
    return (
      <header className="topbar-mobile-overlay">
        <div className="topbar-mobile-overlay-status">
          <div className="topbar-mobile-overlay-phase" aria-label={phaseSummary}>
            <span>{compactPhaseLabel}</span>
            <span>T{state?.turn_number ?? "-"}</span>
          </div>
          {activeMobileOpponent ? (
            <div
              className={`topbar-opponent-chip topbar-mobile-overlay-opponent${activeMobileOpponentIsTargetable && canPickTargets ? " is-targetable" : ""}`}
              aria-label={`Viewing opponent ${activeMobileOpponent.name}`}
            >
              {opponents.length > 1 ? (
                <button
                  type="button"
                  className="topbar-opponent-chip-nav"
                  onClick={() => cycleMobileOpponent(-1)}
                  aria-label="Show previous opponent"
                >
                  <ChevronLeft className="size-3.5" />
                </button>
              ) : null}
              <button
                type="button"
                className="topbar-opponent-chip-body topbar-opponent-chip-body--button"
                onClick={handleMobileOpponentTarget}
                disabled={!activeMobileOpponentIsTargetable || !canPickTargets}
                aria-label={`Opponent ${activeMobileOpponent.name}, life ${activeMobileOpponent.life}`}
              >
                <span
                  className="topbar-opponent-chip-name"
                  style={{ color: activeMobileOpponent.id === activePlayer?.id ? "#fff0ca" : undefined }}
                >
                  {activeMobileOpponent.name}
                </span>
                <span className="topbar-opponent-chip-life">{activeMobileOpponent.life}</span>
                <span className="topbar-opponent-chip-meta">
                  H {activeMobileOpponent.hand_size ?? 0} G {activeMobileOpponent.graveyard_size ?? 0} X {Array.isArray(activeMobileOpponent.exile_cards) ? activeMobileOpponent.exile_cards.length : 0}
                </span>
              </button>
              {opponents.length > 1 ? (
                <button
                  type="button"
                  className="topbar-opponent-chip-nav"
                  onClick={() => cycleMobileOpponent(1)}
                  aria-label="Show next opponent"
                >
                  <ChevronRight className="size-3.5" />
                </button>
              ) : null}
            </div>
          ) : null}
        </div>
        <div className="topbar-mobile-overlay-actions">
          <AddCardSheet
            onAddCardFailure={onAddCardFailure}
            trigger={(
              <Button
                variant="secondary"
                size="icon-xs"
                className="stone-pill topbar-add-card-trigger rounded-none text-[#d8c8a7] hover:text-[#fff1cd]"
                aria-label="Add Card"
              >
                <Plus className="size-3.5" />
              </Button>
            )}
          />
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
      </header>
    );
  }

  return (
    <header className="table-toolbar table-toolbar--primary topbar-shell rounded-none px-3 py-2">
      <div className="topbar-side-cluster topbar-side-cluster--left min-w-0">
        <h1 className="toolbar-brand topbar-brand m-0 whitespace-nowrap font-bold">
          Ironsmith
        </h1>
        {nonDesktopViewport ? (
          <div className="topbar-mobile-status">
            <div className="topbar-phase-chip" aria-label={phaseSummary}>
              <span className="topbar-phase-chip-label">{compactPhaseLabel}</span>
              <span className="topbar-phase-chip-turn">T{state?.turn_number ?? "-"}</span>
            </div>
            {activeMobileOpponent ? (
              <div className="topbar-opponent-chip" aria-label={`Viewing opponent ${activeMobileOpponent.name}`}>
                {opponents.length > 1 ? (
                  <button
                    type="button"
                    className="topbar-opponent-chip-nav"
                    onClick={() => cycleMobileOpponent(-1)}
                    aria-label="Show previous opponent"
                  >
                    <ChevronLeft className="size-3.5" />
                  </button>
                ) : null}
                <div className="topbar-opponent-chip-body">
                  <span className="topbar-opponent-chip-name" style={{ color: activeMobileOpponent.id === activePlayer?.id ? "#fff0ca" : undefined }}>
                    {activeMobileOpponent.name}
                  </span>
                  <span className="topbar-opponent-chip-life">{activeMobileOpponent.life}</span>
                  <span className="topbar-opponent-chip-meta">
                    H {activeMobileOpponent.hand_size ?? 0} G {activeMobileOpponent.graveyard_size ?? 0} X {Array.isArray(activeMobileOpponent.exile_cards) ? activeMobileOpponent.exile_cards.length : 0}
                  </span>
                </div>
                {opponents.length > 1 ? (
                  <button
                    type="button"
                    className="topbar-opponent-chip-nav"
                    onClick={() => cycleMobileOpponent(1)}
                    aria-label="Show next opponent"
                  >
                    <ChevronRight className="size-3.5" />
                  </button>
                ) : null}
              </div>
            ) : null}
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
