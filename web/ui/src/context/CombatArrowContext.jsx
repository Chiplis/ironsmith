import { useState, useCallback, useMemo, useRef } from "react";
import { CombatArrowContext } from "@/context/CombatArrowContext.shared";

export function CombatArrowProvider({ children }) {
  const [combatArrows, setCombatArrows] = useState([]);
  const [stackArrows, setStackArrows] = useState([]);
  // arrows shape: [{ fromId, toId, toPlayerId, color, key }]

  // Live drag arrow: { fromId, x, y, color }
  const [dragArrow, setDragArrow] = useState(null);
  const dragArrowRef = useRef(null);

  // Combat interaction mode — set by AttackersDecision / BlockersDecision
  // Shape: { mode: "attackers"|"blockers", candidates: Set<id>, onDrop(fromId, targetEl) }
  const combatModeRef = useRef(null);
  const [combatMode, _setCombatMode] = useState(null);

  const setCombatMode = useCallback((mode) => {
    combatModeRef.current = mode;
    _setCombatMode(mode);
  }, []);

  const updateArrows = useCallback((newArrows) => {
    setCombatArrows(newArrows);
  }, []);

  const clearArrows = useCallback(() => {
    setCombatArrows([]);
  }, []);

  const updateStackArrows = useCallback((newArrows) => {
    setStackArrows(newArrows);
  }, []);

  const clearStackArrows = useCallback(() => {
    setStackArrows([]);
  }, []);

  const startDragArrow = useCallback((fromId, x, y, color) => {
    dragArrowRef.current = { fromId, x, y, color };
    setDragArrow(dragArrowRef.current);
  }, []);

  const updateDragArrow = useCallback((x, y) => {
    if (dragArrowRef.current) dragArrowRef.current = { ...dragArrowRef.current, x, y };
  }, []);

  const endDragArrow = useCallback(() => {
    dragArrowRef.current = null;
    setDragArrow(null);
  }, []);

  const arrows = useMemo(
    () => [...combatArrows, ...stackArrows],
    [combatArrows, stackArrows]
  );

  return (
    <CombatArrowContext.Provider value={{
      arrows, updateArrows, clearArrows, updateStackArrows, clearStackArrows,
      dragArrow, dragArrowRef, startDragArrow, updateDragArrow, endDragArrow,
      combatMode, combatModeRef, setCombatMode,
    }}>
      {children}
    </CombatArrowContext.Provider>
  );
}
