import { createContext, useContext, useState, useCallback, useRef } from "react";

const CombatArrowContext = createContext(null);

export function CombatArrowProvider({ children }) {
  const [arrows, setArrows] = useState([]);
  // arrows shape: [{ fromId, toId, toPlayerId, color, key }]

  // Live drag arrow: { fromId, x, y, color }
  const [dragArrow, setDragArrow] = useState(null);

  // Combat interaction mode — set by AttackersDecision / BlockersDecision
  // Shape: { mode: "attackers"|"blockers", candidates: Set<id>, onDrop(fromId, targetEl) }
  const combatModeRef = useRef(null);
  const [combatMode, _setCombatMode] = useState(null);

  const setCombatMode = useCallback((mode) => {
    combatModeRef.current = mode;
    _setCombatMode(mode);
  }, []);

  const updateArrows = useCallback((newArrows) => {
    setArrows(newArrows);
  }, []);

  const clearArrows = useCallback(() => {
    setArrows([]);
  }, []);

  const startDragArrow = useCallback((fromId, x, y, color) => {
    setDragArrow({ fromId, x, y, color });
  }, []);

  const updateDragArrow = useCallback((x, y) => {
    setDragArrow((prev) => prev ? { ...prev, x, y } : null);
  }, []);

  const endDragArrow = useCallback(() => {
    setDragArrow(null);
  }, []);

  return (
    <CombatArrowContext.Provider value={{
      arrows, updateArrows, clearArrows,
      dragArrow, startDragArrow, updateDragArrow, endDragArrow,
      combatMode, combatModeRef, setCombatMode,
    }}>
      {children}
    </CombatArrowContext.Provider>
  );
}

export function useCombatArrows() {
  const ctx = useContext(CombatArrowContext);
  if (!ctx) throw new Error("useCombatArrows must be inside CombatArrowProvider");
  return ctx;
}
