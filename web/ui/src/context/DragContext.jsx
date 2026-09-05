/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext, useState, useCallback, useMemo, useRef } from "react";
import { castHoverTargetAtPoint } from "@/lib/hand-drag-intent";

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
  const [dragState, setDragState] = useState(null);
  const [placementSlots, setPlacementSlots] = useState(() => new Map());
  const [pendingPlacement, setPendingPlacement] = useState(null);
  const dragStateRef = useRef(null);
  // dragState shape: {
  //   objectId, cardName, card, actions, glowKind, startX, startY, currentX, currentY,
  //   sourceRect, sourceContainerRect, hiddenSourcePoint, castIntent
  // }

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
  ) => {
    const next = {
      objectId,
      cardName,
      card,
      actions,
      glowKind,
      startX: x,
      startY: y,
      currentX: x,
      currentY: y,
      sourceRect,
      sourceContainerRect,
      hiddenSourcePoint,
      castIntent: null,
    };
    dragStateRef.current = next;
    setDragState(next);
  }, []);

  const updateDrag = useCallback((x, y) => {
    const hit = dragStateRef.current?.castIntent ? castHoverTargetAtPoint(x, y) : null;
    setDragState((prev) => {
      if (!prev) return null;
      const hoverCandidate = JSON.stringify(prev.hoverCandidate ?? null) === JSON.stringify(hit)
        ? prev.hoverCandidate
        : hit;
      const next = { ...prev, currentX: x, currentY: y, hoverCandidate };
      dragStateRef.current = next;
      return next;
    });
  }, []);

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
  }, []);

  const setCastTargetPreview = useCallback((objectId, startedAt, targetDecision) => {
    setDragState((prev) => {
      if (!prev?.castIntent || prev.objectId !== objectId || prev.castIntent.startedAt !== startedAt) return prev;
      const next = { ...prev, castIntent: { ...prev.castIntent, targetDecision } };
      dragStateRef.current = next;
      return next;
    });
  }, []);

  const endDrag = useCallback(() => {
    const state = dragStateRef.current;
    dragStateRef.current = null;
    setDragState(null);
    return state;
  }, []);

  const actions = useMemo(
    () => ({ startDrag, updateDrag, markCastIntent, setCastTargetPreview, endDrag }),
    [startDrag, updateDrag, markCastIntent, setCastTargetPreview, endDrag]
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

  return (
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
  );
}

export function useDragState() {
  const dragState = useContext(DragStateContext);
  if (dragState === undefined) throw new Error("useDragState must be inside DragProvider");
  return dragState;
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
