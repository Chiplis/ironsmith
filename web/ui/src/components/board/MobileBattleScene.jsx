import { useCallback, useEffect, useMemo, useState } from "react";
import { useGame } from "@/context/GameContext";
import DecisionPopupLayer from "@/components/overlays/DecisionPopupLayer";
import ActionPopover from "@/components/overlays/ActionPopover";
import HoverArtOverlay from "@/components/right-rail/HoverArtOverlay";
import StackCard from "@/components/cards/StackCard";
import useNewCards from "@/hooks/useNewCards";
import useStackStartAlert from "@/hooks/useStackStartAlert";
import { X } from "lucide-react";
import { getVisibleStackObjects } from "@/lib/stack-targets";
import OpponentZone from "./OpponentZone";
import MyZone from "./MyZone";
import { getPlayerAccent } from "@/lib/player-colors";
import { cn } from "@/lib/utils";

function zoneCount(player, zone) {
  switch (zone) {
    case "hand":
      return player?.hand_size ?? 0;
    case "graveyard":
      return player?.graveyard_size ?? 0;
    case "library":
      return player?.library_size ?? 0;
    default:
      return 0;
  }
}

function ZoneChip({ label, count, accent, emphasize = false }) {
  return (
    <div
      className={cn(
        "mobile-battle-zone-chip",
        emphasize && "mobile-battle-zone-chip--emphasize"
      )}
      style={accent ? { "--zone-accent": accent.hex } : undefined}
    >
      <span className="mobile-battle-zone-chip-label">{label}</span>
      <span className="mobile-battle-zone-chip-count">{count}</span>
    </div>
  );
}

function MobileStackTray({
  objects = [],
  previews = [],
  onInspect,
  visible = false,
}) {
  const { state } = useGame();
  const stackIds = useMemo(
    () => objects.map((entry) => String(entry.id)),
    [objects]
  );
  const { newIds } = useNewCards(stackIds);
  const { alertEntryId, dismissAlert } = useStackStartAlert(
    objects,
    state?.perspective
  );
  const hasStackObjects = objects.length > 0;
  const hasPreviewEntries = !hasStackObjects && previews.length > 0;
  const stackCount = hasStackObjects ? objects.length : previews.length;

  const handleInspect = useCallback((objectId, meta) => {
    dismissAlert();
    onInspect?.(objectId, meta);
  }, [dismissAlert, onInspect]);

  if (!visible || (!hasStackObjects && !hasPreviewEntries)) return null;

  return (
    <aside
      className="mobile-battle-stack-tray"
      style={{ "--mobile-stack-count": stackCount }}
      aria-label={`Stack${stackCount > 0 ? ` (${stackCount})` : ""}`}
    >
      <div className="mobile-battle-stack-tray-header">
        <span className="mobile-battle-stack-tray-label">Stack</span>
        <span className="mobile-battle-stack-tray-count">{stackCount}</span>
      </div>
      <div className="mobile-battle-stack-tray-scroll">
        <div className="mobile-battle-stack-tray-row">
          {hasStackObjects
            ? objects.map((entry, index) => (
                <div
                  key={entry.id}
                  className="mobile-battle-stack-entry"
                  style={{ zIndex: Math.max(1, objects.length - index) }}
                >
                  <StackCard
                    entry={entry}
                    isNew={newIds.has(String(entry.id))}
                    showStackAlert={
                      alertEntryId != null
                      && String(entry.id) === String(alertEntryId)
                    }
                    className="mobile-battle-stack-card"
                    entryMotion="mobile-stack"
                    onClick={handleInspect}
                  />
                </div>
              ))
            : previews.map((name, index) => (
                <div
                  key={`${name}-${index}`}
                  className="mobile-battle-stack-preview"
                  style={{ zIndex: Math.max(1, previews.length - index) }}
                >
                  <span className="mobile-battle-stack-preview-label">Incoming</span>
                  <span className="mobile-battle-stack-preview-name">{name}</span>
                </div>
              ))}
        </div>
      </div>
    </aside>
  );
}

export default function MobileBattleScene({
  me,
  opponents,
  selectedObjectId,
  onInspect,
  zoneViews,
  zoneActivityByPlayer = {},
  legalTargetPlayerIds = new Set(),
  legalTargetObjectIds = new Set(),
  mobileOpponentIndex = 0,
  setMobileOpponentIndex,
}) {
  const { state, dispatch } = useGame();
  const activeOpponent = opponents[Math.min(mobileOpponentIndex, Math.max(0, opponents.length - 1))] || opponents[0] || null;
  const opponentAccent = getPlayerAccent(state?.players || [], activeOpponent?.id);
  const selfAccent = getPlayerAccent(state?.players || [], me?.id);
  const visibleStackObjects = useMemo(
    () => getVisibleStackObjects(state),
    [state]
  );
  const stackPreviews = useMemo(
    () => Array.isArray(state?.stack_preview) ? state.stack_preview : [],
    [state?.stack_preview]
  );
  const showMobileStackTray = visibleStackObjects.length > 0 || stackPreviews.length > 0;

  const closeInspector = useCallback(() => {
    onInspect?.(null);
  }, [onInspect]);
  const [actionPopoverState, setActionPopoverState] = useState(null);
  const decisionIdentity = useMemo(() => {
    const decision = state?.decision || null;
    return [
      decision?.kind || "",
      decision?.player ?? "",
      decision?.source_id ?? "",
      decision?.source_name || "",
      decision?.reason || "",
      decision?.description || "",
    ].join("|");
  }, [state?.decision]);

  useEffect(() => {
    if (selectedObjectId != null) {
      setActionPopoverState(null);
    }
  }, [selectedObjectId]);

  useEffect(() => {
    setActionPopoverState((current) => {
      if (!current) return current;
      if (current.decisionIdentity !== decisionIdentity) return null;
      if (state?.decision?.kind !== "priority") return null;
      const currentIndices = new Set(
        (state?.decision?.actions || []).map((action) => Number(action?.index))
      );
      const nextActions = (current.actions || []).filter((action) =>
        currentIndices.has(Number(action?.index))
      );
      if (nextActions.length === 0) return null;
      return { ...current, actions: nextActions };
    });
  }, [decisionIdentity, state?.decision]);

  const closeActionPopover = useCallback(() => {
    setActionPopoverState(null);
  }, []);

  const openObjectActions = useCallback(({ card, actions = [], anchorRect = null }) => {
    if (!Array.isArray(actions) || actions.length === 0 || state?.decision?.kind !== "priority") {
      return false;
    }

    const normalizedAnchorRect = anchorRect
      ? {
        left: anchorRect.left,
        top: anchorRect.top,
        right: anchorRect.right,
        bottom: anchorRect.bottom,
        width: anchorRect.width,
        height: anchorRect.height,
      }
      : null;

    setActionPopoverState((current) => {
      if (current?.objectId === Number(card?.id)) return null;
      return {
        objectId: Number(card?.id),
        cardName: card?.name || "Actions",
        anchorRect: normalizedAnchorRect,
        actions,
        decisionIdentity,
      };
    });
    onInspect?.(null);
    return true;
  }, [decisionIdentity, onInspect, state?.decision?.kind]);

  const inspectHeldObject = useCallback(({ card }) => {
    closeActionPopover();
    onInspect?.(card?.id ?? null);
  }, [closeActionPopover, onInspect]);

  const handlePopoverAction = useCallback((action) => {
    if (!action) return;
    dispatch(
      { type: "priority_action", action_index: action.index },
      action.label
    );
    closeActionPopover();
  }, [closeActionPopover, dispatch]);

  return (
    <main
      className="mobile-battle-scene table-gradient table-shell relative h-full min-h-0 overflow-hidden"
      data-drop-zone
      data-mobile-battle-scene
    >
      <div className="mobile-battle-scene-vignette" aria-hidden="true" />
      <div className="mobile-battle-scene-runeband" aria-hidden="true" />

      {activeOpponent ? (
        <div className="mobile-battle-top-hud">
          <div className="mobile-battle-top-zones">
            <ZoneChip label="Deck" count={zoneCount(activeOpponent, "library")} accent={opponentAccent} />
            <ZoneChip label="GY" count={zoneCount(activeOpponent, "graveyard")} accent={opponentAccent} emphasize />
          </div>
        </div>
      ) : null}

      <div className="mobile-battle-scene-opponent-stage">
        <OpponentZone
          opponents={opponents}
          selectedObjectId={selectedObjectId}
          onInspect={onInspect}
          zoneViews={zoneViews}
          zoneActivityByPlayer={zoneActivityByPlayer}
          legalTargetPlayerIds={legalTargetPlayerIds}
          legalTargetObjectIds={legalTargetObjectIds}
          mobileViewport
          mobileBattleScene
          activeOpponentIndex={mobileOpponentIndex}
          setActiveOpponentIndex={setMobileOpponentIndex}
          onMobileCardActionMenu={openObjectActions}
          onMobileCardLongPress={inspectHeldObject}
        />
      </div>

      <div className="mobile-battle-scene-self-stage">
        <MyZone
          player={me}
          selectedObjectId={selectedObjectId}
          onInspect={onInspect}
          zoneViews={zoneViews}
          zoneActivity={zoneActivityByPlayer[String(me?.id ?? me?.index ?? "")] || {}}
          legalTargetPlayerIds={legalTargetPlayerIds}
          legalTargetObjectIds={legalTargetObjectIds}
          mobileBattleScene
          playerAccent={selfAccent}
          onMobileCardActionMenu={openObjectActions}
          onMobileCardLongPress={inspectHeldObject}
        />
        <MobileStackTray
          objects={visibleStackObjects}
          previews={stackPreviews}
          visible={showMobileStackTray}
          onInspect={onInspect}
        />
      </div>

      <div className="mobile-battle-scene-action-dock">
        <DecisionPopupLayer
          priorityInline
          selectedObjectId={selectedObjectId}
          mobileBattle
        />
      </div>

      {selectedObjectId != null ? (
        <>
          <button
            type="button"
            className="mobile-battle-inspect-overlay-backdrop"
            aria-label="Close inspector"
            onClick={closeInspector}
          />
          <div
            className="mobile-battle-inspect-overlay"
            data-mobile-hand-drop-target="inspector"
          >
            <div className="mobile-battle-inspect-overlay-shell">
              <button
                type="button"
                className="mobile-battle-inspect-overlay-close"
                aria-label="Close inspector"
                onClick={closeInspector}
              >
                <X className="h-4 w-4" aria-hidden="true" />
              </button>
              <div className="mobile-battle-inspect-overlay-stage">
                <HoverArtOverlay
                  objectId={selectedObjectId}
                  displayMode="inspector"
                  availableInspectorWidth={360}
                  availableInspectorHeight={228}
                  hideOwnershipMetadata
                  minInspectorTextScale={0.54}
                  minInspectorTitleScale={0.46}
                  onInspectorAccentChange={null}
                />
              </div>
            </div>
          </div>
        </>
      ) : null}

      {actionPopoverState?.anchorRect ? (
        <>
          <button
            type="button"
            className="mobile-battle-action-popover-backdrop"
            aria-label="Close action menu"
            onClick={closeActionPopover}
          />
          <ActionPopover
            anchorRect={actionPopoverState.anchorRect}
            actions={actionPopoverState.actions}
            onAction={handlePopoverAction}
            onClose={closeActionPopover}
            title={actionPopoverState.cardName}
            subtitle="Available actions"
            variant="game"
          />
        </>
      ) : null}
    </main>
  );
}
