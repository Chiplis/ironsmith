import { useRef } from "react";
import { useGame } from "@/context/GameContext";
import useViewportLayout from "@/hooks/useViewportLayout";
import OpponentZone from "./OpponentZone";
import MyZone from "./MyZone";
import DeckLoadingView from "./DeckLoadingView";
import DecisionPopupLayer from "@/components/overlays/DecisionPopupLayer";
import MobileBattleScene from "./MobileBattleScene";
export default function TableCore({
  selectedObjectId,
  onInspect,
  zoneViews,
  zoneActivityByPlayer = {},
  deckLoadingMode,
  onLoadDecks,
  onCancelDeckLoading,
  legalTargetPlayerIds = new Set(),
  legalTargetObjectIds = new Set(),
  myZoneHeaderControls = null,
  mobileOpponentIndex = 0,
  setMobileOpponentIndex,
}) {
  const { state } = useGame();
  const tableRef = useRef(null);
  const {
    portraitCompactViewport,
    landscapeMobileViewport,
    nonDesktopViewport,
  } = useViewportLayout();
  const players = state?.players || [];
  const perspective = state?.perspective;

  if (!players.length) {
    return <main className="table-gradient table-shell rounded-none min-h-0" />;
  }

  if (deckLoadingMode) {
    return <DeckLoadingView onLoad={onLoadDecks} onCancel={onCancelDeckLoading} />;
  }

  const me = players.find((p) => p.id === perspective) || players[0];
  const meIndex = players.findIndex((p) => p.id === me.id);
  const ordered = meIndex >= 0 ? [...players.slice(meIndex), ...players.slice(0, meIndex)] : players;
  const opponents = ordered.filter((p) => p.id !== me.id);
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
  const actionBarHeight = expandedActionBar
    ? (portraitCompactViewport || landscapeMobileViewport ? compactDecisionBarHeight : 124)
    : (portraitCompactViewport || landscapeMobileViewport ? compactPriorityBarHeight : 62);
  const mergeActionBarIntoMyZone = nonDesktopViewport;
  const actionBarElement = (
    <div
      className="table-action-bar relative h-full w-full rounded-none border border-[#2b3f57]/65 bg-[linear-gradient(90deg,rgba(7,15,23,0.92),rgba(14,28,44,0.86),rgba(7,15,23,0.92))] shadow-[inset_0_1px_0_rgba(170,208,245,0.12),0_8px_18px_rgba(0,0,0,0.32)]"
      data-expanded={expandedActionBar ? "true" : "false"}
    >
      <DecisionPopupLayer priorityInline selectedObjectId={selectedObjectId} />
    </div>
  );

  if (landscapeMobileViewport) {
    return (
      <MobileBattleScene
        me={me}
        opponents={opponents}
        selectedObjectId={selectedObjectId}
        onInspect={onInspect}
        zoneViews={zoneViews}
        zoneActivityByPlayer={zoneActivityByPlayer}
        legalTargetPlayerIds={legalTargetPlayerIds}
        legalTargetObjectIds={legalTargetObjectIds}
        mobileOpponentIndex={mobileOpponentIndex}
        setMobileOpponentIndex={setMobileOpponentIndex}
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
          : `minmax(0,1fr) ${actionBarHeight}px minmax(0,1fr)`,
      }}
    >
      <OpponentZone
        opponents={opponents}
        selectedObjectId={selectedObjectId}
        onInspect={onInspect}
        zoneViews={zoneViews}
        zoneActivityByPlayer={zoneActivityByPlayer}
        legalTargetPlayerIds={legalTargetPlayerIds}
        legalTargetObjectIds={legalTargetObjectIds}
        mobileViewport={nonDesktopViewport}
        activeOpponentIndex={mobileOpponentIndex}
        setActiveOpponentIndex={setMobileOpponentIndex}
      />
      {!mergeActionBarIntoMyZone && (
        <div className="relative z-20 flex items-center">
          {actionBarElement}
        </div>
      )}
      <MyZone
        player={me}
        selectedObjectId={selectedObjectId}
        onInspect={onInspect}
        zoneViews={zoneViews}
        zoneActivity={zoneActivityByPlayer[String(me?.id ?? me?.index ?? "")] || {}}
        legalTargetPlayerIds={legalTargetPlayerIds}
        legalTargetObjectIds={legalTargetObjectIds}
        headerControls={myZoneHeaderControls}
        embeddedActionBar={mergeActionBarIntoMyZone ? actionBarElement : null}
      />
    </main>
  );
}
