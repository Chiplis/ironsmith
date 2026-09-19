import useUiText from "@/i18n/useUiText";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { X } from "lucide-react";
import { useGame } from "@/context/GameContext";
import { useCombatArrows } from "@/context/useCombatArrows";
import DecisionPopupLayer from "@/components/overlays/DecisionPopupLayer";
import ActionPopover from "@/components/overlays/ActionPopover";
import HoverArtOverlay from "@/components/right-rail/HoverArtOverlay";
import useModalFocus from "@/hooks/useModalFocus";
import MobileZoneBrowser from "@/components/board/mobile/MobileZoneBrowser";
import useMobileBattleLayout from "@/hooks/useMobileBattleLayout";
import {
  MOBILE_OPPONENT_HUD_HEIGHT_PX,
  MOBILE_CONTROL_BAND_HEIGHT_PX,
  MOBILE_HAND_PEEK_HEIGHT_PX,
  MOBILE_STACK_RAIL_WIDTH_PX,
} from "@/lib/mobile-battle-layout";
import { getVisibleStackObjects } from "@/lib/stack-targets";
import { sameActionRef } from "@/lib/sync-commands";
import { partitionArenaBattlefield } from "@/lib/mobile-arena";
import { usePointerClickGuard } from "@/lib/usePointerClickGuard";
import { samePlayerId } from "@/lib/player-display";
import { requestObjectSelection } from "@/lib/object-selection";

import MobileOpponentHud from "@/components/board/mobile/MobileOpponentHud";
import MobileSelfHud from "@/components/board/mobile/MobileSelfHud";
import PhaseTrack from "@/components/board/PhaseTrack";
import MobileTurnActionStack from "@/components/board/mobile/MobileTurnActionStack";
import MobileBattlefieldBand from "@/components/board/mobile/MobileBattlefieldBand";
import MobileHandFan from "@/components/board/mobile/MobileHandFan";
import MobileHandFullscreen from "@/components/board/mobile/MobileHandFullscreen";
import MobileViewToggle from "@/components/board/mobile/MobileViewToggle";
import MobileStackRail from "@/components/board/mobile/MobileStackRail";

const DEFAULT_TOPBAR_HEIGHT = MOBILE_OPPONENT_HUD_HEIGHT_PX;
const DEFAULT_CONTROL_BAND_HEIGHT = MOBILE_CONTROL_BAND_HEIGHT_PX;
const DEFAULT_HAND_PEEK_HEIGHT = MOBILE_HAND_PEEK_HEIGHT_PX;
const MOBILE_CARD_TAP_MAX_DISTANCE_SQ = 16 * 16;
const MOBILE_OPPONENT_CARD_HIT_SLOP_X = 14;
const MOBILE_OPPONENT_CARD_HIT_SLOP_Y = 18;

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
  const seen = new Set();
  for (const objectId of collectCardObjectIds(card)) {
    if (!activatableMap.has(objectId)) continue;
    for (const action of activatableMap.get(objectId) || []) {
      const idx = Number(action?.index);
      if (Number.isFinite(idx)) {
        if (seen.has(idx)) continue;
        seen.add(idx);
      }
      actions.push(action);
    }
  }
  return actions;
}

function buildActivatableMap(decision, perspective) {
  const map = new Map();
  if (
    decision?.kind !== "priority"
    || !samePlayerId(decision.player, perspective)
    || !Array.isArray(decision.actions)
  ) return map;
  for (const action of decision.actions) {
    if (
      (action.kind === "activate_ability"
        || action.kind === "activate_mana_ability"
        || action.kind === "untap_land")
      && action.object_id != null
    ) {
      const objectId = Number(action.object_id);
      if (!map.has(objectId)) map.set(objectId, []);
      map.get(objectId).push(action);
    }
  }
  return map;
}

function measureElementHeight(target, fallback, setHeight, dimension = "height") {
  if (!target) {
    setHeight((current) => current || fallback);
    return null;
  }
  const update = () => {
    const next = Math.ceil(target.getBoundingClientRect()[dimension] || fallback);
    setHeight((current) => (Math.abs(current - next) < 1 ? current : next));
  };
  update();
  const observer = new ResizeObserver(update);
  observer.observe(target);
  return observer;
}

export default function MobileBattleScene({
  me,
  opponents,
  selectedObjectId,
  focusedStackObjectId = null,
  onInspect,
  onFocusStackObject = null,
  onOpenDecklist,
  legalTargetPlayerIds = new Set(),
  legalTargetObjectIds = new Set(),
  mobileOpponentIndex = 0,
  setMobileOpponentIndex,
  mobileViewMode = "battlefield",
  setMobileViewMode,
}) {
  const ui = useUiText();
  void legalTargetObjectIds; // legality already derived from decision; keep prop for parity
  const { state, dispatch, cancelDecision } = useGame();
  const { combatModeRef } = useCombatArrows();
  const { registerPointerDown, shouldHandleClick } = usePointerClickGuard();

  const opponentCount = opponents?.length || 0;
  const cycleEnabled = opponentCount >= 2;
  const safeIndex = Math.min(mobileOpponentIndex, Math.max(0, opponentCount - 1));
  const activeOpponent = opponentCount > 0 ? opponents[safeIndex] : null;
  const previousOpponent = cycleEnabled
    ? opponents[(safeIndex - 1 + opponentCount) % opponentCount]
    : null;
  const nextOpponent = cycleEnabled
    ? opponents[(safeIndex + 1) % opponentCount]
    : null;

  const visibleStackObjects = useMemo(() => getVisibleStackObjects(state), [state]);
  const stackVisible = visibleStackObjects.length > 0;
  const opponentRows = useMemo(
    () => partitionArenaBattlefield(activeOpponent?.battlefield || []),
    [activeOpponent?.battlefield]
  );
  const selfRows = useMemo(
    () => partitionArenaBattlefield(me?.battlefield || []),
    [me?.battlefield]
  );
  const opponentCardById = useMemo(() => {
    const idx = new Map();
    for (const card of activeOpponent?.battlefield || []) {
      if (card?.id != null) idx.set(String(card.id), card);
    }
    return idx;
  }, [activeOpponent?.battlefield]);

  const activatableMap = useMemo(
    () => buildActivatableMap(state?.decision, state?.perspective),
    [state?.decision, state?.perspective]
  );

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
          if (target?.kind === "object" && target.object != null) ids.add(Number(target.object));
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
    && samePlayerId(state?.decision?.player, state?.perspective);
  const canPickBattlefieldObjects = (
    (state?.decision?.kind === "targets" || state?.decision?.kind === "select_objects")
    && samePlayerId(state?.decision?.player, state?.perspective)
  );
  // A spell played from the full hand must reveal the board for its targets.
  // Run on decision transitions so the player can still reopen the hand to read it.
  useEffect(() => {
    if (canPickBattlefieldObjects) setMobileViewMode?.("battlefield");
  }, [canPickBattlefieldObjects, decisionIdentity, setMobileViewMode]);
  const inspectorOpen = selectedObjectId != null;

  const opponentTargetable = activeOpponent != null && (
    legalTargetPlayerIds.has(Number(activeOpponent.id))
    || legalTargetPlayerIds.has(Number(activeOpponent.index))
  );
  const selfTargetable = me != null && (
    legalTargetPlayerIds.has(Number(me.id))
    || legalTargetPlayerIds.has(Number(me.index))
  );

  // --- Layout solver hookup -------------------------------------------------
  const sceneRef = useRef(null);
  const opponentHudRef = useRef(null);
  const controlBandRef = useRef(null);
  const selfHudRef = useRef(null);
  const actionControlsRef = useRef(null);
  const [opponentWidth, setOpponentWidth] = useState(170);
  const [selfWidth, setSelfWidth] = useState(150);
  const [actionWidth, setActionWidth] = useState(220);
  const [actionStackElement, setActionStackElement] = useState(null);

  const [topbarHeight, setTopbarHeight] = useState(DEFAULT_TOPBAR_HEIGHT);
  const [controlBandHeight, setControlBandHeight] = useState(DEFAULT_CONTROL_BAND_HEIGHT);

  const layout = useMobileBattleLayout({
    topBandHeight: topbarHeight,
    controlBandHeight,
    handPeekHeight: DEFAULT_HAND_PEEK_HEIGHT,
    stackVisible,
    stackRailWidth: MOBILE_STACK_RAIL_WIDTH_PX,
  }, sceneRef);

  useEffect(() => {
    const observers = [
      measureElementHeight(opponentHudRef.current, DEFAULT_TOPBAR_HEIGHT, setTopbarHeight),
      measureElementHeight(controlBandRef.current, DEFAULT_CONTROL_BAND_HEIGHT, setControlBandHeight),
      measureElementHeight(opponentHudRef.current, 170, setOpponentWidth, "width"),
      measureElementHeight(selfHudRef.current, 150, setSelfWidth, "width"),
      measureElementHeight(actionControlsRef.current, 220, setActionWidth, "width"),
    ].filter(Boolean);
    return () => {
      for (const observer of observers) observer.disconnect();
    };
  }, [stackVisible, mobileViewMode]);

  // --- Inspector overlay machinery (salvaged) ------------------------------
  const inspectSuppressUntilRef = useRef(0);
  const inspectOverlayRef = useModalFocus(() => closeInspector(), inspectorOpen);
  const inspectLockReleaseTimerRef = useRef(null);
  const [inspectInteractionLockActive, setInspectInteractionLockActive] = useState(false);

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
    const remaining = Math.max(0, inspectSuppressUntilRef.current - performance.now());
    if (remaining <= 0) {
      setInspectInteractionLockActive(false);
      return undefined;
    }
    inspectLockReleaseTimerRef.current = window.setTimeout(() => {
      inspectLockReleaseTimerRef.current = null;
      setInspectInteractionLockActive(false);
    }, remaining);
    return () => {
      if (inspectLockReleaseTimerRef.current != null) {
        window.clearTimeout(inspectLockReleaseTimerRef.current);
        inspectLockReleaseTimerRef.current = null;
      }
    };
  }, [inspectInteractionLockActive, inspectorOpen]);

  useEffect(() => {
    if (!inspectInteractionLockActive || typeof document === "undefined") return undefined;
    const blockBackground = (event) => {
      const overlayNode = inspectOverlayRef.current;
      const target = event.target;
      if (overlayNode && target instanceof Node && overlayNode.contains(target)) return;
      if (typeof event.stopImmediatePropagation === "function") event.stopImmediatePropagation();
      event.stopPropagation();
      if (event.cancelable) event.preventDefault();
    };
    const opts = { capture: true, passive: false };
    const types = ["pointerdown", "pointerup", "click", "touchstart", "touchend", "mousedown", "mouseup"];
    for (const t of types) document.addEventListener(t, blockBackground, opts);
    return () => {
      for (const t of types) document.removeEventListener(t, blockBackground, opts);
    };
  }, [inspectInteractionLockActive, inspectOverlayRef]);

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

  // --- Tap for abilities; long-press for the full card inspector -----------
  const [actionPopoverState, setActionPopoverState] = useState(null);

  useEffect(() => {
    setActionPopoverState((current) => {
      if (!current) return current;
      if (current.decisionIdentity !== decisionIdentity) return null;
      if (state?.decision?.kind !== "priority") return null;
      // Re-resolve by action_ref so the popover reflects the live decision's
      // labels/refs; index equality alone can pair up unrelated actions.
      const liveActions = state?.decision?.actions || [];
      const next = (current.actions || [])
        .map((held) => liveActions.find((live) => sameActionRef(live?.action_ref, held?.action_ref)))
        .filter(Boolean);
      if (next.length === 0) return null;
      const unchanged = next.length === current.actions.length
        && next.every((action, actionIndex) => action === current.actions[actionIndex]);
      return unchanged ? current : { ...current, actions: next };
    });
  }, [decisionIdentity, state?.decision]);

  useEffect(() => {
    if (selectedObjectId != null) setActionPopoverState(null);
  }, [selectedObjectId]);

  const closeActionPopover = useCallback(() => setActionPopoverState(null), []);

  const openObjectActions = useCallback(({ card, actions = null, anchorRect = null }) => {
    if (selectedObjectId != null) return false;
    const resolved = Array.isArray(actions)
      ? actions
      : collectActivatableActionsForCard(card, activatableMap);
    if (resolved.length === 0 || state?.decision?.kind !== "priority") return false;
    const objectId = Number(card?.id);
    if (actionPopoverState?.objectId === objectId) {
      setActionPopoverState(null);
      return false;
    }
    const normalizedAnchor = anchorRect
      ? { left: anchorRect.left, top: anchorRect.top, right: anchorRect.right, bottom: anchorRect.bottom, width: anchorRect.width, height: anchorRect.height }
      : null;
    setActionPopoverState({
      objectId,
      cardName: card?.name || "Actions",
      anchorRect: normalizedAnchor,
      actions: resolved,
      decisionIdentity,
    });
    onInspect?.(null);
    return true;
  }, [actionPopoverState?.objectId, activatableMap, decisionIdentity, onInspect, selectedObjectId, state?.decision?.kind]);

  const inspectHeldObject = useCallback(({ card }) => {
    closeActionPopover();
    requestInspectObject(card?.id ?? null);
  }, [closeActionPopover, requestInspectObject]);

  // A hand-card drop with several possible plays surfaces them as an anchored
  // popover at the drop point (dispatched by Workspace's drop handler).
  useEffect(() => {
    const onMobileCardActions = (event) => {
      const detail = event?.detail || null;
      const actions = Array.isArray(detail?.actions) ? detail.actions : [];
      if (!detail || actions.length <= 1) return;
      if (selectedObjectId != null || state?.decision?.kind !== "priority") return;
      setActionPopoverState({
        objectId: Number(detail.objectId),
        cardName: detail.cardName || "Actions",
        anchorRect: detail.anchorRect || null,
        actions,
        collapseEquivalentActions: false,
        decisionIdentity,
      });
    };
    window.addEventListener("ironsmith:mobile-card-actions", onMobileCardActions);
    return () => window.removeEventListener("ironsmith:mobile-card-actions", onMobileCardActions);
  }, [decisionIdentity, selectedObjectId, state?.decision?.kind]);

  const handlePopoverAction = useCallback((action) => {
    if (!action) return;
    if (action.kind === "untap_land") {
      cancelDecision();
      closeActionPopover();
      return;
    }
    dispatch(
      { type: "priority_action", action_index: action.index, action_ref: action.action_ref },
      action.label
    );
    closeActionPopover();
  }, [cancelDecision, closeActionPopover, dispatch]);

  // --- Card click / target-pointer-down (salvaged) -------------------------
  const handleCardInspect = useCallback((event, card) => {
    if (canPickBattlefieldObjects && !shouldHandleClick(event)) return;
    const candidateIds = collectCardObjectIds(card);
    if (canPickBattlefieldObjects) {
      const matched = candidateIds.find((id) => legalSelectableObjectIds.has(id));
      if (matched != null) {
        if (state?.decision?.kind === "select_objects") {
          requestObjectSelection(matched, "add");
          return;
        }
        window.dispatchEvent(new CustomEvent("ironsmith:target-choice", {
          detail: { target: { kind: "object", object: matched } },
        }));
        return;
      }
    }
    if (selectedObjectId != null) return;
    requestInspectObject(card.id, { candidateObjectIds: candidateIds });
  }, [canPickBattlefieldObjects, legalSelectableObjectIds, requestInspectObject, selectedObjectId, shouldHandleClick, state?.decision?.kind]);

  const handleCardTargetPointerDown = useCallback((event, card) => {
    if (!canPickBattlefieldObjects || !registerPointerDown(event)) return;
    const candidateIds = collectCardObjectIds(card);
    const matched = candidateIds.find((id) => legalSelectableObjectIds.has(id));
    if (matched == null) return;
    event.preventDefault();
    event.stopPropagation();
    if (state?.decision?.kind === "select_objects") {
      requestObjectSelection(matched, "add");
      return;
    }
    window.dispatchEvent(new CustomEvent("ironsmith:target-choice", {
      detail: { target: { kind: "object", object: matched } },
    }));
  }, [canPickBattlefieldObjects, legalSelectableObjectIds, registerPointerDown, state?.decision?.kind]);

  // --- Opponent-band pointer / click capture (salvaged) --------------------
  const opponentTapRef = useRef(null);
  const opponentBandSelector = ".mobile-mtga-battlefield-band--opponent";

  const opponentCardFromPointerEvent = useCallback((event) => {
    const withinExpanded = (rect, x, y) => (
      x >= (rect.left - MOBILE_OPPONENT_CARD_HIT_SLOP_X)
      && x <= (rect.right + MOBILE_OPPONENT_CARD_HIT_SLOP_X)
      && y >= (rect.top - MOBILE_OPPONENT_CARD_HIT_SLOP_Y)
      && y <= (rect.bottom + MOBILE_OPPONENT_CARD_HIT_SLOP_Y)
    );

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

    const pathEl = tryFromPath();
    if (pathEl) {
      const id = pathEl.dataset?.objectId;
      if (id) {
        const card = opponentCardById.get(String(id));
        if (card) return card;
      }
    }

    const targetEl = event.target instanceof Element
      ? event.target.closest(".game-card[data-object-id]")
      : null;
    if (targetEl) {
      const id = targetEl.dataset?.objectId;
      if (id) {
        const card = opponentCardById.get(String(id));
        if (card) return card;
      }
    }

    if (Number.isFinite(event?.clientX) && Number.isFinite(event?.clientY) && typeof document !== "undefined") {
      const offsets = [[0, 0], [-12, 0], [12, 0], [0, -12], [0, 12], [-10, -10], [10, -10], [-10, 10], [10, 10]];
      for (const [ox, oy] of offsets) {
        const hit = document.elementFromPoint(event.clientX + ox, event.clientY + oy);
        const cardEl = hit?.closest?.(".game-card[data-object-id]");
        if (!cardEl) continue;
        const id = cardEl.dataset?.objectId;
        if (!id) continue;
        const card = opponentCardById.get(String(id));
        if (card) return card;
      }

      // Geometry fallback
      const nodes = document.querySelectorAll(`${opponentBandSelector} .game-card[data-object-id]`);
      let best = null;
      let bestDistSq = Infinity;
      for (const node of nodes) {
        const rect = node.getBoundingClientRect();
        if (!withinExpanded(rect, event.clientX, event.clientY)) continue;
        const id = node.dataset?.objectId;
        if (!id) continue;
        const card = opponentCardById.get(String(id));
        if (!card) continue;
        const cx = rect.left + rect.width / 2;
        const cy = rect.top + rect.height / 2;
        const distSq = (event.clientX - cx) ** 2 + (event.clientY - cy) ** 2;
        if (distSq < bestDistSq) {
          best = card;
          bestDistSq = distSq;
        }
      }
      if (best) return best;
    }

    return null;
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
    const pending = opponentTapRef.current;
    opponentTapRef.current = null;
    if (canPickBattlefieldObjects || !pending) return;
    if (pending.pointerId != null && event.pointerId !== pending.pointerId) return;
    const dx = event.clientX - pending.startX;
    const dy = event.clientY - pending.startY;
    if ((dx * dx + dy * dy) > MOBILE_CARD_TAP_MAX_DISTANCE_SQ) return;
    const resolved = (pending.cardId != null
      ? opponentCardById.get(String(pending.cardId)) || null
      : null) || opponentCardFromPointerEvent(event);
    if (!resolved) return;
    const opened = openObjectActions({
      card: resolved,
      anchorRect: event.target instanceof Element
        ? event.target.closest(".game-card[data-object-id]")?.getBoundingClientRect?.() || null
        : null,
    });
    if (opened) {
      event.preventDefault();
      event.stopPropagation();
    }
  }, [canPickBattlefieldObjects, openObjectActions, opponentCardFromPointerEvent, opponentCardById]);

  const handleOpponentBandPointerCancelCapture = useCallback(() => {
    opponentTapRef.current = null;
  }, []);

  const handleOpponentBandPointerLeave = useCallback((event) => {
    if (event.pointerType === "mouse") opponentTapRef.current = null;
  }, []);

  const handleOpponentBandClickCapture = useCallback((event) => {
    const cm = combatModeRef.current;
    if (!activeOpponent || !cm?.onTargetAreaClick || cm.selectedAttacker == null) return;
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
    if (tryFromPath()) return;
    if (event.target instanceof Element && event.target.closest(".game-card[data-object-id]")) return;
    const hit = document.elementFromPoint(event.clientX, event.clientY);
    if (hit?.closest(".game-card[data-object-id]")) return;
    event.preventDefault();
    event.stopPropagation();
    const validTargets = cm.validTargetPlayersByAttacker?.[Number(cm.selectedAttacker)];
    const directId = Number(activeOpponent.id);
    const fallbackId = Number(activeOpponent.index);
    const playerId = validTargets?.has?.(directId) ? directId : fallbackId;
    cm.onTargetAreaClick(playerId, null);
  }, [activeOpponent, combatModeRef]);

  // --- Player-target taps --------------------------------------------------
  const dispatchPlayerChoice = useCallback((player) => {
    if (!canPickTargets || !player) return;
    const target = legalTargetPlayerIds.has(Number(player.id))
      ? Number(player.id)
      : Number(player.index);
    if (!Number.isFinite(target)) return;
    window.dispatchEvent(
      new CustomEvent("ironsmith:target-choice", {
        detail: { target: { kind: "player", player: target } },
      })
    );
  }, [canPickTargets, legalTargetPlayerIds]);

  // --- Cycling -------------------------------------------------------------
  const cycleOpponent = useCallback((direction) => {
    if (typeof setMobileOpponentIndex !== "function" || opponentCount <= 1) return;
    setMobileOpponentIndex((current) => {
      const next = Number(current || 0) + direction;
      if (next < 0) return opponentCount - 1;
      if (next >= opponentCount) return 0;
      return next;
    });
  }, [opponentCount, setMobileOpponentIndex]);

  // --- View toggle ---------------------------------------------------------
  const handleToggleView = useCallback(() => {
    if (typeof setMobileViewMode !== "function") return;
    setMobileViewMode((current) => (current === "hand" ? "battlefield" : "hand"));
  }, [setMobileViewMode]);

  // --- Zone open ----------------------------------------------------------
  const [openZone, setOpenZone] = useState(null);
  const handleOpenZone = useCallback((zoneKey, player = me) => {
    if (zoneKey === "library") {
      setOpenZone(null);
      onOpenDecklist?.(player);
    } else setOpenZone({ zone: zoneKey, playerId: player?.id });
  }, [me, onOpenDecklist]);
  useEffect(() => {
    const openTargetZone = (event) => {
      const { zone, playerId } = event.detail || {};
      if (["graveyard", "exile", "command", "ante"].includes(zone)) setOpenZone({ zone, playerId });
    };
    window.addEventListener("ironsmith:open-target-zone", openTargetZone);
    return () => window.removeEventListener("ironsmith:open-target-zone", openTargetZone);
  }, []);
  const zonePlayer = openZone && state?.players?.find((player) => samePlayerId(player.id, openZone.playerId));

  return (
    <main
      ref={sceneRef}
      className="mobile-battle-scene mobile-mtga-scene table-gradient table-shell relative h-full min-h-0 overflow-hidden"
      data-drop-zone
      data-mobile-battle-scene
      data-stack-visible={stackVisible ? "true" : "false"}
      data-layout-overflow={layout.fitsViewport ? "false" : "true"}
      data-inspector-open={inspectInteractionLockActive ? "true" : "false"}
      style={{
        "--mobile-arena-resource-height": `${layout.landHeight}px`,
        "--mobile-battle-card-width": `${layout.cardWidth}px`,
        "--mobile-battle-card-height": `${layout.cardHeight}px`,
        "--mobile-battle-top-status-height": `${layout.topStatusHeight}px`,
        "--mobile-battle-control-height": `${layout.controlBandHeight}px`,
        "--mobile-battle-opponent-band-height": `${layout.opponentBandHeight}px`,
        "--mobile-battle-self-band-height": `${layout.selfBandHeight}px`,
        "--mobile-battle-self-back-visible-height": `${layout.selfBackVisibleHeight}px`,
        "--mobile-battle-scene-padding": `${layout.sidePadding}px`,
        "--mobile-battle-section-gap": `${layout.sectionGap}px`,
        "--mobile-battle-row-gap": `${layout.rowGap}px`,
        "--mobile-mtga-hand-peek-height": `${layout.handPeekHeight}px`,
        "--mobile-mtga-stack-rail-width": `${layout.stackRailWidth}px`,
        "--mobile-opponent-controls-width": `${opponentWidth}px`,
        "--mobile-self-controls-width": `${selfWidth}px`,
        "--mobile-action-controls-width": `${actionWidth}px`,
        "--mobile-zone-pile-width": `${layout.zonePileWidth}px`,
        "--mobile-inspector-height": `${Math.min(300, layout.viewportHeight - 44)}px`,
      }}
    >

        <div className="mobile-mtga-scene-layout">
          <div ref={opponentHudRef} className="mobile-mtga-scene-row mobile-opponent-controls">
            <MobileOpponentHud
              opponent={activeOpponent}
              cycleEnabled={cycleEnabled}
              previousOpponent={previousOpponent}
              nextOpponent={nextOpponent}
              onCyclePrev={() => cycleOpponent(-1)}
              onCycleNext={() => cycleOpponent(1)}
              onTap={canPickTargets ? dispatchPlayerChoice : (player) => handleOpenZone("graveyard", player)}
              targetable={opponentTargetable && canPickTargets}
            />
          </div>

          <MobileBattlefieldBand
            side="opponent"
            manaIndented={cycleEnabled}
            player={activeOpponent}
            onOpenZone={handleOpenZone}
            rows={opponentRows}
            cardWidth={layout.cardWidth}
            cardHeight={layout.cardHeight}
            landHeight={layout.landHeight}
            selectedObjectId={selectedObjectId}
            onInspect={requestInspectObject}
            onCardClick={handleCardInspect}
            onCardPointerDown={handleCardTargetPointerDown}
            onMobileCardActionMenu={openObjectActions}
            onMobileCardLongPress={inspectHeldObject}
            activatableMap={activatableMap}
            legalTargetObjectIds={legalSelectableObjectIds}
            onPointerDownCapture={handleOpponentBandPointerDownCapture}
            onPointerUpCapture={handleOpponentBandPointerUpCapture}
            onPointerCancelCapture={handleOpponentBandPointerCancelCapture}
            onPointerLeave={handleOpponentBandPointerLeave}
            onClickCapture={handleOpponentBandClickCapture}
          />

          <section
            ref={controlBandRef}
            className="mobile-mtga-control-row"
            data-mobile-hand-drop-target="battlefield"
          >
            <div ref={selfHudRef} className="mobile-self-controls">
            <MobileSelfHud
              me={me}
              onTap={canPickTargets ? dispatchPlayerChoice : () => handleOpenZone("library")}
              onOpenZone={handleOpenZone}
              targetable={selfTargetable && canPickTargets}
            />
            </div>
            <div ref={actionControlsRef} className="mobile-action-controls">
              <PhaseTrack compact />
              <MobileTurnActionStack ref={setActionStackElement} />
            </div>
          </section>

          <MobileBattlefieldBand
            side="self"
            player={me}
            onOpenZone={handleOpenZone}
            rows={selfRows}
            cardWidth={layout.cardWidth}
            cardHeight={layout.cardHeight}
            landHeight={layout.landHeight}
            selectedObjectId={selectedObjectId}
            onInspect={requestInspectObject}
            onCardClick={handleCardInspect}
            onCardPointerDown={handleCardTargetPointerDown}
            onMobileCardActionMenu={openObjectActions}
            onMobileCardLongPress={inspectHeldObject}
            activatableMap={activatableMap}
            legalTargetObjectIds={legalSelectableObjectIds}
          />

          <div className="mobile-mtga-scene-row mobile-mtga-scene-row--hand">
            <MobileHandFan
              me={me}
              selectedObjectId={selectedObjectId}
              onInspect={requestInspectObject}
            />
          </div>
        </div>

        <MobileViewToggle
          mode={mobileViewMode}
          onToggle={handleToggleView}
          className="mobile-mtga-view-toggle--floating"
        />

        {stackVisible ? (
          <MobileStackRail
            objects={visibleStackObjects}
            focusedStackObjectId={focusedStackObjectId}
            onFocusStackObject={onFocusStackObject}
            onInspect={requestInspectObject}
          />
        ) : null}

        <DecisionPopupLayer
          selectedObjectId={selectedObjectId}
          mobileBattle
          mobileBattlePortalTarget={actionStackElement}
          mobileBattleDockInline
          mobileBattleDockHidden={actionPopoverState != null}
          mobileBattleDockOrientation="horizontal"
        />

        {actionPopoverState ? (
          <ActionPopover
            key={String(actionPopoverState.objectId)}
            anchorRect={actionPopoverState.anchorRect || {
              left: window.innerWidth / 2,
              right: window.innerWidth / 2,
              top: window.innerHeight * 0.55,
              bottom: window.innerHeight * 0.55,
              width: 0,
              height: 0,
            }}
            actions={actionPopoverState.actions}
            collapseEquivalentActions={actionPopoverState.collapseEquivalentActions !== false}
            title={ui(actionPopoverState.cardName)}
            variant="game"
            previewCards={false}
            fitViewport
            ariaLabel={ui("Abilities: {0}", { 0: actionPopoverState.cardName })}
            onAction={handlePopoverAction}
            onClose={() => {
              const closingObjectId = actionPopoverState.objectId;
              setActionPopoverState((current) => (
                current?.objectId === closingObjectId ? null : current
              ));
            }}
          />
        ) : null}

        {mobileViewMode === "hand" ? (
          <MobileHandFullscreen
            me={me}
            selectedObjectId={selectedObjectId}
            onInspect={requestInspectObject}
            onClose={() => setMobileViewMode?.("battlefield")}
          />
        ) : null}

        {openZone && zonePlayer ? (
          <MobileZoneBrowser player={zonePlayer} zone={openZone.zone}
            onZoneChange={handleOpenZone}
            legalIds={legalSelectableObjectIds} choosing={canPickBattlefieldObjects}
            onClose={() => setOpenZone(null)}
            onCardClick={(event, card) => {
              handleCardInspect(event, card);
              if (canPickBattlefieldObjects) setOpenZone(null);
            }} />
        ) : null}

        {inspectorOpen ? (
          <div
            ref={inspectOverlayRef}
            className="mobile-battle-inspect-overlay"
            data-card-inspector="true"
            role="dialog"
            tabIndex={-1}
            aria-modal="true"
            aria-label={ui("Card inspector")}
            onPointerDown={(e) => e.stopPropagation()}
            onPointerUp={(e) => e.stopPropagation()}
            onClick={(e) => {
              e.stopPropagation();
              if (e.target === e.currentTarget) closeInspector();
            }}
          >
            <div className="mobile-battle-inspect-overlay-backdrop" aria-hidden="true" />
            <div
              className="mobile-battle-inspect-overlay-shell"
              data-card-inspector="true"
              onClick={(e) => e.stopPropagation()}
              onPointerDown={(e) => e.stopPropagation()}
              onPointerUp={(e) => e.stopPropagation()}
            >
              <div className="mobile-battle-inspect-overlay-stage">
                <HoverArtOverlay
                  objectId={selectedObjectId}
                  displayMode="inspector"
                  availableInspectorWidth={Math.min(820, layout.viewportWidth) - 56}
                  availableInspectorHeight={Math.min(300, layout.viewportHeight - 44)}
                  minInspectorTextScale={0.54}
                  minInspectorTitleScale={0.46}
                  onInspectorAccentChange={null}
                />
              </div>
              <button
                type="button"
                className="mobile-battle-inspect-overlay-close"
                aria-label={ui("Close inspector")}
                onClick={closeInspector}
              >
                <X className="size-4" aria-hidden="true" />
              </button>
            </div>
          </div>
        ) : null}
    </main>
  );
}
