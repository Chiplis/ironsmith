import useUiText from "@/i18n/useUiText";
import DiagnosticsSheet from "@/components/layout/DiagnosticsSheet";
import PriorityHoldControl from "@/components/decisions/PriorityHoldControl";
import { useCastPlayerHovered } from "@/context/DragContext";
import { cloneElement, useCallback, useLayoutEffect, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { normalizePhaseStep } from "@/lib/constants";
import useDecisionControlMotion from "@/hooks/useDecisionControlMotion";
import useViewportLayout from "@/hooks/useViewportLayout";
import OpponentZone from "./OpponentZone";
import MyZone from "./MyZone";
import DeckLoadingView from "./DeckLoadingView";
import OpenDecklistModal from "./OpenDecklistModal";
import PuzzleSetupView from "./PuzzleSetupView";
import DecisionPopupLayer from "@/components/overlays/DecisionPopupLayer";
import MobileBattleScene from "./MobileBattleScene";
import PlanarZone from "./PlanarZone";
import ManaPool from "@/components/left-rail/ManaPool";
import LobbyChat from "@/components/right-rail/LobbyChat";
import StackTimelineRail from "@/components/right-rail/StackTimelineRail";
import { DEFAULT_PLAYER_ACCENT, getPlayerAccent } from "@/lib/player-colors";
import { cn } from "@/lib/utils";
import { usePointerClickGuard } from "@/lib/usePointerClickGuard";
import { playerDisplayName, samePlayerId } from "@/lib/player-display";
import { useI18n } from "@/i18n/I18nContext";
import { findFloatingDockPosition } from "@/lib/floating-dock-position";

const FLOATING_DOCK_OBSTACLES = [
  ".game-card",
  ".deck-zone-pile",
  ".zone-pile-slot",
  ".zone-pile",
  ".battlefield-panel-header",
  ".table-shared-player-header",
  ".topbar-phase-shell",
  ".topbar-brand-stack",
  ".player-header-utility-controls",
  ".my-zone-stack-rail",
  ".stack-timeline-rail",
  ".player-zone-chat-dock",
  ".zone-pile-menu",
].join(",");

function visibleRect(element) {
  if (!element || !element.isConnected) return null;
  const style = window.getComputedStyle(element);
  if (style.display === "none" || style.visibility === "hidden" || Number(style.opacity) === 0) return null;
  const rect = element.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) return null;
  if (rect.right <= 0 || rect.bottom <= 0 || rect.left >= window.innerWidth || rect.top >= window.innerHeight) return null;
  return { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom };
}

function playerAccentStyle(accent) {
  const resolvedAccent = accent || DEFAULT_PLAYER_ACCENT;
  return {
    "--player-accent": resolvedAccent.hex,
    "--panel-accent": resolvedAccent.hex,
    "--player-accent-rgb": resolvedAccent.rgb,
  };
}

function sanitizeDeckCards(cards) {
  if (!Array.isArray(cards)) return [];
  return cards.map((card) => String(card || "").trim()).filter(Boolean);
}

function decisionContentPreferredWidth(decision) {
  if (!decision) return "420px";
  const optionLabels = [
    ...(Array.isArray(decision.candidates) ? decision.candidates : []),
    ...(Array.isArray(decision.options) ? decision.options : []),
  ].flatMap((option) => [option?.name, option?.label, option?.description]);
  const longestText = [
    decision.description,
    decision.context_text,
    decision.consequence_text,
    decision.reason,
    ...optionLabels,
  ].reduce((longest, value) => Math.max(longest, String(value || "").trim().length), 0);
  const preferredWidth = Math.max(
    420,
    Math.min(480, 420 + Math.max(0, longestText - 48) * 1.2),
  );
  return `${Math.round(preferredWidth)}px`;
}

function decisionCompactPreferredWidth(decision) {
  if (!decision) return "320px";
  const summaryLength = [
    decision.description,
    decision.context_text,
    decision.consequence_text,
    decision.reason,
  ].reduce((longest, value) => Math.max(longest, String(value || "").trim().length), 0);
  return `${Math.round(Math.max(320, Math.min(390, 320 + Math.max(0, summaryLength - 44) * 1.2)))}px`;
}

export default function TableCore({
  selectedObjectId,
  onInspect,
  focusedStackObjectId = null,
  onFocusStackObject = null,
  zoneViews,
  zoneActivityByPlayer = {},
  deckLoadingMode,
  puzzleSetupMode = false,
  onOpenLobby,
  onTestDecks,
  onCancelDeckLoading,
  onLoadPuzzle,
  onCancelPuzzleSetup,
  legalTargetPlayerIds = new Set(),
  legalTargetObjectIds = new Set(),
  myZoneHeaderControls = null,
  mobileOpponentIndex = 0,
  setMobileOpponentIndex,
  mobileViewMode = "battlefield",
  setMobileViewMode,
  middleUtilityControls = null,
  middleTopbar = null,
  middleAddCardBar = null,
  zoneActionControls = null,
  middleInspectorDock = null,
}) {
  const ui = useUiText();
  const {
    state,
    playerAccentOverrides,
    multiplayer,
    autoPassEnabled,
    setAutoPassEnabled,
  } = useGame();
  const { t } = useI18n();
  const { registerPointerDown, shouldHandleClick } = usePointerClickGuard();
  const tableRef = useRef(null);
  const humanActionDockRef = useRef(null);
  const [openDecklist, setOpenDecklist] = useState(null);
  const [humanActionDockPosition, setHumanActionDockPosition] = useState(null);
  const {
    portraitCompactViewport,
    landscapeMobileViewport,
    nonDesktopViewport,
    tabletCompactViewport,
    smallDesktopViewport,
    largeDesktopViewport,
  } = useViewportLayout();
  const focusedHudDesktop = !nonDesktopViewport && !tabletCompactViewport;
  const players = state?.players || [];
  const perspective = state?.perspective;

  const me = players.find((p) => p.id === perspective) || players[0] || null;
  const meIndex = me ? players.findIndex((p) => p.id === me.id) : -1;
  const ordered = me && meIndex >= 0 ? [...players.slice(meIndex), ...players.slice(0, meIndex)] : players;
  const opponents = me ? ordered.filter((p) => p.id !== me.id) : [];
  const playerAccent = me ? getPlayerAccent(players, me?.id, perspective, playerAccentOverrides) : null;
  const decision = state?.decision || null;
  const expandedActionBar = Boolean(
    state?.game_over
    || (decision && decision.kind !== "priority")
  );
  useDecisionControlMotion(tableRef, expandedActionBar);
  const compactPriorityBarHeight = portraitCompactViewport
    ? 188
    : (landscapeMobileViewport ? 44 : 58);
  const desktopDecisionBarHeight = largeDesktopViewport ? 138 : (smallDesktopViewport ? 112 : 128);
  // Keep a small breathing room below the shared controls on desktop. The
  // player's battlefield spans this track, so using the full toolbar height
  // here also becomes top padding on its card grid and leaves an oversized
  // empty band between the menu and the cards.
  const sharedMiddleBattlefieldInset = portraitCompactViewport || landscapeMobileViewport || tabletCompactViewport
    ? compactPriorityBarHeight
    : (expandedActionBar ? desktopDecisionBarHeight : 18);
  const mergeActionBarIntoMyZone = nonDesktopViewport || tabletCompactViewport;
  const dockStackRailInBoard = !mergeActionBarIntoMyZone && Boolean(zoneActionControls);
  const sharedMiddleControls = !mergeActionBarIntoMyZone
    && (focusedHudDesktop || Boolean(middleTopbar || middleAddCardBar));
  const isActivePlayer = Number(state?.active_player) === Number(me?.id);
  const isPriorityPlayer = Number(state?.priority_player) === Number(me?.id);
  const activePhaseStep = state ? normalizePhaseStep(state.phase, state.step) : null;
  const activePhaseLabel = activePhaseStep
    ? t(`game.track.${activePhaseStep}`, null, activePhaseStep)
    : "";
  const castPlayerHovered = useCastPlayerHovered(me?.id);
  const isPlayerLegalTarget =
    legalTargetPlayerIds.has(Number(me?.id)) || legalTargetPlayerIds.has(Number(me?.index));
  const canPickTargetFromBoard = state?.decision?.kind === "targets"
    && samePlayerId(state?.decision?.player, state?.perspective);
  const dispatchPlayerTargetChoice = useCallback(() => {
    if (!canPickTargetFromBoard || !isPlayerLegalTarget) return;
    const targetPlayer = legalTargetPlayerIds.has(Number(me?.id))
      ? Number(me?.id)
      : Number(me?.index);
    if (!Number.isFinite(targetPlayer)) return;
    window.dispatchEvent(
      new CustomEvent("ironsmith:target-choice", {
        detail: { target: { kind: "player", player: targetPlayer } },
      })
    );
  }, [
    canPickTargetFromBoard,
    isPlayerLegalTarget,
    legalTargetPlayerIds,
    me?.id,
    me?.index,
  ]);
  const handlePlayerTargetPointerDown = useCallback((event) => {
    if (!registerPointerDown(event)) return;
    event.preventDefault();
    event.stopPropagation();
    dispatchPlayerTargetChoice();
  }, [dispatchPlayerTargetChoice, registerPointerDown]);
  const handlePlayerTargetClick = useCallback((event) => {
    if (!shouldHandleClick(event)) return;
    event.preventDefault();
    event.stopPropagation();
    dispatchPlayerTargetChoice();
  }, [dispatchPlayerTargetChoice, shouldHandleClick]);

  const handleOpenDecklist = useCallback((player) => {
    const seat = Number(player?.index ?? player?.id);
    const matchPlayer = (multiplayer?.players || []).find((candidate) =>
      Number(candidate?.index) === seat
      || samePlayerId(candidate?.index, player?.id)
    );
    const deck = sanitizeDeckCards(matchPlayer?.deck);
    const sideboard = sanitizeDeckCards(matchPlayer?.sideboard);
    const commanders = sanitizeDeckCards(matchPlayer?.commanders);
    const available = Boolean(matchPlayer && Array.isArray(matchPlayer.deck));
    setOpenDecklist({
      playerName: playerDisplayName(state?.players || [], player),
      deck,
      sideboard,
      commanders,
      available,
    });
  }, [multiplayer?.players, state?.players]);

  useLayoutEffect(() => {
    const dock = humanActionDockRef.current;
    const table = tableRef.current;
    if (!focusedHudDesktop || !dock || !table) return undefined;

    let frame = 0;
    const observedElements = new Set();
    const resizeObserver = typeof ResizeObserver === "function"
      ? new ResizeObserver(schedule)
      : null;
    const observe = (element) => {
      if (!resizeObserver || !element || observedElements.has(element)) return;
      observedElements.add(element);
      resizeObserver.observe(element);
    };
    const update = () => {
      frame = 0;
      if (!dock.isConnected) return;
      const dockRect = dock.getBoundingClientRect();
      const dockWidth = dockRect.width || dock.offsetWidth;
      const dockHeight = dockRect.height || dock.offsetHeight;
      if (!dockWidth || !dockHeight) return;

      const obstacleElements = [...document.querySelectorAll(FLOATING_DOCK_OBSTACLES)]
        .filter((element) => element !== dock && !dock.contains(element));
      obstacleElements.forEach(observe);
      observe(dock);
      observe(table);
      const obstacles = obstacleElements.map((element) => {
        const rect = visibleRect(element);
        return rect ? {
          ...rect,
          protected: Boolean(element.closest(".zone-pile-slot, .deck-zone-pile, .zone-pile, .zone-pile-menu")),
        } : null;
      }).filter(Boolean);
      const handTop = [...table.querySelectorAll(".hand-card")]
        .map(visibleRect)
        .filter(Boolean)
        .reduce((top, rect) => Math.min(top, rect.top), Number.POSITIVE_INFINITY);
      const preferredTop = Number.isFinite(handTop)
        ? handTop - dockHeight - 18
        : window.innerHeight - dockHeight - 132;
      const position = findFloatingDockPosition({
        viewportWidth: window.innerWidth,
        viewportHeight: window.innerHeight,
        dockWidth,
        dockHeight,
        obstacles,
        preferredLeft: window.innerWidth - dockWidth - 18,
        preferredTop,
      });
      if (!position) return;
      setHumanActionDockPosition((previous) => (
        previous?.left === position.left
          && previous?.top === position.top
          && previous?.overlaps === position.overlaps
          ? previous
          : position
      ));
    };
    function schedule() {
      if (frame) return;
      frame = window.requestAnimationFrame(update);
    }

    schedule();
    const mutationObserver = new MutationObserver((records) => {
      if (records.some((record) => !dock.contains(record.target))) schedule();
    });
    mutationObserver.observe(table, { childList: true, subtree: true });
    window.addEventListener("resize", schedule);
    window.addEventListener("scroll", schedule, true);
    return () => {
      if (frame) window.cancelAnimationFrame(frame);
      resizeObserver?.disconnect();
      mutationObserver.disconnect();
      window.removeEventListener("resize", schedule);
      window.removeEventListener("scroll", schedule, true);
    };
  }, [focusedHudDesktop, deckLoadingMode, puzzleSetupMode, state?.players, state?.decision, state?.phase, state?.step]);

  if (!players.length) {
    return <main className="table-gradient table-shell rounded-none min-h-0" />;
  }

  if (deckLoadingMode) {
    return <DeckLoadingView onOpenLobby={onOpenLobby} onTestDecks={onTestDecks} onCancel={onCancelDeckLoading} />;
  }

  if (puzzleSetupMode) {
    return <PuzzleSetupView onLoadPuzzle={onLoadPuzzle} onCancel={onCancelPuzzleSetup} />;
  }
  const actionBarElement = (
    <div
      className="table-action-bar relative h-full w-full rounded-none border"
      data-expanded={expandedActionBar ? "true" : "false"}
    >
      <DecisionPopupLayer
        priorityInline
        replaceMiddleControls={expandedActionBar && sharedMiddleControls}
        selectedObjectId={selectedObjectId}
      />
    </div>
  );
  const humanQuickControlsElement = focusedHudDesktop ? (
    <div className="battlefield-phase-priority-controls">
      <div className="battlefield-human-quick-controls">
        <PriorityHoldControl compact />
        <button
          type="button"
          className="battlefield-auto-pass-toggle"
          data-enabled={autoPassEnabled ? "true" : "false"}
          aria-pressed={Boolean(autoPassEnabled)}
          aria-label={t("action.autoPass")}
          data-tooltip={t("action.autoPass")}
          onClick={() => setAutoPassEnabled((enabled) => !enabled)}
        >
          <svg aria-hidden="true" viewBox="0 0 20 20" fill="none">
            <path d="m3.25 4.5 5.5 5.5-5.5 5.5" />
            <path d="m10.25 4.5 5.5 5.5-5.5 5.5" />
          </svg>
          <span className="sr-only">{t("action.autoPass")}</span>
        </button>
      </div>
    </div>
  ) : null;
  const middleTopbarElement = middleTopbar;
  const middleToolbarElement = middleTopbarElement || middleAddCardBar ? (
    <div className="table-middle-toolbars relative z-20 grid gap-2 min-h-0 overflow-visible">
      <div className="table-middle-toolbar-stack grid gap-2 min-h-0">
        {middleTopbarElement}
        {middleAddCardBar}
      </div>
    </div>
  ) : null;
  const middlePlayerHeaderElement = sharedMiddleControls ? (
    <div
      className="table-shared-player-header battlefield-panel-header relative z-[92] flex h-full min-w-0 items-center gap-2 overflow-visible pr-2"
      data-turn-priority={isPriorityPlayer ? "true" : "false"}
    >
      <div className="flex min-w-0 items-center gap-2" data-my-zone-header-content>
        <div
          className={cn("player-identity-box inline-flex min-w-0 items-center gap-2", isPlayerLegalTarget && "player-target-box")}
          data-cast-hovered={isPlayerLegalTarget && castPlayerHovered ? "true" : undefined}
          style={playerAccentStyle(playerAccent)}
          data-player-target={me.id}
          onPointerDown={(event) => { if (event.target === event.currentTarget) handlePlayerTargetPointerDown(event); }}
          onClick={(event) => { if (event.target === event.currentTarget) handlePlayerTargetClick(event); }}
        >
          <span
            className={cn(
              "battlefield-life text-[23px] font-bold leading-none text-[#f5d08b] tabular-nums"
            )}
            data-player-target={me.id}
            onPointerDown={handlePlayerTargetPointerDown}
            onClick={handlePlayerTargetClick}
            role={isPlayerLegalTarget && canPickTargetFromBoard ? "button" : undefined}
            tabIndex={isPlayerLegalTarget && canPickTargetFromBoard ? 0 : undefined}
            aria-label={ui(isPlayerLegalTarget && canPickTargetFromBoard
              ? `Target ${playerDisplayName(state?.players || [], me)}`
              : undefined)}
            onKeyDown={(event) => {
              if (!isPlayerLegalTarget || !canPickTargetFromBoard) return;
              if (event.key !== "Enter" && event.key !== " ") return;
              event.preventDefault();
              dispatchPlayerTargetChoice();
            }}
            style={{ cursor: isPlayerLegalTarget && canPickTargetFromBoard ? "pointer" : undefined }}
          >
            {me.life}
          </span>
          <span
            className={cn(
              "battlefield-name min-w-0 text-[16px] uppercase tracking-wider font-bold"
            )}
            data-player-target={me.id}
            data-player-target-name={me.id}
            onPointerDown={handlePlayerTargetPointerDown}
            onClick={handlePlayerTargetClick}
            style={{
              cursor: isPlayerLegalTarget && canPickTargetFromBoard ? "pointer" : undefined,
            }}
          >
            <span className={cn(isActivePlayer && "battlefield-name-text--active")}>
              {playerDisplayName(state?.players || [], me)}
            </span>
          </span>
        </div>
        {!focusedHudDesktop ? <PriorityHoldControl /> : null}
        <ManaPool
          pool={me.mana_pool}
          alwaysVisible
          compact
          className="player-name-mana battlefield-header-mana"
        />
        {focusedHudDesktop ? (
          // Chat tab sits right after the player's name, between it and the
          // hand; the panel opens upward from there.
          <div className="player-header-chat-dock">
            <LobbyChat showOffline />
          </div>
        ) : null}
        {middleUtilityControls ? (
          <div className="player-header-utility-controls">
            {cloneElement(middleUtilityControls, {
              children: zoneActionControls ? (
                <>
                  <DiagnosticsSheet />
                  {zoneActionControls}
                </>
              ) : null,
            })}
          </div>
        ) : null}
      </div>
      {!dockStackRailInBoard ? (
        <StackTimelineRail
          selectedObjectId={selectedObjectId}
          onInspectObject={onInspect}
          className="h-full flex-1 self-stretch pl-2"
        />
      ) : null}
    </div>
  ) : null;
  const sharedMiddleElement = sharedMiddleControls ? (
    <div
      className={cn(
        "table-shared-control-band relative min-h-0 overflow-visible",
        expandedActionBar ? "z-[90]" : (middleInspectorDock ? "z-[70]" : "z-20")
      )}
      data-inspector-open={middleInspectorDock && selectedObjectId != null ? "true" : "false"}
      data-expanded-decision={expandedActionBar ? "true" : "false"}
      style={{
        ...playerAccentStyle(playerAccent),
        "--middle-inspector-width": "clamp(460px, calc(100vw - 600px), 840px)",
      }}
    >
      {expandedActionBar && middleTopbar && !focusedHudDesktop ? (
        <div className="table-decision-turn-status">
          {cloneElement(middleTopbar, { statusOnly: true })}
        </div>
      ) : null}
      <div className="table-decision-strips relative min-w-0">
        {!expandedActionBar ? (
          <div
            className="table-shared-control-stack relative z-[1] grid min-h-0 gap-0 overflow-visible"
          >
            <div className="table-shared-toolbar-slot relative overflow-visible">
              {middleToolbarElement}
            </div>
            <div className="table-shared-player-slot relative overflow-visible">
              {middlePlayerHeaderElement}
            </div>
          </div>
        ) : null}
        {expandedActionBar && !focusedHudDesktop ? (
          <div
            className="table-shared-action-slot table-decision-overlay-slot absolute inset-0 z-[115] overflow-visible"
          >
            {actionBarElement}
          </div>
        ) : null}
        {expandedActionBar && !focusedHudDesktop ? (
          <div
            className="table-decision-submit-slot"
            data-decision-submit-portal-host="true"
          />
        ) : null}
        {middleInspectorDock ? (
          <div
            className="table-shared-inspector-dock pointer-events-none absolute right-2 z-[110] flex items-start justify-end overflow-visible"
            style={{
              top: "2px",
              right: "20px",
              bottom: "0px",
              width: "var(--middle-inspector-width)",
            }}
            data-inspector-dock="middle"
          >
            {middleInspectorDock}
          </div>
        ) : null}
      </div>
      {expandedActionBar ? (
        <div className="table-decision-utility-row">
          {middlePlayerHeaderElement}
        </div>
      ) : null}
    </div>
  ) : null;
  const planarZoneElement = (
    <PlanarZone
      state={state}
      selectedObjectId={selectedObjectId}
      onInspect={onInspect}
    />
  );
  const humanActionDockElement = focusedHudDesktop ? (
    <div
      ref={humanActionDockRef}
      className="battlefield-human-action-dock"
      data-human-action-dock
      data-placement-overlap={humanActionDockPosition?.overlaps ? "true" : undefined}
      style={{
        "--decision-panel-content-width": decisionContentPreferredWidth(decision),
        "--decision-panel-compact-width": decisionCompactPreferredWidth(decision),
        ...(humanActionDockPosition
          ? {
            left: `${humanActionDockPosition.left}px`,
            top: `${humanActionDockPosition.top}px`,
            right: "auto",
            bottom: "auto",
            visibility: "visible",
          }
          : { visibility: "hidden" }),
      }}
    >
      <div className="battlefield-human-decision-dock">
        <div className="table-action-bar battlefield-human-decision-panel">
          <DecisionPopupLayer
            priorityInline
            selectedObjectId={selectedObjectId}
          />
          <div className="battlefield-human-step-submit-row">
            <span
              className="battlefield-human-step-chip"
              data-phase-name={activePhaseStep || "none"}
              aria-label={activePhaseLabel || undefined}
            >
              <span className="battlefield-human-step-marker" aria-hidden="true" />
              <span>{activePhaseLabel || ui("Waiting")}</span>
            </span>
            <div
              className="table-decision-submit-slot"
              data-decision-submit-portal-host="true"
            />
          </div>
        </div>
      </div>
    </div>
  ) : null;
  if (landscapeMobileViewport) {
    return (
      <div className="relative h-full min-h-0">
        <MobileBattleScene
          me={me}
          opponents={opponents}
          selectedObjectId={selectedObjectId}
          onInspect={onInspect}
          focusedStackObjectId={focusedStackObjectId}
          onFocusStackObject={onFocusStackObject}
          legalTargetPlayerIds={legalTargetPlayerIds}
          legalTargetObjectIds={legalTargetObjectIds}
          mobileOpponentIndex={mobileOpponentIndex}
          setMobileOpponentIndex={setMobileOpponentIndex}
          mobileViewMode={mobileViewMode}
          setMobileViewMode={setMobileViewMode}
          onOpenDecklist={handleOpenDecklist}
        />
        <OpenDecklistModal decklist={openDecklist} onClose={() => setOpenDecklist(null)} />
        {planarZoneElement}
      </div>
    );
  }

  return (
    <main
      ref={tableRef}
      className="table-gradient table-shell relative rounded-none grid gap-0 p-0 min-h-0 h-full overflow-visible"
      data-drop-zone
      data-tablet-compact={tabletCompactViewport ? "true" : "false"}
      data-decision-strip-removed={sharedMiddleElement ? "true" : "false"}
      data-focused-hud={focusedHudDesktop ? "true" : "false"}
      style={{
        // Keep in sync with the focused HUD split in design-system.css.
        gridTemplateRows: focusedHudDesktop
          ? "minmax(0,0.86fr) minmax(0,1.14fr)"
          : mergeActionBarIntoMyZone
          ? (tabletCompactViewport
            ? "minmax(0,0.9fr) minmax(0,1.1fr)"
            : "minmax(0,1fr) minmax(0,1fr)")
          : sharedMiddleElement
            ? `minmax(0,1.09fr) auto ${sharedMiddleBattlefieldInset}px minmax(0,1fr)`
            : "minmax(0,1fr) minmax(0,1fr)",
      }}
    >
      <OpponentZone
        opponents={opponents}
        selectedObjectId={selectedObjectId}
        onInspect={onInspect}
        onOpenDecklist={handleOpenDecklist}
        zoneViews={zoneViews}
        zoneActivityByPlayer={zoneActivityByPlayer}
        legalTargetPlayerIds={legalTargetPlayerIds}
        legalTargetObjectIds={legalTargetObjectIds}
        mobileViewport={nonDesktopViewport}
        activeOpponentIndex={mobileOpponentIndex}
        setActiveOpponentIndex={setMobileOpponentIndex}
      />
      {planarZoneElement}
      {!mergeActionBarIntoMyZone && sharedMiddleElement}
      {!mergeActionBarIntoMyZone && !sharedMiddleElement && middleToolbarElement}
      {humanQuickControlsElement}
      <MyZone
        player={me}
        selectedObjectId={selectedObjectId}
        onInspect={onInspect}
        onOpenDecklist={handleOpenDecklist}
        zoneViews={zoneViews}
        zoneActivity={zoneActivityByPlayer[String(me?.id ?? me?.index ?? "")] || {}}
        legalTargetPlayerIds={legalTargetPlayerIds}
        legalTargetObjectIds={legalTargetObjectIds}
        headerControls={myZoneHeaderControls}
        stackAdjacentControls={null}
        hidePriorityHold={focusedHudDesktop}
        headerInspectorDock={!mergeActionBarIntoMyZone && !sharedMiddleElement ? middleInspectorDock : null}
        headerActionBar={!mergeActionBarIntoMyZone && !sharedMiddleElement ? actionBarElement : null}
        embeddedActionBar={mergeActionBarIntoMyZone ? actionBarElement : null}
        zoneActionControls={!mergeActionBarIntoMyZone && !sharedMiddleElement ? zoneActionControls : null}
        dockStackRail={dockStackRailInBoard}
        hideHeader={Boolean(sharedMiddleElement)}
        hideMobileHandRail={tabletCompactViewport}
        tableGridRow={sharedMiddleElement && !focusedHudDesktop ? "3 / span 2" : null}
        battlefieldTopInset={focusedHudDesktop
          ? 16
          : (sharedMiddleElement ? sharedMiddleBattlefieldInset : 0)}
      />
      {humanActionDockElement}
      <OpenDecklistModal
        decklist={openDecklist}
        onClose={() => setOpenDecklist(null)}
      />
    </main>
  );
}
