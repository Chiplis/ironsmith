import { useCallback, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import useViewportLayout from "@/hooks/useViewportLayout";
import OpponentZone from "./OpponentZone";
import MyZone, { ZoneCountInline } from "./MyZone";
import DeckLoadingView from "./DeckLoadingView";
import OpenDecklistModal from "./OpenDecklistModal";
import PuzzleSetupView from "./PuzzleSetupView";
import DecisionPopupLayer from "@/components/overlays/DecisionPopupLayer";
import MobileBattleScene from "./MobileBattleScene";
import ManaPool from "@/components/left-rail/ManaPool";
import StackTimelineRail from "@/components/right-rail/StackTimelineRail";
import { getPlayerAccent } from "@/lib/player-colors";
import { cn } from "@/lib/utils";
import { usePointerClickGuard } from "@/lib/usePointerClickGuard";
import { playerDisplayName, samePlayerId } from "@/lib/player-display";

function playerAccentStyle(accent) {
  return {
    "--player-accent": accent?.hex || "#d8bf6a",
    "--panel-accent": accent?.hex || "#b98946",
    "--player-accent-rgb": accent?.rgb || "216, 191, 106",
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
  middleInspectorDock = null,
}) {
  const { state, playerAccentOverrides, multiplayer } = useGame();
  const { registerPointerDown, shouldHandleClick } = usePointerClickGuard();
  const tableRef = useRef(null);
  const [openDecklist, setOpenDecklist] = useState(null);
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
  const expandedActionBar = Boolean(
    decision
    && decision.kind !== "priority"
    && decision.kind !== "attackers"
    && decision.kind !== "blockers"
  );
  const compactPriorityBarHeight = portraitCompactViewport
    ? 188
    : (landscapeMobileViewport ? 44 : 58);
  const compactDecisionBarHeight = portraitCompactViewport
    ? 236
    : (landscapeMobileViewport ? 92 : 112);
  const desktopPriorityBarHeight = largeDesktopViewport ? 54 : (smallDesktopViewport ? 48 : 50);
  const desktopDecisionBarHeight = largeDesktopViewport ? 138 : (smallDesktopViewport ? 112 : 128);
  const actionBarHeight = expandedActionBar
    ? (portraitCompactViewport || landscapeMobileViewport || tabletCompactViewport ? compactDecisionBarHeight : desktopDecisionBarHeight)
    : (portraitCompactViewport || landscapeMobileViewport || tabletCompactViewport ? compactPriorityBarHeight : desktopPriorityBarHeight);
  const mergeActionBarIntoMyZone = nonDesktopViewport || tabletCompactViewport;
  const dockStackRailInBoard = !mergeActionBarIntoMyZone && Boolean(zoneActionControls);
  const sharedMiddleControls = !mergeActionBarIntoMyZone && Boolean(middleTopbar || middleAddCardBar);
  const isActivePlayer = Number(state?.active_player) === Number(me?.id);
  const isPriorityPlayer = Number(state?.priority_player) === Number(me?.id);
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
    if (deck.length === 0 && sideboard.length === 0 && commanders.length === 0) return;
    setOpenDecklist({
      playerName: playerDisplayName(state?.players || [], player),
      deck,
      sideboard,
      commanders,
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
      className="table-action-bar relative h-full w-full rounded-none border border-[#2b3f57]/65 bg-[linear-gradient(90deg,rgba(7,15,23,0.92),rgba(14,28,44,0.86),rgba(7,15,23,0.92))] shadow-[inset_0_1px_0_rgba(170,208,245,0.12),0_8px_18px_rgba(0,0,0,0.32)]"
      data-expanded={expandedActionBar ? "true" : "false"}
    >
      <DecisionPopupLayer priorityInline selectedObjectId={selectedObjectId} />
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
          className={cn(
            "battlefield-life text-[23px] font-bold leading-none text-[#f5d08b] tabular-nums",
            isPlayerLegalTarget
              && "text-[#d7ebff] rounded-none px-1 py-0.5 shadow-[0_0_10px_rgba(100,169,255,0.5)] ring-1 ring-[#64a9ff]/55"
          )}
          onPointerDown={handlePlayerTargetPointerDown}
          onClick={handlePlayerTargetClick}
          style={{ cursor: isPlayerLegalTarget && canPickTargetFromBoard ? "pointer" : undefined }}
        >
          {me.life}
        </span>
        <span
          className={cn(
            "battlefield-name min-w-0 text-[16px] uppercase tracking-wider font-bold",
            isPlayerLegalTarget && "drop-shadow-[0_0_7px_rgba(100,169,255,0.7)]"
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
        <ManaPool
          pool={me.mana_pool}
          alwaysVisible
          compact
          className="player-name-mana battlefield-header-mana"
        />
        <div className="battlefield-header-zone-counts ml-auto flex min-w-0 flex-1 items-center justify-end gap-2">
          <ZoneCountInline player={me} onOpenDecklist={handleOpenDecklist} />
        </div>
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
        middleInspectorDock ? "z-[70]" : "z-20"
      )}
      style={playerAccentStyle(playerAccent)}
    >
      <div className="table-shared-control-stack relative z-[1] grid min-h-0 gap-0 overflow-visible">
        <div className="table-shared-action-slot relative overflow-visible" style={{ height: `${actionBarHeight}px` }}>
          {actionBarElement}
        </div>
        <div className="table-shared-toolbar-slot relative overflow-visible">
          {middleToolbarElement}
        </div>
        <div className="table-shared-player-slot relative overflow-visible">
          {middlePlayerHeaderElement}
        </div>
      </div>
      {middleInspectorDock ? (
        <div
          className="table-shared-inspector-dock pointer-events-none absolute bottom-0 right-2 z-[110] flex items-start justify-end overflow-visible"
          style={{ top: `${actionBarHeight}px`, width: "40vw" }}
          data-inspector-dock="middle"
        >
          {middleInspectorDock}
        </div>
      ) : null}
    </div>
  ) : null;
  if (landscapeMobileViewport) {
    return (
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
    );
  }

  return (
    <main
      ref={tableRef}
      className="table-gradient table-shell relative rounded-none grid gap-0 p-0 min-h-0 h-full overflow-visible"
      data-drop-zone
      style={{
        gridTemplateRows: mergeActionBarIntoMyZone
          ? "minmax(0,1fr) minmax(0,1fr)"
          : sharedMiddleElement
            ? "minmax(0,1.09fr) auto minmax(0,1fr)"
            : `minmax(0,1fr) ${actionBarHeight}px minmax(0,1fr)`,
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
      {!mergeActionBarIntoMyZone && !sharedMiddleElement && (
        <div className="relative z-20 flex items-center">
          {actionBarElement}
        </div>
      )}
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
        embeddedActionBar={mergeActionBarIntoMyZone ? actionBarElement : null}
        zoneActionControls={!mergeActionBarIntoMyZone ? zoneActionControls : null}
        hideHeader={Boolean(sharedMiddleElement)}
      />
      <OpenDecklistModal
        decklist={openDecklist}
        onClose={() => setOpenDecklist(null)}
      />
    </main>
  );
}
