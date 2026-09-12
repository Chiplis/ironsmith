/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext, useEffect, useState, useCallback, useMemo, useRef, useSyncExternalStore } from "react";
import { castHoverTargetAtPoint } from "@/lib/hand-drag-intent";

import { createFrameStore } from "@/lib/frame-store";

const DragMotionContext = createContext(null);
const CastHoverContext = createContext(null);
const CastTargetContext = createContext(null);
const DragStateContext = createContext(undefined);
const DragActionsContext = createContext(undefined);
const PlacementSlotsContext = createContext(undefined);
const PlacementActionsContext = createContext(undefined);
const PendingPlacementContext = createContext(undefined);

function placementKeys(card) {
  const keys = [];
  for (const stableId of card?.member_stable_ids || []) {
    if (stableId != null) keys.push(`stable:${stableId}`);
  }
  if (card?.stable_id != null) keys.push(`stable:${card.stable_id}`);
  if (card?.id != null) keys.push(`object:${card.id}`);
  if (keys.length === 0 && card?.name) keys.push(`name:${card.name}`);
  return Array.from(new Set(keys));
}

export function DragProvider({ children }) {
  const [dragState, setSession] = useState(null);
  const [motion] = useState(() => createFrameStore());
  const setDragState = useCallback((update) => {
    const next = typeof update === "function" ? update(motion.getSnapshot()) : update;
    motion.set(next);
    setSession(next);
  }, [motion]);
  useEffect(() => () => motion.dispose(), [motion]);
  const [placementSlots, setPlacementSlots] = useState(() => new Map());
  const [pendingPlacement, setPendingPlacement] = useState(null);
  const dragStateRef = useRef(null);
  // dragState shape: {
  //   objectId, cardName, card, actions, glowKind, startX, startY, currentX, currentY,
  //   sourceRect, sourceContainerRect, hiddenSourcePoint, castIntent, keyboard,
  //   held
  // }
  // `keyboard` marks a card held by the activation key rather than a pointer:
  // no button is down, so the mouse is tracked for as long as it is held, and
  // the hold starts aimed at dead space instead of wherever the mouse rests.

  const startDrag = useCallback((
    objectId,
    cardName,
    actions,
    glowKind,
    x,
    y,
    sourceRect = null,
    card = null,
    sourceContainerRect = null,
    hiddenSourcePoint = null,
    { keyboard = false, aim = null } = {},
  ) => {
    const next = {
      objectId,
      cardName,
      card,
      actions,
      glowKind,
      startX: x,
      startY: y,
      // A key press aims at nothing yet, so the hold can start pointing
      // somewhere inert rather than at the resting pointer.
      currentX: aim?.x ?? x,
      currentY: aim?.y ?? y,
      sourceRect,
      sourceContainerRect,
      hiddenSourcePoint,
      keyboard,
      castIntent: null,
    };
    dragStateRef.current = next;
    setDragState(next);
  }, [setDragState]);

  const updateDrag = useCallback((x, y) => {
    const prev = dragStateRef.current;
    if (!prev || (prev.currentX === x && prev.currentY === y)) return;
    const hit = prev.castIntent ? castHoverTargetAtPoint(x, y) : null;
    const hoverChanged = JSON.stringify(prev.hoverCandidate ?? null) !== JSON.stringify(hit);
    const next = { ...prev, currentX: x, currentY: y,
      hoverCandidate: hoverChanged ? hit : prev.hoverCandidate };
    dragStateRef.current = next;
    motion.set(next);
    if (hoverChanged) setSession(next);
  }, [motion]);

  const markCastIntent = useCallback((sourcePoint) => {
    setDragState((prev) => {
      if (!prev || prev.castIntent) return prev;
      const next = {
        ...prev,
        hoverCandidate: castHoverTargetAtPoint(prev.currentX, prev.currentY),
        castIntent: {
          sourcePoint,
          startedAt: Date.now(),
        },
      };
      dragStateRef.current = next;
      return next;
    });
  }, [setDragState]);

  const setCastTargetPreview = useCallback((objectId, startedAt, targetDecision) => {
    setDragState((prev) => {
      if (!prev?.castIntent || prev.objectId !== objectId || prev.castIntent.startedAt !== startedAt) return prev;
      const next = { ...prev, castIntent: { ...prev.castIntent, targetDecision } };
      dragStateRef.current = next;
      return next;
    });
  }, [setDragState]);

  /**
   * Put a released gesture back in the player's hands without the button.
   *
   * A card with more than one way to be cast has not entered the engine yet,
   * so letting go over dead space leaves nothing to keep aiming at. Rather
   * than throw the cast away, the gesture itself carries on: the aim follows
   * the bare pointer until a click picks a target or drops it.
   */
  const resumeDrag = useCallback((snapshot) => {
    if (!snapshot) return null;
    const next = { ...snapshot, keyboard: true, held: true };
    dragStateRef.current = next;
    setDragState(next);
    return next;
  }, [setDragState]);

  const endDrag = useCallback(() => {
    const state = dragStateRef.current;
    dragStateRef.current = null;
    setDragState(null);
    return state;
  }, [setDragState]);

  const actions = useMemo(
    () => ({ startDrag, updateDrag, markCastIntent, setCastTargetPreview, resumeDrag, endDrag }),
    [startDrag, updateDrag, markCastIntent, setCastTargetPreview, resumeDrag, endDrag]
  );

  const commitPlacementSlot = useCallback((card, slot) => {
    const row = Math.max(1, Math.floor(Number(slot?.row) || 0));
    const column = Math.max(1, Math.floor(Number(slot?.column) || 0));
    const keys = placementKeys(card);
    if (keys.length === 0 || row <= 0 || column <= 0) return;
    setPlacementSlots((current) => {
      const next = new Map(current);
      const value = { row, column, committedAt: Date.now() };
      for (const key of keys) next.set(key, value);
      return next;
    });
  }, []);

  const stagePlacement = useCallback((card, actions, slot) => {
    const row = Math.max(1, Math.floor(Number(slot?.row) || 0));
    const column = Math.max(1, Math.floor(Number(slot?.column) || 0));
    if (!card || row <= 0 || column <= 0) return;
    setPendingPlacement({
      card,
      actions: Array.isArray(actions) ? actions : [],
      slot: { ...slot, row, column },
    });
  }, []);

  const clearPendingPlacement = useCallback(() => {
    setPendingPlacement(null);
  }, []);

  const placementActions = useMemo(
    () => ({ commitPlacementSlot, stagePlacement, clearPendingPlacement }),
    [clearPendingPlacement, commitPlacementSlot, stagePlacement]
  );

  // A card held by the activation key has no button holding it down, so the
  // hold follows the bare pointer for as long as it lasts and a click releases
  // it through the same handler a drag uses. Escape puts the card back: with no
  // button to lift, nothing else would end the hold.
  useEffect(() => {
    if (!dragState?.keyboard) return undefined;
    const onPointerMove = (event) => updateDrag(event.clientX, event.clientY);
    const onKeyDown = (event) => {
      if (event.key !== "Escape" || event.defaultPrevented) return;
      event.preventDefault();
      endDrag();
      clearPendingPlacement();
    };
    document.addEventListener("pointermove", onPointerMove, { passive: true });
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointermove", onPointerMove);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [clearPendingPlacement, dragState?.keyboard, endDrag, updateDrag]);

  return (
    <DragMotionContext.Provider value={motion}>
    <CastHoverContext.Provider value={dragState?.castIntent ? dragState.hoverCandidate : null}>
      <CastTargetContext.Provider value={dragState?.castIntent || null}>
        <DragStateContext.Provider value={dragState}>
          <DragActionsContext.Provider value={actions}>
            <PlacementSlotsContext.Provider value={placementSlots}>
              <PlacementActionsContext.Provider value={placementActions}>
                <PendingPlacementContext.Provider value={pendingPlacement}>
                  {children}
                </PendingPlacementContext.Provider>
              </PlacementActionsContext.Provider>
            </PlacementSlotsContext.Provider>
          </DragActionsContext.Provider>
        </DragStateContext.Provider>
      </CastTargetContext.Provider>
    </CastHoverContext.Provider>
    </DragMotionContext.Provider>
  );
}

export function useDragSession() {
  const dragState = useContext(DragStateContext);
  if (dragState === undefined) throw new Error("useDragState must be inside DragProvider");
  return dragState;
}

export function useDragState() {
  const motion = useContext(DragMotionContext);
  if (!motion) throw new Error("useDragState must be inside DragProvider");
  return useSyncExternalStore(motion.subscribe, motion.getSnapshot, motion.getSnapshot);
}

export function useDragActions() {
  const ctx = useContext(DragActionsContext);
  if (!ctx) throw new Error("useDragActions must be inside DragProvider");
  return ctx;
}

export function useDrag() {
  const dragState = useDragState();
  const { startDrag, updateDrag, markCastIntent, endDrag } = useDragActions();
  return { dragState, startDrag, updateDrag, markCastIntent, endDrag };
}

export function usePlacementSlots() {
  const slots = useContext(PlacementSlotsContext);
  if (slots === undefined) throw new Error("usePlacementSlots must be inside DragProvider");
  return slots;
}

export function usePlacementActions() {
  const actions = useContext(PlacementActionsContext);
  if (!actions) throw new Error("usePlacementActions must be inside DragProvider");
  return actions;
}

export function usePendingPlacement() {
  const pending = useContext(PendingPlacementContext);
  if (pending === undefined) throw new Error("usePendingPlacement must be inside DragProvider");
  return pending;
}

export function placementSlotForCard(placementSlots, card) {
  for (const key of placementKeys(card)) {
    const slot = placementSlots?.get?.(key);
    if (slot) return slot;
  }
  return null;
}

export function useCastTargeting() {
  return useContext(CastTargetContext);
}

export function useCastTargetHover() {
  return useContext(CastHoverContext);
}

export function useCastPlayerHovered(playerId) {
  const candidate = useCastTargetHover();
  return candidate?.kind === "player" && candidate.playerIds.some(id => Number(id) === Number(playerId));
}
