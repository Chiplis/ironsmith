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
const MOBILE_CARD_TAP_MAX_DISTANCE_SQ = 16 * 16;
const MOBILE_OPPONENT_CARD_HIT_SLOP_X = 14;
const MOBILE_OPPONENT_CARD_HIT_SLOP_Y = 18;

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

function collectActivatableActionsForCard(card, activatableMap) {
  if (!card || !activatableMap) return [];

  const actions = [];
  const seenActionIndices = new Set();

  for (const objectId of collectCardObjectIds(card)) {
    if (!activatableMap.has(objectId)) continue;
    for (const action of activatableMap.get(objectId) || []) {
      const actionIndex = Number(action?.index);
      if (Number.isFinite(actionIndex)) {
        if (seenActionIndices.has(actionIndex)) continue;
        seenActionIndices.add(actionIndex);
      }
      actions.push(action);
    }
  }

  return actions;
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
  const opponentTapRef = useRef(null);
  const stackPressTimerRef = useRef(null);
  const stackLongPressTriggeredRef = useRef(false);
  const inspectSuppressUntilRef = useRef(0);
  const inspectOverlayRef = useRef(null);
  const inspectLockReleaseTimerRef = useRef(null);
  const [inspectInteractionLockActive, setInspectInteractionLockActive] = useState(false);
  const opponentRows = useMemo(
    () => partitionBattlefieldCards(activeOpponent?.battlefield || []),
    [activeOpponent?.battlefield]
  );
  const selfRows = useMemo(
    () => partitionBattlefieldCards(me?.battlefield || []),
    [me?.battlefield]
  );
  const opponentCardById = useMemo(() => {
    const index = new Map();
    for (const card of activeOpponent?.battlefield || []) {
      if (card?.id != null) {
        index.set(String(card.id), card);
      }
    }
    return index;
  }, [activeOpponent?.battlefield]);
  const opponentBandSelector = ".mobile-battle-opponent-band";
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
  const legalSelectableObjectIds = useMemo(() => {
    const ids = new Set();
    const decision = state?.decision || null;
    if (!decision) return ids;

    if (decision.kind === "targets") {
      for (const req of decision.requirements || []) {
        for (const target of req.legal_targets || []) {
          if (target?.kind === "object" && target.object != null) {
            ids.add(Number(target.object));
          }
        }
      }
      return ids;
    }

    if (decision.kind === "select_objects") {
      for (const candidate of decision.candidates || []) {
        if (candidate?.legal === false || candidate?.id == null) continue;
        ids.add(Number(candidate.id));
      }
    }

    return ids;
  }, [state?.decision]);
  const canPickTargets = state?.decision?.kind === "targets"
    && state?.decision?.player === state?.perspective;
  const canPickBattlefieldObjects = (
    (state?.decision?.kind === "targets" || state?.decision?.kind === "select_objects")
    && state?.decision?.player === state?.perspective
  );
  const inspectorOpen = selectedObjectId != null;

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

  useEffect(() => {
    if (!actionPopoverState) return;
    setHandExpanded(false);
  }, [actionPopoverState]);

  useEffect(() => () => {
    if (stackPressTimerRef.current) {
      clearTimeout(stackPressTimerRef.current);
      stackPressTimerRef.current = null;
    }
  }, []);

  useEffect(() => () => {
    opponentTapRef.current = null;
  }, []);

  useEffect(() => () => {
    if (inspectLockReleaseTimerRef.current != null && typeof window !== "undefined") {
      window.clearTimeout(inspectLockReleaseTimerRef.current);
      inspectLockReleaseTimerRef.current = null;
    }
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") return undefined;

    if (inspectLockReleaseTimerRef.current != null) {
      window.clearTimeout(inspectLockReleaseTimerRef.current);
      inspectLockReleaseTimerRef.current = null;
    }

    if (inspectorOpen) {
      setInspectInteractionLockActive(true);
      return undefined;
    }

    if (!inspectInteractionLockActive) return undefined;

    const remainingLockMs = Math.max(0, inspectSuppressUntilRef.current - performance.now());
    if (remainingLockMs <= 0) {
      setInspectInteractionLockActive(false);
      return undefined;
    }

    inspectLockReleaseTimerRef.current = window.setTimeout(() => {
      inspectLockReleaseTimerRef.current = null;
      setInspectInteractionLockActive(false);
    }, remainingLockMs);

    return () => {
      if (inspectLockReleaseTimerRef.current != null) {
        window.clearTimeout(inspectLockReleaseTimerRef.current);
        inspectLockReleaseTimerRef.current = null;
      }
    };
  }, [inspectInteractionLockActive, inspectorOpen]);

  useEffect(() => {
    if (!inspectInteractionLockActive || typeof document === "undefined") return undefined;

    const blockBackgroundInteraction = (event) => {
      const overlayNode = inspectOverlayRef.current;
      const eventTarget = event.target;
      if (overlayNode && eventTarget instanceof Node && overlayNode.contains(eventTarget)) {
        return;
      }
      if (typeof event.stopImmediatePropagation === "function") {
        event.stopImmediatePropagation();
      }
      event.stopPropagation();
      if (event.cancelable) {
        event.preventDefault();
      }
    };

    const listenerOptions = { capture: true, passive: false };
    const eventTypes = ["pointerdown", "pointerup", "click", "touchstart", "touchend", "mousedown", "mouseup"];

    for (const eventType of eventTypes) {
      document.addEventListener(eventType, blockBackgroundInteraction, listenerOptions);
    }

    return () => {
      for (const eventType of eventTypes) {
        document.removeEventListener(eventType, blockBackgroundInteraction, listenerOptions);
      }
    };
  }, [inspectInteractionLockActive]);

  const closeInspector = useCallback(() => {
    inspectSuppressUntilRef.current = performance.now() + 320;
    setInspectInteractionLockActive(true);
    onInspect?.(null);
  }, [onInspect]);

  const requestInspectObject = useCallback((objectId, meta = undefined) => {
    if (performance.now() < inspectSuppressUntilRef.current) return false;
    if (objectId == null) {
      onInspect?.(null);
      return true;
    }
    if (selectedObjectId != null) return false;
    onInspect?.(objectId, meta);
    return true;
  }, [onInspect, selectedObjectId]);

  const closeActionPopover = useCallback(() => {
    setActionPopoverState(null);
  }, []);

  const setControlBandElement = useCallback((node) => {
    controlBandRef.current = node;
  }, []);

  const setControlDockElement = useCallback((node) => {
    setControlDockNode(node);
  }, []);

  const openObjectActions = useCallback(({ card, actions = null, anchorRect = null }) => {
    if (selectedObjectId != null) return false;
    const resolvedActions = Array.isArray(actions)
      ? actions
      : collectActivatableActionsForCard(card, activatableMap);
    if (resolvedActions.length === 0 || state?.decision?.kind !== "priority") {
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
        actions: resolvedActions,
        decisionIdentity,
      };
    });
    onInspect?.(null);
    return true;
  }, [activatableMap, decisionIdentity, onInspect, selectedObjectId, state?.decision?.kind]);

  const inspectHeldObject = useCallback(({ card }) => {
    closeActionPopover();
    requestInspectObject(card?.id ?? null);
  }, [closeActionPopover, requestInspectObject]);

  const handlePopoverAction = useCallback((action) => {
    if (!action) return;
    dispatch(
      { type: "priority_action", action_index: action.index },
      action.label
    );
    closeActionPopover();
  }, [closeActionPopover, dispatch]);

  const handleCardInspect = useCallback((event, card) => {
    if (canPickBattlefieldObjects && !shouldHandleClick(event)) return;
    const candidateObjectIds = collectCardObjectIds(card);
    if (canPickBattlefieldObjects) {
      const matchedTargetId = candidateObjectIds.find((id) => legalSelectableObjectIds.has(id));
      if (matchedTargetId != null) {
        const eventName = state?.decision?.kind === "select_objects"
          ? "ironsmith:select-object-choice"
          : "ironsmith:target-choice";
        const detail = state?.decision?.kind === "select_objects"
          ? { objectId: matchedTargetId }
          : { target: { kind: "object", object: matchedTargetId } };
        window.dispatchEvent(new CustomEvent(eventName, { detail }));
        return;
      }
    }
    if (selectedObjectId != null) return;
    requestInspectObject(card.id, { candidateObjectIds });
  }, [canPickBattlefieldObjects, legalSelectableObjectIds, requestInspectObject, selectedObjectId, shouldHandleClick, state?.decision?.kind]);

  const opponentCardFromPointerEvent = useCallback((event) => {
    const withinExpandedRect = (rect, x, y) => (
      x >= (rect.left - MOBILE_OPPONENT_CARD_HIT_SLOP_X)
      && x <= (rect.right + MOBILE_OPPONENT_CARD_HIT_SLOP_X)
      && y >= (rect.top - MOBILE_OPPONENT_CARD_HIT_SLOP_Y)
      && y <= (rect.bottom + MOBILE_OPPONENT_CARD_HIT_SLOP_Y)
    );

    const cardFromGeometry = () => {
      if (typeof document === "undefined") return null;
      if (!Number.isFinite(event?.clientX) || !Number.isFinite(event?.clientY)) return null;
      const cardNodes = document.querySelectorAll(
        `${opponentBandSelector} .game-card[data-object-id]`
      );
      let bestMatch = null;
      let bestDistanceSq = Infinity;

      for (const node of cardNodes) {
        const rect = node.getBoundingClientRect();
        if (!withinExpandedRect(rect, event.clientX, event.clientY)) {
          continue;
        }
        const objectId = node.dataset?.objectId;
        if (!objectId) continue;
        const card = opponentCardById.get(String(objectId));
        if (!card) continue;
        const centerX = rect.left + (rect.width / 2);
        const centerY = rect.top + (rect.height / 2);
        const distanceSq = ((event.clientX - centerX) ** 2) + ((event.clientY - centerY) ** 2);
        if (distanceSq < bestDistanceSq) {
          bestMatch = card;
          bestDistanceSq = distanceSq;
        }
      }

      return bestMatch;
    };

    const tryElementFromPointOffsets = () => {
      if (typeof document === "undefined") return null;
      if (!Number.isFinite(event?.clientX) || !Number.isFinite(event?.clientY)) return null;

      const sampleOffsets = [
        [0, 0],
        [-12, 0],
        [12, 0],
        [0, -12],
        [0, 12],
        [-10, -10],
        [10, -10],
        [-10, 10],
        [10, 10],
      ];

      for (const [offsetX, offsetY] of sampleOffsets) {
        const hitElement = document.elementFromPoint(event.clientX + offsetX, event.clientY + offsetY);
        const hitCardEl = hitElement?.closest?.(".game-card[data-object-id]");
        if (!hitCardEl) continue;
        const id = hitCardEl.dataset?.objectId;
        if (!id) continue;
        const card = opponentCardById.get(String(id));
        if (card) return card;
      }

      return null;
    };

    // Prefer the event composed path when available (handles Shadow DOM/layered elements)
    const tryFromPath = () => {
      const path = typeof event.composedPath === "function" ? event.composedPath() : (event.path || null);
      if (Array.isArray(path)) {
        for (const node of path) {
          if (!(node instanceof Element)) continue;
          const cardEl = node.closest?.(".game-card[data-object-id]") || (node.matches?.(".game-card[data-object-id]") ? node : null);
          if (cardEl) return cardEl;
        }
      }
      return null;
    };

    const pathCardEl = tryFromPath();
    if (pathCardEl) {
      const id = pathCardEl.dataset?.objectId;
      if (id) {
        const card = opponentCardById.get(String(id));
        if (card) return card;
      }
    }

    const targetCardEl = event.target instanceof Element
      ? event.target.closest(".game-card[data-object-id]")
      : null;
    if (targetCardEl) {
      const id = targetCardEl.dataset?.objectId;
      if (id) {
        const card = opponentCardById.get(String(id));
        if (card) return card;
      }
    }

    const sampledCard = tryElementFromPointOffsets();
    if (sampledCard) return sampledCard;

    return cardFromGeometry();
  }, [opponentBandSelector, opponentCardById]);

  const handleOpponentBandPointerDownCapture = useCallback((event) => {
    if (canPickBattlefieldObjects) {
      opponentTapRef.current = null;
      return;
    }
    if (event.button != null && event.button !== 0) {
      opponentTapRef.current = null;
      return;
    }

    const card = opponentCardFromPointerEvent(event);
    if (!card) {
      opponentTapRef.current = null;
      return;
    }

    opponentTapRef.current = {
      pointerId: event.pointerId,
      cardId: String(card.id),
      startX: event.clientX,
      startY: event.clientY,
    };
  }, [canPickBattlefieldObjects, opponentCardFromPointerEvent]);

  const handleOpponentBandPointerUpCapture = useCallback((event) => {
    const pendingTap = opponentTapRef.current;
    opponentTapRef.current = null;

    if (canPickBattlefieldObjects || !pendingTap) return;
    if (pendingTap.pointerId != null && event.pointerId !== pendingTap.pointerId) return;

    const deltaX = event.clientX - pendingTap.startX;
    const deltaY = event.clientY - pendingTap.startY;
    if (((deltaX * deltaX) + (deltaY * deltaY)) > MOBILE_CARD_TAP_MAX_DISTANCE_SQ) return;

    const resolvedCard = (
      pendingTap.cardId != null
        ? opponentCardById.get(String(pendingTap.cardId)) || null
        : null
    ) || opponentCardFromPointerEvent(event);

    if (!resolvedCard) {
      return;
    }

    const didOpenMenu = openObjectActions({
      card: resolvedCard,
      anchorRect: event.target instanceof Element
        ? event.target.closest(".game-card[data-object-id]")?.getBoundingClientRect?.() || null
        : null,
    });
    if (didOpenMenu) {
      event.preventDefault();
      event.stopPropagation();
    }
  }, [canPickBattlefieldObjects, openObjectActions, opponentCardFromPointerEvent, opponentCardById]);

  const handleOpponentBandPointerCancelCapture = useCallback(() => {
    opponentTapRef.current = null;
  }, []);

  const handleOpponentBandPointerLeave = useCallback((event) => {
    if (event.pointerType === "mouse") {
      opponentTapRef.current = null;
    }
  }, []);

  const handleCardTargetPointerDown = useCallback((event, card) => {
    if (!canPickBattlefieldObjects || !registerPointerDown(event)) return;
    const candidateObjectIds = collectCardObjectIds(card);
    const matchedTargetId = candidateObjectIds.find((id) => legalSelectableObjectIds.has(id));
    if (matchedTargetId == null) return;
    event.preventDefault();
    event.stopPropagation();
    const eventName = state?.decision?.kind === "select_objects"
      ? "ironsmith:select-object-choice"
      : "ironsmith:target-choice";
    const detail = state?.decision?.kind === "select_objects"
      ? { objectId: matchedTargetId }
      : { target: { kind: "object", object: matchedTargetId } };
    window.dispatchEvent(new CustomEvent(eventName, { detail }));
  }, [canPickBattlefieldObjects, legalSelectableObjectIds, registerPointerDown, state?.decision?.kind]);

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

    // Prefer composed path when available (handles Shadow DOM/layered elements)
    const tryFromPath = () => {
      const path = typeof event.composedPath === "function" ? event.composedPath() : (event.path || null);
      if (Array.isArray(path)) {
        for (const node of path) {
          if (!(node instanceof Element)) continue;
          const cardEl = node.closest?.(".game-card[data-object-id]") || (node.matches?.(".game-card[data-object-id]") ? node : null);
          if (cardEl) return cardEl;
        }
      }
      return null;
    };

    const pathCardEl = tryFromPath();
    if (pathCardEl) return;

    const targetCardEl = event.target instanceof Element
      ? event.target.closest(".game-card[data-object-id]")
      : null;
    if (targetCardEl) {
      return;
    }

    const hitElement = document.elementFromPoint(event.clientX, event.clientY);
    const hitCardEl = hitElement?.closest(".game-card[data-object-id]");
    if (hitCardEl) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    const validTargets = currentCombatMode.validTargetPlayersByAttacker?.[Number(currentCombatMode.selectedAttacker)];
    const directId = Number(activeOpponent.id);
    const fallbackId = Number(activeOpponent.index);
    const playerId = validTargets?.has?.(directId) ? directId : fallbackId;
    currentCombatMode.onTargetAreaClick(playerId, null);
  }, [activeOpponent, combatModeRef]);
  const opponentBandCaptureEnabled = Boolean(
    activeOpponent
    && combatModeRef.current?.onTargetAreaClick
    && combatModeRef.current?.selectedAttacker != null
  );

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
      requestInspectObject(topStackObject.inspect_object_id ?? topStackObject.id, {
        source: "stack",
        stackEntry: topStackObject,
      });
    }, 380);
  }, [clearPendingStackLongPress, requestInspectObject, topStackObject]);
  const handleStackPointerUp = useCallback(() => {
    clearPendingStackLongPress();
  }, [clearPendingStackLongPress]);
  const handleStackPointerCancel = useCallback(() => {
    clearPendingStackLongPress();
  }, [clearPendingStackLongPress]);
  const handleInlineStackClick = useCallback((event) => {
    if (inspectorOpen) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    if (stackLongPressTriggeredRef.current) {
      stackLongPressTriggeredRef.current = false;
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    if (!topStackObject) return;
    setActionPopoverState(null);
    onFocusStackObject?.(topStackObject);
  }, [inspectorOpen, onFocusStackObject, topStackObject]);

  return (
    <main
      className="mobile-battle-scene table-gradient table-shell relative h-full min-h-0 overflow-hidden"
      data-drop-zone
      data-mobile-battle-scene
      data-inspector-open={inspectInteractionLockActive ? "true" : "false"}
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
          onPointerDownCapture={handleOpponentBandPointerDownCapture}
          onPointerUpCapture={handleOpponentBandPointerUpCapture}
          onPointerCancelCapture={handleOpponentBandPointerCancelCapture}
          onPointerLeave={handleOpponentBandPointerLeave}
          onClickCapture={opponentBandCaptureEnabled ? handleOpponentBandClickCapture : undefined}
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
            legalTargetObjectIds={legalSelectableObjectIds}
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
            legalTargetObjectIds={legalSelectableObjectIds}
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
            legalTargetObjectIds={legalSelectableObjectIds}
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
            legalTargetObjectIds={legalSelectableObjectIds}
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

          {!actionPopoverState ? (
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
                  onInspect={requestInspectObject}
                  isExpanded
                  layout="mobile-fan"
                />
              </div>
            </div>
          ) : null}
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
                onInspect={requestInspectObject}
                isExpanded
                layout="mobile-fan"
              />
            </div>
          </section>
        </>
      ) : null}

      {inspectorOpen ? (
        <div
          ref={inspectOverlayRef}
          className="mobile-battle-inspect-overlay"
          data-mobile-hand-drop-target="inspector"
          role="dialog"
          aria-modal="true"
          aria-label="Card inspector"
          onPointerDown={(event) => {
            event.stopPropagation();
          }}
          onPointerUp={(event) => {
            event.stopPropagation();
          }}
          onClick={(event) => {
            event.stopPropagation();
            if (event.target === event.currentTarget) {
              closeInspector();
            }
          }}
        >
          <div
            className="mobile-battle-inspect-overlay-backdrop"
            aria-hidden="true"
          />
          <div
            className="mobile-battle-inspect-overlay-shell"
            onClick={(event) => event.stopPropagation()}
            onPointerDown={(event) => {
              event.stopPropagation();
            }}
            onPointerUp={(event) => {
              event.stopPropagation();
            }}
          >
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
