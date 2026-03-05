import { createContext, useContext, useState, useCallback, useMemo, useRef } from "react";

const DragStateContext = createContext(undefined);
const DragActionsContext = createContext(undefined);

export function DragProvider({ children }) {
  const [dragState, setDragState] = useState(null);
  const dragStateRef = useRef(null);
  // dragState shape: {
  //   objectId, cardName, actions, glowKind, startX, startY, currentX, currentY, sourceRect
  // }

  const startDrag = useCallback((objectId, cardName, actions, glowKind, x, y, sourceRect = null) => {
    const next = {
      objectId,
      cardName,
      actions,
      glowKind,
      startX: x,
      startY: y,
      currentX: x,
      currentY: y,
      sourceRect,
    };
    dragStateRef.current = next;
    setDragState(next);
  }, []);

  const updateDrag = useCallback((x, y) => {
    setDragState((prev) => {
      if (!prev) return null;
      const next = { ...prev, currentX: x, currentY: y };
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
    () => ({ startDrag, updateDrag, endDrag }),
    [startDrag, updateDrag, endDrag]
  );

  return (
    <DragStateContext.Provider value={dragState}>
      <DragActionsContext.Provider value={actions}>
        {children}
      </DragActionsContext.Provider>
    </DragStateContext.Provider>
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
  const { startDrag, updateDrag, endDrag } = useDragActions();
  return { dragState, startDrag, updateDrag, endDrag };
}
