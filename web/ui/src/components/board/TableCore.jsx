import PriorityHoldControl from "@/components/decisions/PriorityHoldControl";
import { useCastPlayerHovered } from "@/context/DragContext";
import { useCallback, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import useViewportLayout from "@/hooks/useViewportLayout";
import OpponentZone from "./OpponentZone";
import MyZone from "./MyZone";
import DeckLoadingView from "./DeckLoadingView";
import OpenDecklistModal from "./OpenDecklistModal";
import PuzzleSetupView from "./PuzzleSetupView";
import DecisionPopupLayer from "@/components/overlays/DecisionPopupLayer";
import MobileBattleScene from "./MobileBattleScene";
import PlanarZone from "./PlanarZone";
import ZoneViewer from "./ZoneViewer";
import ManaPool from "@/components/left-rail/ManaPool";
import StackTimelineRail from "@/components/right-rail/StackTimelineRail";
import { DEFAULT_PLAYER_ACCENT, getPlayerAccent } from "@/lib/player-colors";
import { cn } from "@/lib/utils";
import { usePointerClickGuard } from "@/lib/usePointerClickGuard";
import { playerDisplayName, samePlayerId } from "@/lib/player-display";
import { useI18n } from "@/i18n/I18nContext";
import { ChevronDown, ChevronRight } from "lucide-react";

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

export default function TableCore({
  selectedObjectId,
  onInspect,
  focusedStackObjectId = null,
  onFocusStackObject = null,
  zoneViews,
  zoneViewerViews = zoneViews,
  zoneActivityByPlayer = {},
  deckLoadingMode,
  puzzleSetupMode = false,
  onLoadDecks,
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
  mobilePhaseStops,
  setMobilePhaseStops,
  middleTopbar = null,
  middleAddCardBar = null,
  zoneActionControls = null,
  setZoneViews = null,
  middleInspectorDock = null,
}) {
  const { state, playerAccentOverrides, multiplayer } = useGame();
  const { t } = useI18n();
  const { registerPointerDown, shouldHandleClick } = usePointerClickGuard();
  const tableRef = useRef(null);
  const [openDecklist, setOpenDecklist] = useState(null);
  const [tableToolsExpanded, setTableToolsExpanded] = useState(true);
  const {
    portraitCompactViewport,
    landscapeMobileViewport,
    nonDesktopViewport,
    tabletCompactViewport,
    smallDesktopViewport,
    largeDesktopViewport,
  } = useViewportLayout();
  const players = state?.players || [];
  const perspective = state?.perspective;

  const me = players.find((p) => p.id === perspective) || players[0] || null;
  const meIndex = me ? players.findIndex((p) => p.id === me.id) : -1;
  const ordered = me && meIndex >= 0 ? [...players.slice(meIndex), ...players.slice(0, meIndex)] : players;
  const opponents = me ? ordered.filter((p) => p.id !== me.id) : [];
  const playerAccent = me ? getPlayerAccent(players, me?.id, perspective, playerAccentOverrides) : null;
  const decision = state?.decision || null;
  const activeZoneActionControls = tableToolsExpanded ? zoneActionControls : null;
  const expandedActionBar = Boolean(
    decision
    && decision.kind !== "priority"
  );
  const compactPriorityBarHeight = portraitCompactViewport
    ? 188
    : (landscapeMobileViewport ? 44 : 58);
  const compactDecisionBarHeight = portraitCompactViewport
    ? 236
    : (landscapeMobileViewport ? 92 : 112);
  const desktopPriorityBarHeight = largeDesktopViewport ? 60 : (smallDesktopViewport ? 54 : 56);
  const desktopDecisionBarHeight = largeDesktopViewport ? 138 : (smallDesktopViewport ? 112 : 128);
  const actionBarHeight = expandedActionBar
    ? (portraitCompactViewport || landscapeMobileViewport || tabletCompactViewport ? compactDecisionBarHeight : desktopDecisionBarHeight)
    : (portraitCompactViewport || landscapeMobileViewport || tabletCompactViewport ? compactPriorityBarHeight : desktopPriorityBarHeight);
  const sharedMiddleBattlefieldInset = portraitCompactViewport || landscapeMobileViewport || tabletCompactViewport
    ? compactPriorityBarHeight
    : desktopPriorityBarHeight;
  const mergeActionBarIntoMyZone = nonDesktopViewport || tabletCompactViewport;
  const dockStackRailInBoard = !mergeActionBarIntoMyZone && Boolean(zoneActionControls);
  const sharedMiddleControls = !mergeActionBarIntoMyZone && Boolean(middleTopbar || middleAddCardBar);
  const isActivePlayer = Number(state?.active_player) === Number(me?.id);
  const isPriorityPlayer = Number(state?.priority_player) === Number(me?.id);
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

  if (!players.length) {
    return <main className="table-gradient table-shell rounded-none min-h-0" />;
  }

  if (deckLoadingMode) {
    return <DeckLoadingView onLoad={onLoadDecks} onCancel={onCancelDeckLoading} />;
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
  const middleToolbarElement = middleTopbar || middleAddCardBar ? (
    <div className="table-middle-toolbars relative z-20 grid gap-2 min-h-0 overflow-visible">
      <div className="table-middle-toolbar-stack grid gap-2 min-h-0">
        {middleTopbar}
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
        <span
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
            aria-label={isPlayerLegalTarget && canPickTargetFromBoard
              ? `Target ${playerDisplayName(state?.players || [], me)}`
              : undefined}
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
        </span>
        <PriorityHoldControl />
        {zoneActionControls ? (
          <button
            type="button"
            className="table-tools-toggle"
            aria-expanded={tableToolsExpanded}
            aria-controls="table-utility-actions"
            aria-label={t(tableToolsExpanded ? "action.hideTableTools" : "action.showTableTools")}
            title={t(tableToolsExpanded ? "action.hideTableTools" : "action.showTableTools")}
            onClick={() => setTableToolsExpanded((expanded) => !expanded)}
          >
            {tableToolsExpanded ? (
              <ChevronDown aria-hidden="true" />
            ) : (
              <ChevronRight aria-hidden="true" />
            )}
          </button>
        ) : null}
        <ManaPool
          pool={me.mana_pool}
          alwaysVisible
          compact
          className="player-name-mana battlefield-header-mana"
        />
        {zoneViewerViews && setZoneViews ? (
          <div className="player-header-zone-visibility" aria-label="Visible table zones">
            <ZoneViewer
              player={me}
              zoneViews={zoneViewerViews}
              setZoneViews={setZoneViews}
              onOpenDecklist={handleOpenDecklist}
              embedded
            />
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
      <div
        className="table-shared-control-stack relative z-[1] grid min-h-0 gap-0 overflow-visible"
        aria-hidden={expandedActionBar ? "true" : undefined}
        inert={expandedActionBar ? true : undefined}
      >
        <div className="table-shared-toolbar-slot relative overflow-visible">
          {middleToolbarElement}
        </div>
        <div className="table-shared-player-slot relative overflow-visible">
          {middlePlayerHeaderElement}
        </div>
      </div>
      {expandedActionBar ? (
        <div
          className="table-shared-action-slot table-decision-overlay-slot absolute inset-0 z-[115] overflow-visible"
          data-tools-expanded={activeZoneActionControls ? "true" : "false"}
        >
          {actionBarElement}
        </div>
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
  ) : null;
  const planarZoneElement = (
    <PlanarZone
      state={state}
      selectedObjectId={selectedObjectId}
      onInspect={onInspect}
    />
  );
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
          mobilePhaseStops={mobilePhaseStops}
          setMobilePhaseStops={setMobilePhaseStops}
        />
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
      style={{
        gridTemplateRows: mergeActionBarIntoMyZone
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
        headerInspectorDock={!mergeActionBarIntoMyZone && !sharedMiddleElement ? middleInspectorDock : null}
        headerActionBar={!mergeActionBarIntoMyZone && !sharedMiddleElement ? actionBarElement : null}
        embeddedActionBar={mergeActionBarIntoMyZone ? actionBarElement : null}
        zoneActionControls={!mergeActionBarIntoMyZone ? zoneActionControls : null}
        zoneActionControlsOpen={tableToolsExpanded}
        zoneActionRailOffset={!mergeActionBarIntoMyZone && !sharedMiddleElement && activeZoneActionControls ? actionBarHeight : 0}
        dockStackRail={dockStackRailInBoard}
        hideHeader={Boolean(sharedMiddleElement)}
        hideMobileHandRail={tabletCompactViewport}
        tableGridRow={sharedMiddleElement ? "3 / span 2" : null}
        battlefieldTopInset={sharedMiddleElement ? sharedMiddleBattlefieldInset : 0}
      />
      <OpenDecklistModal
        decklist={openDecklist}
        onClose={() => setOpenDecklist(null)}
      />
    </main>
  );
}
