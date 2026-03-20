import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { X } from "lucide-react";
import { useGame } from "@/context/GameContext";
import { useCombatArrows } from "@/context/useCombatArrows";
import DecisionPopupLayer, {
  MobileDecisionActionList,
  MobileDecisionSheet,
} from "@/components/overlays/DecisionPopupLayer";
import HoverArtOverlay from "@/components/right-rail/HoverArtOverlay";
import StackCard from "@/components/cards/StackCard";
import BattlefieldRow from "@/components/board/BattlefieldRow";
import HandZone from "@/components/board/HandZone";
import useNewCards from "@/hooks/useNewCards";
import useStackStartAlert from "@/hooks/useStackStartAlert";
import useMobileBattleLayout from "@/hooks/useMobileBattleLayout";
import { getVisibleStackObjects } from "@/lib/stack-targets";
import { partitionBattlefieldCards } from "@/lib/battlefield-layout";
import { getPlayerAccent } from "@/lib/player-colors";
import { scryfallImageUrl } from "@/lib/scryfall";
import { cn } from "@/lib/utils";
import { usePointerClickGuard } from "@/lib/usePointerClickGuard";

const DEFAULT_TOPBAR_HEIGHT = 30;
const DEFAULT_CONTROL_BAND_HEIGHT = 30;
const DEFAULT_BOTTOM_BAND_HEIGHT = 58;

function stripActionPrefix(label = "") {
  const activateMatch = String(label).match(/^Activate\s+.+?:\s*(.+)$/i);
  if (activateMatch) return activateMatch[1];
  return String(label);
}

function stackKindLabel(entry) {
  if (!entry) return "Stack";
  if (!entry.ability_kind) return "Spell";
  return `${entry.ability_kind} ability`;
}

function buildInlineStackPreview(objects = [], previews = []) {
  const topObject = objects[0] || null;
  if (topObject) {
    return {
      key: `object:${topObject.id}`,
      name: topObject.name || `Object #${topObject.id}`,
      subtitle: stackKindLabel(topObject),
      artUrl: scryfallImageUrl(topObject.name || "", "art_crop"),
    };
  }

  const topPreview = previews[0] || null;
  if (topPreview) {
    return {
      key: `preview:${topPreview}`,
      name: String(topPreview),
      subtitle: "Incoming",
      artUrl: scryfallImageUrl(String(topPreview), "art_crop"),
    };
  }

  return null;
}

function zoneCount(player, zone) {
  switch (zone) {
    case "hand":
      return player?.hand_size ?? 0;
    case "graveyard":
      return player?.graveyard_size ?? 0;
    case "library":
      return player?.library_size ?? 0;
    case "exile":
      return Array.isArray(player?.exile_cards) ? player.exile_cards.length : 0;
    default:
      return 0;
  }
}

function collectCardObjectIds(card) {
  const ids = [Number(card?.id)];
  if (Array.isArray(card?.member_ids)) {
    for (const memberId of card.member_ids) {
      ids.push(Number(memberId));
    }
  }
  return ids.filter((id) => Number.isFinite(id));
}

function buildActivatableMap(decision) {
  const activatableMap = new Map();
  if (decision?.kind !== "priority" || !Array.isArray(decision.actions)) {
    return activatableMap;
  }

  for (const action of decision.actions) {
    if (
      (action.kind === "activate_ability" || action.kind === "activate_mana_ability")
      && action.object_id != null
    ) {
      const objectId = Number(action.object_id);
      if (!activatableMap.has(objectId)) activatableMap.set(objectId, []);
      activatableMap.get(objectId).push(action);
    }
  }

  return activatableMap;
}

function MobileStackTray({
  objects = [],
  previews = [],
  onInspect,
  visible = false,
  inline = false,
  embedded = false,
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
      className={cn(
        "mobile-battle-stack-tray",
        inline && "mobile-battle-stack-tray--inline",
        embedded && "mobile-battle-stack-tray--embedded"
      )}
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

function measureElementHeight(target, fallback, setHeight) {
  if (!target) {
    setHeight((current) => current || fallback);
    return null;
  }

  const update = () => {
    const nextHeight = Math.max(fallback, Math.ceil(target.getBoundingClientRect().height || 0));
    setHeight((current) => (Math.abs(current - nextHeight) < 1 ? current : nextHeight));
  };

  update();
  const observer = new ResizeObserver(update);
  observer.observe(target);
  return observer;
}

function BattlefieldLane({
  cards = [],
  cardHeight = 48,
  cardWidth = 62,
  clippedHeight = null,
  battlefieldSide,
  selectedObjectId,
  onCardClick,
  onCardPointerDown,
  onMobileCardActionMenu,
  onMobileCardLongPress,
  activatableMap,
  legalTargetObjectIds,
  className = "",
}) {
  const viewportHeight = clippedHeight ?? cardHeight;
  return (
    <div
      className={cn("mobile-battle-lane", className)}
      style={{ height: `${viewportHeight}px` }}
    >
      <div
        className="mobile-battle-lane-track"
        style={{ height: `${cardHeight}px` }}
      >
        <BattlefieldRow
          cards={cards}
          battlefieldSide={battlefieldSide}
          paperLayoutMode="single-row"
          layoutOverride={{
            rows: 1,
            cols: Math.max(1, cards.length),
            cardWidth,
            cardHeight,
            overlapPx: 0,
          }}
          selectedObjectId={selectedObjectId}
          onCardClick={onCardClick}
          onCardPointerDown={onCardPointerDown}
          onMobileCardActionMenu={onMobileCardActionMenu}
          onMobileCardLongPress={onMobileCardLongPress}
          activatableMap={activatableMap}
          legalTargetObjectIds={legalTargetObjectIds}
        />
      </div>
    </div>
  );
}

export default function MobileBattleScene({
  me,
  opponents,
  selectedObjectId,
  focusedStackObjectId = null,
  onInspect,
  onFocusStackObject = null,
  legalTargetPlayerIds = new Set(),
  legalTargetObjectIds = new Set(),
  mobileOpponentIndex = 0,
}) {
  const { registerPointerDown, shouldHandleClick } = usePointerClickGuard();
  const { state, dispatch } = useGame();
  const { combatModeRef } = useCombatArrows();
  const activeOpponent = opponents[Math.min(mobileOpponentIndex, Math.max(0, opponents.length - 1))] || opponents[0] || null;
  const opponentAccent = getPlayerAccent(state?.players || [], activeOpponent?.id);
  const visibleStackObjects = useMemo(
    () => getVisibleStackObjects(state),
    [state]
  );
  const stackPreviews = useMemo(
    () => Array.isArray(state?.stack_preview) ? state.stack_preview : [],
    [state?.stack_preview]
  );
  const showMobileStackTray = visibleStackObjects.length > 0 || stackPreviews.length > 0;
  const inlineStackPreview = useMemo(
    () => buildInlineStackPreview(visibleStackObjects, stackPreviews),
    [stackPreviews, visibleStackObjects]
  );
  const activatableMap = useMemo(
    () => buildActivatableMap(state?.decision),
    [state?.decision]
  );
  const controlBandRef = useRef(null);
  const [controlDockNode, setControlDockNode] = useState(null);
  const bottomBandRef = useRef(null);
  const [topbarHeight, setTopbarHeight] = useState(DEFAULT_TOPBAR_HEIGHT);
  const [controlBandHeight, setControlBandHeight] = useState(DEFAULT_CONTROL_BAND_HEIGHT);
  const [bottomBandHeight, setBottomBandHeight] = useState(DEFAULT_BOTTOM_BAND_HEIGHT);
  const [handExpanded, setHandExpanded] = useState(false);
  const [actionPopoverState, setActionPopoverState] = useState(null);
  const stackPressTimerRef = useRef(null);
  const stackLongPressTriggeredRef = useRef(false);
  const opponentRows = useMemo(
    () => partitionBattlefieldCards(activeOpponent?.battlefield || []),
    [activeOpponent?.battlefield]
  );
  const selfRows = useMemo(
    () => partitionBattlefieldCards(me?.battlefield || []),
    [me?.battlefield]
  );
  const layout = useMobileBattleLayout({
    topBandHeight: topbarHeight,
    controlBandHeight,
    collapsedHandRailHeight: bottomBandHeight,
    opponentFrontCount: opponentRows.frontCount,
    opponentBackCount: opponentRows.backCount,
    selfFrontCount: selfRows.frontCount,
    selfBackCount: selfRows.backCount,
  });
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
  const canPickTargets = state?.decision?.kind === "targets"
    && state?.decision?.player === state?.perspective;

  useEffect(() => {
    if (selectedObjectId != null) {
      setActionPopoverState(null);
      setHandExpanded(false);
    }
  }, [selectedObjectId]);

  useEffect(() => {
    const topbarEl = document.querySelector(".topbar-mobile-overlay");
    const controlBandEl = controlBandRef.current;
    const bottomBandEl = bottomBandRef.current;
    const observers = [
      measureElementHeight(topbarEl, DEFAULT_TOPBAR_HEIGHT, setTopbarHeight),
      measureElementHeight(controlBandEl, DEFAULT_CONTROL_BAND_HEIGHT, setControlBandHeight),
      measureElementHeight(bottomBandEl, DEFAULT_BOTTOM_BAND_HEIGHT, setBottomBandHeight),
    ].filter(Boolean);

    return () => {
      for (const observer of observers) observer.disconnect();
    };
  }, [showMobileStackTray, handExpanded, state?.decision?.kind]);

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

  useEffect(() => () => {
    if (stackPressTimerRef.current) {
      clearTimeout(stackPressTimerRef.current);
      stackPressTimerRef.current = null;
    }
  }, []);

  const closeInspector = useCallback(() => {
    onInspect?.(null);
  }, [onInspect]);

  const closeActionPopover = useCallback(() => {
    setActionPopoverState(null);
  }, []);

  const setControlBandElement = useCallback((node) => {
    controlBandRef.current = node;
  }, []);

  const setControlDockElement = useCallback((node) => {
    setControlDockNode(node);
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

  const handleCardInspect = useCallback((event, card) => {
    if (canPickTargets && !shouldHandleClick(event)) return;
    const candidateObjectIds = collectCardObjectIds(card);
    if (canPickTargets) {
      const matchedTargetId = candidateObjectIds.find((id) => legalTargetObjectIds.has(id));
      if (matchedTargetId != null) {
        window.dispatchEvent(
          new CustomEvent("ironsmith:target-choice", {
            detail: { target: { kind: "object", object: matchedTargetId } },
          })
        );
        return;
      }
    }
    onInspect?.(card.id, { candidateObjectIds });
  }, [canPickTargets, legalTargetObjectIds, onInspect, shouldHandleClick]);

  const handleCardTargetPointerDown = useCallback((event, card) => {
    if (!canPickTargets || !registerPointerDown(event)) return;
    const candidateObjectIds = collectCardObjectIds(card);
    const matchedTargetId = candidateObjectIds.find((id) => legalTargetObjectIds.has(id));
    if (matchedTargetId == null) return;
    event.preventDefault();
    event.stopPropagation();
    window.dispatchEvent(
      new CustomEvent("ironsmith:target-choice", {
        detail: { target: { kind: "object", object: matchedTargetId } },
      })
    );
  }, [canPickTargets, legalTargetObjectIds, registerPointerDown]);

  const dispatchOpponentPlayerChoice = useCallback(() => {
    if (!canPickTargets || !activeOpponent) return;
    const targetPlayer = legalTargetPlayerIds.has(Number(activeOpponent.id))
      ? Number(activeOpponent.id)
      : Number(activeOpponent.index);
    if (!Number.isFinite(targetPlayer)) return;
    window.dispatchEvent(
      new CustomEvent("ironsmith:target-choice", {
        detail: { target: { kind: "player", player: targetPlayer } },
      })
    );
  }, [activeOpponent, canPickTargets, legalTargetPlayerIds]);

  const dispatchSelfPlayerChoice = useCallback(() => {
    if (!canPickTargets || !me) return;
    const targetPlayer = legalTargetPlayerIds.has(Number(me.id))
      ? Number(me.id)
      : Number(me.index);
    if (!Number.isFinite(targetPlayer)) return;
    window.dispatchEvent(
      new CustomEvent("ironsmith:target-choice", {
        detail: { target: { kind: "player", player: targetPlayer } },
      })
    );
  }, [canPickTargets, legalTargetPlayerIds, me]);

  const handleOpponentBandClickCapture = useCallback((event) => {
    const currentCombatMode = combatModeRef.current;
    if (!activeOpponent || !currentCombatMode?.onTargetAreaClick || currentCombatMode.selectedAttacker == null) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    const hitElement = document.elementFromPoint(event.clientX, event.clientY);
    const cardEl = hitElement?.closest(".game-card[data-object-id]");
    const objectId = cardEl ? Number(cardEl.dataset.objectId) : null;
    const validTargets = currentCombatMode.validTargetPlayersByAttacker?.[Number(currentCombatMode.selectedAttacker)];
    const directId = Number(activeOpponent.id);
    const fallbackId = Number(activeOpponent.index);
    const playerId = validTargets?.has?.(directId) ? directId : fallbackId;
    currentCombatMode.onTargetAreaClick(playerId, objectId);
  }, [activeOpponent, combatModeRef]);

  const opponentTargetable = activeOpponent != null && (
    legalTargetPlayerIds.has(Number(activeOpponent.id))
    || legalTargetPlayerIds.has(Number(activeOpponent.index))
  );
  const selfTargetable = me != null && (
    legalTargetPlayerIds.has(Number(me.id))
    || legalTargetPlayerIds.has(Number(me.index))
  );
  const stackCount = visibleStackObjects.length > 0
    ? visibleStackObjects.length
    : stackPreviews.length;
  const topStackObject = visibleStackObjects[0] || null;
  const inlineStackIsActive = (
    topStackObject != null
    && focusedStackObjectId != null
    && String(focusedStackObjectId) === String(topStackObject.id)
  );
  const clearPendingStackLongPress = useCallback(() => {
    if (stackPressTimerRef.current) {
      clearTimeout(stackPressTimerRef.current);
      stackPressTimerRef.current = null;
    }
  }, []);
  const handleStackPointerDown = useCallback((event) => {
    if (!topStackObject) return;
    if (event.pointerType === "mouse" && event.button !== 0) return;
    clearPendingStackLongPress();
    stackLongPressTriggeredRef.current = false;
    stackPressTimerRef.current = window.setTimeout(() => {
      stackLongPressTriggeredRef.current = true;
      stackPressTimerRef.current = null;
      onInspect?.(topStackObject.inspect_object_id ?? topStackObject.id, {
        source: "stack",
        stackEntry: topStackObject,
      });
    }, 380);
  }, [clearPendingStackLongPress, onInspect, topStackObject]);
  const handleStackPointerUp = useCallback(() => {
    clearPendingStackLongPress();
  }, [clearPendingStackLongPress]);
  const handleStackPointerCancel = useCallback(() => {
    clearPendingStackLongPress();
  }, [clearPendingStackLongPress]);
  const handleInlineStackClick = useCallback((event) => {
    if (stackLongPressTriggeredRef.current) {
      stackLongPressTriggeredRef.current = false;
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    if (!topStackObject) return;
    setActionPopoverState(null);
    onFocusStackObject?.(topStackObject);
  }, [onFocusStackObject, topStackObject]);

  return (
    <main
      className="mobile-battle-scene table-gradient table-shell relative h-full min-h-0 overflow-hidden"
      data-drop-zone
      data-mobile-battle-scene
      style={{
        "--mobile-battle-card-width": `${layout.cardWidth}px`,
        "--mobile-battle-card-height": `${layout.cardHeight}px`,
        "--mobile-battle-top-status-height": `${layout.topStatusHeight}px`,
        "--mobile-battle-control-height": `${layout.controlBandHeight}px`,
        "--mobile-battle-bottom-band-height": `${layout.bottomBandHeight}px`,
        "--mobile-battle-opponent-band-height": `${layout.opponentBandHeight}px`,
        "--mobile-battle-self-back-visible-height": `${layout.selfBackVisibleHeight}px`,
        "--mobile-battle-scene-padding": `${layout.sidePadding}px`,
        "--mobile-battle-section-gap": `${layout.sectionGap}px`,
        "--mobile-battle-row-gap": `${layout.rowGap}px`,
      }}
    >
      <div className="mobile-battle-scene-vignette" aria-hidden="true" />
      <div className="mobile-battle-scene-runeband" aria-hidden="true" />

      <div className="mobile-battle-scene-layout">
        <div className="mobile-battle-top-spacer" aria-hidden="true" />

        <section
          className="mobile-battle-opponent-band"
          onClickCapture={handleOpponentBandClickCapture}
        >
          <BattlefieldLane
            cards={opponentRows.backCards}
            cardWidth={layout.cardWidth}
            cardHeight={layout.cardHeight}
            battlefieldSide="top"
            selectedObjectId={selectedObjectId}
            onCardClick={handleCardInspect}
            onCardPointerDown={handleCardTargetPointerDown}
            onMobileCardActionMenu={openObjectActions}
            onMobileCardLongPress={inspectHeldObject}
            activatableMap={activatableMap}
            legalTargetObjectIds={legalTargetObjectIds}
            className="mobile-battle-lane--opponent"
          />
          <BattlefieldLane
            cards={opponentRows.frontCards}
            cardWidth={layout.cardWidth}
            cardHeight={layout.cardHeight}
            battlefieldSide="top"
            selectedObjectId={selectedObjectId}
            onCardClick={handleCardInspect}
            onCardPointerDown={handleCardTargetPointerDown}
            onMobileCardActionMenu={openObjectActions}
            onMobileCardLongPress={inspectHeldObject}
            activatableMap={activatableMap}
            legalTargetObjectIds={legalTargetObjectIds}
            className="mobile-battle-lane--opponent"
          />
        </section>

        <section ref={setControlBandElement} className="mobile-battle-control-band">
          <div className="mobile-battle-control-band-inner">
            {showMobileStackTray ? (
              <button
                type="button"
                className={cn(
                  "mobile-battle-stack-button",
                  inlineStackIsActive && "is-active"
                )}
                data-arrow-anchor={topStackObject ? "stack" : undefined}
                data-object-id={topStackObject?.id ?? undefined}
                data-card-name={topStackObject?.name || inlineStackPreview?.name || "Stack"}
                aria-label={
                  inlineStackPreview
                    ? `Focus stack object: ${inlineStackPreview.name}, ${inlineStackPreview.subtitle}. Hold to inspect. ${stackCount} item${stackCount === 1 ? "" : "s"} on stack`
                    : `Focus stack. Hold to inspect. ${stackCount} item${stackCount === 1 ? "" : "s"} on stack`
                }
                onPointerDown={handleStackPointerDown}
                onPointerUp={handleStackPointerUp}
                onPointerCancel={handleStackPointerCancel}
                onPointerLeave={handleStackPointerCancel}
                onClick={handleInlineStackClick}
              >
                {inlineStackPreview?.artUrl ? (
                  <span className="mobile-battle-stack-button-art" aria-hidden="true">
                    <img
                      src={inlineStackPreview.artUrl}
                      alt=""
                      loading="lazy"
                      referrerPolicy="no-referrer"
                    />
                  </span>
                ) : null}
                <span className="mobile-battle-stack-button-copy">
                  <span className="mobile-battle-stack-button-title">
                    {inlineStackPreview?.name || "Stack"}
                  </span>
                  <span className="mobile-battle-stack-button-subtitle">
                    {inlineStackPreview?.subtitle || "Stack"}
                  </span>
                </span>
                <span className="mobile-battle-stack-button-count">{stackCount}</span>
              </button>
            ) : (
              <div className="mobile-battle-control-band-spacer" aria-hidden="true" />
            )}
            <div
              ref={setControlDockElement}
              className="mobile-battle-control-band-actions"
            />
          </div>

          {actionPopoverState ? (
            <MobileDecisionSheet
              eyebrow="Your Action"
              title={actionPopoverState.cardName}
              subtitle={`${actionPopoverState.actions.length} action${actionPopoverState.actions.length === 1 ? "" : "s"}`}
              onClose={closeActionPopover}
              closeLabel="Close action menu"
              inline={false}
              className="mobile-decision-sheet--action-list mobile-battle-action-menu-sheet"
              bodyClassName="mobile-decision-sheet-body--action-list mobile-battle-action-menu-sheet-body"
            >
              <MobileDecisionActionList
                items={actionPopoverState.actions.map((action) => ({
                  key: String(action.index),
                  label: stripActionPrefix(action.label || "Action"),
                  onClick: () => handlePopoverAction(action),
                }))}
                emptyText="No available actions."
              />
            </MobileDecisionSheet>
          ) : null}
        </section>

        <section className="mobile-battle-self-band">
          <BattlefieldLane
            cards={selfRows.frontCards}
            cardWidth={layout.cardWidth}
            cardHeight={layout.cardHeight}
            battlefieldSide="bottom"
            selectedObjectId={selectedObjectId}
            onCardClick={handleCardInspect}
            onCardPointerDown={handleCardTargetPointerDown}
            onMobileCardActionMenu={openObjectActions}
            onMobileCardLongPress={inspectHeldObject}
            activatableMap={activatableMap}
            legalTargetObjectIds={legalTargetObjectIds}
            className="mobile-battle-lane--self-front"
          />
          <BattlefieldLane
            cards={selfRows.backCards}
            cardWidth={layout.cardWidth}
            cardHeight={layout.cardHeight}
            clippedHeight={layout.selfBackVisibleHeight}
            battlefieldSide="bottom"
            selectedObjectId={selectedObjectId}
            onCardClick={handleCardInspect}
            onCardPointerDown={handleCardTargetPointerDown}
            onMobileCardActionMenu={openObjectActions}
            onMobileCardLongPress={inspectHeldObject}
            activatableMap={activatableMap}
            legalTargetObjectIds={legalTargetObjectIds}
            className="mobile-battle-lane--self-back"
          />
        </section>

        <footer ref={bottomBandRef} className="mobile-battle-bottom-utility">
          <button
            type="button"
            className={cn(
              "mobile-battle-self-identity-bar",
              selfTargetable && canPickTargets && "is-targetable"
            )}
            onPointerDown={(event) => {
              if (!registerPointerDown(event)) return;
              event.preventDefault();
              event.stopPropagation();
              dispatchSelfPlayerChoice();
            }}
            onClick={(event) => {
              if (!shouldHandleClick(event)) return;
              event.preventDefault();
              event.stopPropagation();
              dispatchSelfPlayerChoice();
            }}
          >
            <span className="mobile-battle-self-identity-name">{me?.name || "You"}</span>
            <span className="mobile-battle-self-identity-meta">
              H {zoneCount(me, "hand")} G {zoneCount(me, "graveyard")} X {zoneCount(me, "exile")} D {zoneCount(me, "library")}
            </span>
            <span className="mobile-battle-inline-life">{me?.life ?? 0}</span>
          </button>

          <div
            className={cn("mobile-battle-hand-rail", handExpanded && "is-open")}
            onClick={(event) => {
              if (event.target.closest(".game-card.hand-card")) return;
              setHandExpanded((current) => !current);
            }}
          >
            <div className="mobile-battle-hand-rail-viewport">
              <HandZone
                player={me}
                selectedObjectId={selectedObjectId}
                onInspect={onInspect}
                isExpanded
                layout="mobile-fan"
              />
            </div>
          </div>
        </footer>
      </div>

      <DecisionPopupLayer
        selectedObjectId={selectedObjectId}
        mobileBattle
        mobileBattlePortalTarget={controlDockNode}
        mobileBattleDockInline
        mobileBattleDockHidden={actionPopoverState != null}
      />

      {handExpanded ? (
        <>
          <button
            type="button"
            className="mobile-battle-hand-overlay-backdrop"
            aria-label="Close hand"
            onClick={() => setHandExpanded(false)}
          />
          <section className="mobile-battle-hand-overlay">
            <button
              type="button"
              className="mobile-battle-hand-overlay-close"
              aria-label="Close hand"
              onClick={() => setHandExpanded(false)}
            >
              <X className="size-4" />
            </button>
            <div className="mobile-battle-hand-overlay-header">
              <span className="mobile-battle-hand-overlay-title">Hand</span>
              <span className="mobile-battle-hand-overlay-count">{zoneCount(me, "hand")} cards</span>
            </div>
            <div className="mobile-battle-hand-overlay-body">
              <HandZone
                player={me}
                selectedObjectId={selectedObjectId}
                onInspect={onInspect}
                isExpanded
                layout="mobile-fan"
              />
            </div>
          </section>
        </>
      ) : null}

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

      {activeOpponent && canPickTargets ? (
        <button
          type="button"
          className={cn(
            "mobile-battle-opponent-target-chip",
            opponentTargetable && canPickTargets && "is-targetable"
          )}
          data-player-target={activeOpponent.index ?? activeOpponent.id}
          data-player-target-name={activeOpponent.id ?? activeOpponent.index}
          style={opponentAccent ? { "--player-accent": opponentAccent.hex } : undefined}
          onPointerDown={(event) => {
            if (!registerPointerDown(event)) return;
            event.preventDefault();
            event.stopPropagation();
            dispatchOpponentPlayerChoice();
          }}
          onClick={(event) => {
            if (!shouldHandleClick(event)) return;
            event.preventDefault();
            event.stopPropagation();
            dispatchOpponentPlayerChoice();
          }}
        >
          <span className="mobile-battle-opponent-target-name">{activeOpponent.name}</span>
          <span className="mobile-battle-opponent-target-meta">
            H {zoneCount(activeOpponent, "hand")} G {zoneCount(activeOpponent, "graveyard")} D {zoneCount(activeOpponent, "library")}
          </span>
          <span className="mobile-battle-inline-life">{activeOpponent.life}</span>
        </button>
      ) : null}

    </main>
  );
}
