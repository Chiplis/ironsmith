import { useGame } from "@/context/GameContext";
import { useCombatArrows } from "@/context/useCombatArrows";
import useViewportLayout from "@/hooks/useViewportLayout";
import { formatPhase, formatStep } from "@/lib/constants";
import { Button } from "@/components/ui/button";
import PhaseTrack from "@/components/board/PhaseTrack";
import { Bug, ChevronLeft, ChevronRight, Clock3, Github, ScrollText, WifiOff } from "lucide-react";
import TopbarMenuSheet from "./TopbarMenuSheet";
import { playerDisplayName, samePlayerId } from "@/lib/player-display";

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

function formatTimerRemaining(ms) {
  const totalSeconds = Math.max(0, Math.ceil(Number(ms || 0) / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

function disconnectCountdownLabel(warnings) {
  const entries = Array.isArray(warnings) ? warnings : [];
  if (entries.length === 0) return "";
  const remainingMs = Math.min(...entries.map((warning) => Number(warning.remainingMs || 0)));
  return formatTimerRemaining(remainingMs);
}

export default function Topbar({
  playerNames,
  setPlayerNames,
  startingLife,
  setStartingLife,
  onReset,
  onRefresh,
  onToggleLog,
  onEnterDeckLoading,
  onOpenPuzzleSetup,
  onOpenLobby,
  deckLoadingMode,
  puzzleSetupMode = false,
  onAddCardNotice,
  mobileOpponentIndex = 0,
  setMobileOpponentIndex,
  mobileOverlay = false,
  middleDocked = false,
}) {
  const {
    inspectorDebug,
    multiplayer,
    setInspectorDebug,
    state,
  } = useGame();
  const { combatMode, combatModeRef } = useCombatArrows();
  const { nonDesktopViewport, tabletCompactViewport, smallDesktopViewport, largeDesktopViewport } = useViewportLayout();

  const players = state?.players || [];
  const activePlayer = players.find((player) => samePlayerId(player.id, state?.active_player)) || null;
  const me = players.find((player) => samePlayerId(player.id, state?.perspective)) || players[0];
  const meIndex = players.findIndex((player) => samePlayerId(player.id, me?.id));
  const orderedPlayers = meIndex >= 0
    ? [...players.slice(meIndex), ...players.slice(0, meIndex)]
    : players;
  const opponents = orderedPlayers.filter((player) => !samePlayerId(player.id, me?.id));
  const hasMobileOpponent = nonDesktopViewport && opponents.length > 0;
  const resolvedOpponentIndex = opponents.length > 0
    ? Math.min(mobileOpponentIndex, opponents.length - 1)
    : 0;
  const activeMobileOpponent = hasMobileOpponent
    ? opponents[resolvedOpponentIndex] || opponents[0]
    : null;
  const previousMobileOpponent = opponents.length > 1
    ? opponents[(resolvedOpponentIndex - 1 + opponents.length) % opponents.length]
    : null;
  const nextMobileOpponent = opponents.length > 1
    ? opponents[(resolvedOpponentIndex + 1) % opponents.length]
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
  const connectionWarnings = multiplayer?.connectionWarnings || [];
  const matchClock = multiplayer?.matchClock || multiplayer?.actionTimer || null;
  const matchClockEntries = Array.isArray(matchClock?.remainingMsByPlayer)
    ? matchClock.remainingMsByPlayer.map((remainingMs, index) => ({
        player: players.find((candidate) =>
          Number(candidate.id) === Number(index) || Number(candidate.index) === Number(index)
        ) || { index },
        index,
        remainingMs,
        active: Number(matchClock.activePlayerIndex ?? matchClock.currentPlayerIndex) === Number(index),
        expired: Number(remainingMs || 0) <= 0,
      }))
    : [];
  const showMatchClock = Boolean(
    multiplayer?.matchStarted
    && matchClock?.enabled
    && matchClockEntries.length > 0
  );
  const offlinePlayers = connectionWarnings.filter((warning) => !warning.local);
  const connectionWarningLabel = offlinePlayers.length > 0
    ? offlinePlayers.map((warning) => {
        const display = playerDisplayName(players, warning.playerIndex ?? warning.index ?? warning.id);
        return display === "?" ? warning.name : display;
      }).join(", ")
    : "";
  const disconnectCountdown = disconnectCountdownLabel(offlinePlayers);
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
    && samePlayerId(state?.decision?.player, state?.perspective);
  const activeCombatAttackerId = combatMode?.mode === "attackers"
    ? Number(combatMode?.selectedAttacker ?? NaN)
    : NaN;
  const activeCombatTargetPlayers = Number.isFinite(activeCombatAttackerId)
    ? combatMode?.validTargetPlayersByAttacker?.[activeCombatAttackerId]
    : null;
  const activeMobileOpponentCombatTargetable = (
    Number.isFinite(activeCombatAttackerId)
    && (
      !!activeCombatTargetPlayers?.has?.(Number(activeMobileOpponent?.id ?? NaN))
      || !!activeCombatTargetPlayers?.has?.(Number(activeMobileOpponent?.index ?? NaN))
    )
  );
  const activeMobileOpponentIsTargetable = activeMobileOpponent != null && (
    legalTargetPlayerIds.has(Number(activeMobileOpponent.id))
    || legalTargetPlayerIds.has(Number(activeMobileOpponent.index))
  );
  const activeMobileOpponentButtonEnabled = (
    (activeMobileOpponentIsTargetable && canPickTargets)
    || activeMobileOpponentCombatTargetable
  );
  const handleMobileOpponentTarget = () => {
    if (!canPickTargets || !activeMobileOpponentIsTargetable || !activeMobileOpponent) return;
    dispatchPlayerTargetChoice(activeMobileOpponent, legalTargetPlayerIds);
  };
  const handleCombatOpponentTarget = (event) => {
    const currentCombatMode = combatModeRef.current;
    if (!activeMobileOpponent || !currentCombatMode?.onTargetAreaClick || currentCombatMode.selectedAttacker == null) {
      return false;
    }
    const validTargets = currentCombatMode.validTargetPlayersByAttacker?.[Number(currentCombatMode.selectedAttacker)];
    const directId = Number(activeMobileOpponent.id);
    const fallbackId = Number(activeMobileOpponent.index);
    const playerId = validTargets?.has?.(directId) ? directId : fallbackId;
    if (!validTargets?.has?.(playerId)) {
      return false;
    }
    event.preventDefault();
    event.stopPropagation();
    currentCombatMode.onTargetAreaClick(playerId, null);
    return true;
  };

  if (mobileOverlay) {
    // MTGA-aligned mobile UI moves phase + opponent chrome into MobileBattleScene.
    // The Topbar's mobile branch shrinks to a single floating cog at the top-right.
    return (
      <header className="topbar-mobile-overlay topbar-mobile-overlay--cog-only" aria-label="Mobile menu">
        <TopbarMenuSheet
          playerNames={playerNames}
          setPlayerNames={setPlayerNames}
          startingLife={startingLife}
          setStartingLife={setStartingLife}
          onReset={onReset}
          onRefresh={onRefresh}
          onToggleLog={onToggleLog}
          onEnterDeckLoading={onEnterDeckLoading}
          onOpenPuzzleSetup={onOpenPuzzleSetup}
          onOpenLobby={onOpenLobby}
          deckLoadingMode={deckLoadingMode}
          puzzleSetupMode={puzzleSetupMode}
          onAddCardNotice={onAddCardNotice}
          triggerIcon="settings"
          showQuickActions
        />
      </header>
    );
  }

  const showCompactPhase = nonDesktopViewport || tabletCompactViewport;
  const showCenterLane = !nonDesktopViewport && !tabletCompactViewport;
  const showInlineControls = !nonDesktopViewport && !tabletCompactViewport;
  const viewportTier = largeDesktopViewport ? "large" : smallDesktopViewport ? "small" : tabletCompactViewport ? "tablet" : nonDesktopViewport ? "phone" : "desktop";
  const utilityControls = (
    <div className="topbar-minor-controls topbar-minor-controls--utility">
      {showInlineControls ? (
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
            title="GitHub"
          >
            <Github className="size-3.5" />
          </a>
        </Button>
      ) : null}
      <TopbarMenuSheet
        playerNames={playerNames}
        setPlayerNames={setPlayerNames}
        startingLife={startingLife}
        setStartingLife={setStartingLife}
        onReset={onReset}
        onRefresh={onRefresh}
        onToggleLog={onToggleLog}
        onEnterDeckLoading={onEnterDeckLoading}
        onOpenPuzzleSetup={onOpenPuzzleSetup}
        onOpenLobby={onOpenLobby}
        deckLoadingMode={deckLoadingMode}
        puzzleSetupMode={puzzleSetupMode}
        onAddCardNotice={onAddCardNotice}
        triggerIcon={showInlineControls ? "settings" : "menu"}
        showQuickActions={!showInlineControls}
      />
      {showInlineControls ? (
        <>
          <Button
            variant="secondary"
            size="icon-xs"
            className="stone-pill topbar-log-trigger rounded-none text-[#d8c8a7] hover:text-[#fff1cd]"
            onClick={onToggleLog}
            aria-label="Toggle activity log"
            title="Log"
          >
            <ScrollText className="size-3.5" />
          </Button>
          <Button
            variant="secondary"
            size="icon-xs"
            className={`stone-pill topbar-debug-trigger rounded-none text-[#d8c8a7] hover:text-[#fff1cd]${inspectorDebug ? " is-active" : ""}`}
            onClick={() => setInspectorDebug(!inspectorDebug)}
            aria-label={inspectorDebug ? "Disable debug overlay" : "Enable debug overlay"}
            aria-pressed={inspectorDebug}
            title={inspectorDebug ? "Debug enabled" : "Debug"}
          >
            <Bug className="size-3.5" />
          </Button>
        </>
      ) : null}
    </div>
  );

  return (
    <header
      className={`table-toolbar table-toolbar--primary topbar-shell rounded-none px-3 py-2${middleDocked ? " topbar-shell--middle-docked" : ""}`}
      data-viewport-tier={viewportTier}
    >
      <div className="topbar-side-cluster topbar-side-cluster--left min-w-0">
        <h1 className="toolbar-brand topbar-brand m-0 whitespace-nowrap font-bold">
          Ironsmith
        </h1>
        {multiplayer?.matchStarted && offlinePlayers.length > 0 ? (
          <button
            type="button"
            className="stone-pill inline-flex min-h-8 max-w-[240px] items-center gap-2 rounded-none border border-[#7d302f] bg-[#2b1114]/90 px-2.5 py-1 text-[11px] font-semibold uppercase tracking-[0.14em] text-[#ffb8c0]"
            onClick={onOpenLobby}
            title={`Disconnected: ${connectionWarningLabel}. Timeout policy in ${disconnectCountdown}.`}
            aria-label={`Disconnected players: ${connectionWarningLabel}`}
          >
            <WifiOff className="size-3.5 shrink-0" />
            <span className="truncate">
              {offlinePlayers.length === 1
                ? `${connectionWarningLabel} ${disconnectCountdown}`
                : `${offlinePlayers.length} offline ${disconnectCountdown}`}
            </span>
          </button>
        ) : null}
        {showMatchClock ? (
          <div
            className="stone-pill inline-flex min-h-8 max-w-[520px] items-center gap-2 overflow-hidden rounded-none border border-[#5f4a22] bg-[#231c0e]/90 px-2.5 py-1 text-[11px] font-semibold uppercase tracking-[0.14em] text-[#ffd98a]"
            title="Match clocks"
            aria-label="Per-player match clocks"
          >
            <Clock3 className="size-3.5 shrink-0" />
            <span className="flex min-w-0 items-center gap-2 overflow-hidden">
              {matchClockEntries.map((entry) => (
                <span
                  key={entry.index}
                  className={`whitespace-nowrap ${
                    entry.expired
                      ? "text-[#ffb8c0]"
                      : entry.active
                        ? "text-[#fff1cd]"
                        : "text-[#c9b98f]"
                  }`}
                >
                  {playerDisplayName(players, entry.player) || `P${entry.index + 1}`} {formatTimerRemaining(entry.remainingMs)}
                </span>
              ))}
            </span>
          </div>
        ) : null}
        {showInlineControls ? utilityControls : null}
        {showCenterLane ? (
          <div className="topbar-phase-shell">
            <PhaseTrack compact={middleDocked} />
          </div>
        ) : null}
        {showCompactPhase ? (
          <div className="topbar-mobile-status">
            <div className="topbar-phase-chip" aria-label={phaseSummary}>
              <span className="topbar-phase-chip-label">{compactPhaseLabel}</span>
              <span className="topbar-phase-chip-turn">T{state?.turn_number ?? "-"}</span>
            </div>
            {nonDesktopViewport && activeMobileOpponent ? (
              <div
                className={`topbar-opponent-chip${activeMobileOpponentButtonEnabled ? " is-targetable" : ""}`}
                aria-label={`Viewing opponent ${playerDisplayName(players, activeMobileOpponent)}`}
              >
                {opponents.length > 1 ? (
                  <button
                    type="button"
                    className="topbar-opponent-chip-nav"
                    data-player-nav-target={previousMobileOpponent?.index ?? previousMobileOpponent?.id}
                    data-player-nav-target-name={previousMobileOpponent?.id ?? previousMobileOpponent?.index}
                    onClick={() => cycleMobileOpponent(-1)}
                    aria-label="Show previous opponent"
                  >
                    <ChevronLeft className="size-3.5" />
                  </button>
                ) : null}
                <button
                  type="button"
                  className="topbar-opponent-chip-body topbar-opponent-chip-body--button"
                  data-player-target={activeMobileOpponent.index ?? activeMobileOpponent.id}
                  data-player-target-name={activeMobileOpponent.id ?? activeMobileOpponent.index}
                  onClick={(event) => {
                    if (handleCombatOpponentTarget(event)) return;
                    handleMobileOpponentTarget();
                  }}
                  disabled={!activeMobileOpponentButtonEnabled}
                  aria-label={`Opponent ${playerDisplayName(players, activeMobileOpponent)}, life ${activeMobileOpponent.life}`}
                >
                  <span className="topbar-opponent-chip-name" style={{ color: activeMobileOpponent.id === activePlayer?.id ? "#fff0ca" : undefined }}>
                    {playerDisplayName(players, activeMobileOpponent)}
                  </span>
                  <span className="topbar-opponent-chip-life">{activeMobileOpponent.life}</span>
                  <span className="topbar-opponent-chip-meta">
                    H {activeMobileOpponent.hand_size ?? 0} G {activeMobileOpponent.graveyard_size ?? 0} D {activeMobileOpponent.library_size ?? 0}
                  </span>
                </button>
                {opponents.length > 1 ? (
                  <button
                    type="button"
                    className="topbar-opponent-chip-nav"
                    data-player-nav-target={nextMobileOpponent?.index ?? nextMobileOpponent?.id}
                    data-player-nav-target-name={nextMobileOpponent?.id ?? nextMobileOpponent?.index}
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
      </div>

      {!showInlineControls ? (
        <div className="topbar-side-cluster topbar-side-cluster--right">
          {utilityControls}
        </div>
      ) : null}
    </header>
  );
}
