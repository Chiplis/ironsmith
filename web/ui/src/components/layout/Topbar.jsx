import useUiText from "@/i18n/useUiText";
import { useGame, useMatchClock } from "@/context/GameContext";
import { useCombatArrows } from "@/context/useCombatArrows";
import useViewportLayout from "@/hooks/useViewportLayout";
import { formatPhase, formatStep } from "@/lib/constants";
import PhaseTrack from "@/components/board/PhaseTrack";
import DecisionPopupLayer from "@/components/overlays/DecisionPopupLayer";
import { ChevronLeft, ChevronRight, Clock3, WifiOff } from "lucide-react";
import TopbarMenuSheet from "./TopbarMenuSheet";
import { DEFAULT_PLAYER_ACCENT, getPlayerAccent } from "@/lib/player-colors";
import { playerDisplayName, samePlayerId } from "@/lib/player-display";
import { useI18n } from "@/i18n/I18nContext";

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
  onChangePerspective,
  utilityControls,
  tableToolsToggle,
}) {
  const ui = useUiText();
  const {
    multiplayer,
    playerAccentOverrides,
    state,
  } = useGame();
  const { t } = useI18n();
  const { combatMode, combatModeRef } = useCombatArrows();
  const { nonDesktopViewport, tabletCompactViewport, smallDesktopViewport, largeDesktopViewport } = useViewportLayout();

  const players = state?.players || [];
  const activePlayer = players.find((player) => samePlayerId(player.id, state?.active_player)) || null;
  const priorityPlayer = players.find((player) => samePlayerId(player.id, state?.priority_player)) || null;
  const decisionPlayer = state?.decision?.player != null
    ? players.find((player) => samePlayerId(player.id, state.decision.player)) || null
    : null;
  const decisionOwnerDiffersFromPriority = decisionPlayer
    && (!priorityPlayer || !samePlayerId(decisionPlayer.id, priorityPlayer.id));
  const me = players.find((player) => samePlayerId(player.id, state?.perspective)) || players[0];
  const perspectiveAccent = getPlayerAccent(
    players,
    me?.id,
    state?.perspective,
    playerAccentOverrides
  ) || DEFAULT_PLAYER_ACCENT;
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
  const connectionWarnings = multiplayer?.connectionWarnings || [];
  const matchClock = useMatchClock();
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
  const translatedPhaseSummary = `${formatPhase(state?.phase, t)}${state?.step ? ` - ${formatStep(state?.step, t)}` : ""}`;
  const translatedCompactPhaseLabel = formatStep(state?.step, t) || formatPhase(state?.phase, t) || t("game.advance.next");

  if (mobileOverlay) {
    // MTGA-aligned mobile UI moves phase + opponent chrome into MobileBattleScene.
    // The Topbar's mobile branch shrinks to a single floating cog at the top-right.
    return (
      <header className="topbar-mobile-overlay topbar-mobile-overlay--cog-only" aria-label={ui("Mobile menu")}>
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
  const viewportTier = largeDesktopViewport ? "large" : smallDesktopViewport ? "small" : tabletCompactViewport ? "tablet" : nonDesktopViewport ? "phone" : "desktop";


  return (
    <header
      className={`table-toolbar table-toolbar--primary topbar-shell rounded-none px-3 py-2${middleDocked ? " topbar-shell--middle-docked" : ""}`}
      data-viewport-tier={viewportTier}
    >
      <div className="topbar-side-cluster topbar-side-cluster--left min-w-0">
        {showCenterLane ? (
          <div
            className="topbar-main-decision-host relative shrink-0 overflow-visible"
            data-topbar-main-decision-host="true"
            style={{
              "--topbar-decision-accent": perspectiveAccent.hex,
              "--topbar-decision-rgb": perspectiveAccent.rgb,
            }}
          >
            {state?.decision?.kind === "priority" ? (
              <div className="table-action-bar relative h-full w-full rounded-none border">
                <DecisionPopupLayer priorityInline />
              </div>
            ) : null}
          </div>
        ) : (
          <h1 className="toolbar-brand topbar-brand m-0 whitespace-nowrap font-bold">
            Ironsmith
          </h1>
        )}
        {multiplayer?.matchStarted && offlinePlayers.length > 0 ? (
          <button
            type="button"
            className="stone-pill inline-flex min-h-8 max-w-[240px] items-center gap-2 rounded-none border border-[#7d302f] bg-[#2b1114]/90 px-2.5 py-1 text-[11px] font-semibold uppercase tracking-[0.14em] text-[#ffb8c0]"
            onClick={onOpenLobby}
            title={ui("Disconnected: {0}. Timeout policy in {1}.", { 0: connectionWarningLabel, 1: disconnectCountdown })}
            aria-label={ui("Disconnected players: {0}", { 0: connectionWarningLabel })}
          >
            <WifiOff className="size-3.5 shrink-0" />
            <span className="truncate">
              {offlinePlayers.length === 1
                ? `${connectionWarningLabel} ${disconnectCountdown}`
                : ui("{0} offline {1}", { 0: offlinePlayers.length, 1: disconnectCountdown })}
            </span>
          </button>
        ) : null}
        {showMatchClock ? (
          <div
            className="stone-pill inline-flex min-h-8 max-w-[520px] items-center gap-2 overflow-hidden rounded-none border border-[#5f4a22] bg-[#231c0e]/90 px-2.5 py-1 text-[11px] font-semibold uppercase tracking-[0.14em] text-[#ffd98a]"
            title={ui("Match clocks")}
            aria-label={ui("Per-player match clocks")}
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
                  {playerDisplayName(players, entry.player) || ui("P{0}", { 0: entry.index + 1 })} {formatTimerRemaining(entry.remainingMs)}
                </span>
              ))}
            </span>
          </div>
        ) : null}
        {showCenterLane ? (
          <div className="topbar-phase-shell">
            <PhaseTrack compact={middleDocked} />
            <div
              className="topbar-phase-status"
              aria-label={t("game.currentTurnSummary")}
            >
              <span>{t("game.turn", { turn: state?.turn_number ?? "-" })}</span>
              {activePlayer ? (
                <>
                  <span className="topbar-phase-status-dot" aria-hidden="true">•</span>
                  <span>{t("game.activePlayer", { player: playerDisplayName(players, activePlayer) })}</span>
                </>
              ) : null}
              {decisionOwnerDiffersFromPriority ? (
                <>
                  <span className="topbar-phase-status-dot" aria-hidden="true">•</span>
                  <span>{t("game.decisionPlayer", { player: playerDisplayName(players, decisionPlayer) })}</span>
                </>
              ) : priorityPlayer ? (
                <>
                  <span className="topbar-phase-status-dot" aria-hidden="true">•</span>
                  <span>
                    {t("game.priorityPlayer").split("{player}").map((part, index) => (
                      <span key={index}>
                        {index > 0 ? (
                          <span style={{ color: getPlayerAccent(players, priorityPlayer.id, state?.perspective, playerAccentOverrides)?.hex }}>
                            {playerDisplayName(players, priorityPlayer)}
                          </span>
                        ) : null}
                        {part}
                      </span>
                    ))}
                  </span>
                </>
              ) : null}
              {players.length > 0 ? (
                <>
                  <span className="topbar-phase-status-dot" aria-hidden="true">•</span>
                  <label className="topbar-phase-perspective">
                    <span>{t("action.playingAs")}</span>
                    <select
                      className="stone-select topbar-phase-perspective-select"
                      value={state?.perspective ?? me?.id ?? 0}
                      disabled={multiplayer.matchStarted}
                      onChange={(event) => onChangePerspective?.(Number(event.target.value))}
                      aria-label={t("action.playingAs")}
                    >
                      {players.map((player) => (
                        <option key={player.id} value={player.id}>
                          {playerDisplayName(players, player)}
                        </option>
                      ))}
                    </select>
                  </label>
                </>
              ) : null}
            </div>
          </div>
        ) : null}
        {showCompactPhase ? (
          <div className="topbar-mobile-status">
            <div className="topbar-phase-chip" aria-label={ui(translatedPhaseSummary)}>
              <span className="topbar-phase-chip-label">{ui(translatedCompactPhaseLabel)}</span>
              <span className="topbar-phase-chip-turn">{t("game.turn", { turn: state?.turn_number ?? "-" })}</span>
            </div>
            {nonDesktopViewport && activeMobileOpponent ? (
              <div
                className={`topbar-opponent-chip${activeMobileOpponentButtonEnabled ? " is-targetable" : ""}`}
                aria-label={ui("Viewing opponent {0}", { 0: playerDisplayName(players, activeMobileOpponent) })}
              >
                {opponents.length > 1 ? (
                  <button
                    type="button"
                    className="topbar-opponent-chip-nav"
                    data-player-nav-target={previousMobileOpponent?.index ?? previousMobileOpponent?.id}
                    data-player-nav-target-name={previousMobileOpponent?.id ?? previousMobileOpponent?.index}
                    onClick={() => cycleMobileOpponent(-1)}
                    aria-label={ui("Show previous opponent")}
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
                  aria-label={ui("Opponent {0}, life {1}", { 0: playerDisplayName(players, activeMobileOpponent), 1: activeMobileOpponent.life })}
                >
                  <span className="topbar-opponent-chip-name" style={{ color: activeMobileOpponent.id === activePlayer?.id ? "#fff0ca" : undefined }}>
                    {playerDisplayName(players, activeMobileOpponent)}
                  </span>
                  <span className="topbar-opponent-chip-life">{activeMobileOpponent.life}</span>
                  <span className="topbar-opponent-chip-meta">{ui("H") + " "}{activeMobileOpponent.hand_size ?? 0}{" " + ui("G") + " "}{activeMobileOpponent.graveyard_size ?? 0}{" " + ui("D") + " "}{activeMobileOpponent.library_size ?? 0}
                  </span>
                </button>
                {opponents.length > 1 ? (
                  <button
                    type="button"
                    className="topbar-opponent-chip-nav"
                    data-player-nav-target={nextMobileOpponent?.index ?? nextMobileOpponent?.id}
                    data-player-nav-target-name={nextMobileOpponent?.id ?? nextMobileOpponent?.index}
                    onClick={() => cycleMobileOpponent(1)}
                    aria-label={ui("Show next opponent")}
                  >
                    <ChevronRight className="size-3.5" />
                  </button>
                ) : null}
              </div>
            ) : null}
          </div>
        ) : null}
      </div>

      <div className="topbar-side-cluster topbar-side-cluster--right">
        {showCenterLane ? (
          <div className="topbar-brand-stack">
            <h1 className="toolbar-brand topbar-brand m-0 whitespace-nowrap font-bold">Ironsmith</h1>
            {tableToolsToggle}
          </div>
        ) : utilityControls}
      </div>
    </header>
  );
}
