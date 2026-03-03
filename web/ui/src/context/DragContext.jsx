import { createContext, useContext, useState, useCallback } from "react";

const DragContext = createContext(null);

export function DragProvider({ children }) {
  const [dragState, setDragState] = useState(null);
  // dragState shape: { objectId, cardName, actions, glowKind, startX, startY, currentX, currentY }

  const startDrag = useCallback((objectId, cardName, actions, glowKind, x, y) => {
    setDragState({ objectId, cardName, actions, glowKind, startX: x, startY: y, currentX: x, currentY: y });
  }, []);

  const updateDrag = useCallback((x, y) => {
    setDragState((prev) => prev ? { ...prev, currentX: x, currentY: y } : null);
  }, []);

  const endDrag = useCallback(() => {
    const state = dragState;
    setDragState(null);
    return state;
  }, [dragState]);

  return (
    <DragContext.Provider value={{ dragState, startDrag, updateDrag, endDrag }}>
      {children}
    </DragContext.Provider>
  );
}

export function useDrag() {
  const ctx = useContext(DragContext);
  if (!ctx) throw new Error("useDrag must be inside DragProvider");
  return ctx;
}
