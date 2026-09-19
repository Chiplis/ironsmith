import { useState, useCallback, useMemo, useRef } from "react";
import { CombatArrowContext } from "@/context/CombatArrowContext.shared";
import { combatStateArrowSignature } from "@/lib/combat-arrows";

export function CombatArrowProvider({ children }) {
  // The declaring seat's in-progress attacker/blocker declarations.
  const [combatArrows, setCombatArrows] = useState([]);
  // Accepted attackers and blockers from the engine's combat state, shown to
  // every seat until combat ends (see lib/combat-arrows.js).
  const [combatStateArrows, setCombatStateArrows] = useState([]);
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

  const updateCombatStateArrows = useCallback((newArrows) => {
    // Snapshots arrive on every action; keep the previous array while combat
    // is unchanged so the overlay does not restart its measuring loop.
    setCombatStateArrows((previous) => (
      combatStateArrowSignature(previous) === combatStateArrowSignature(newArrows)
        ? previous
        : newArrows
    ));
  }, []);

  const clearCombatStateArrows = useCallback(() => {
    setCombatStateArrows([]);
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

  const arrows = useMemo(() => {
    // A declaration in progress redraws the same creature; let it win over the
    // accepted-state arrow so a creature never carries two arrows.
    const declaredFrom = new Set(combatArrows.map((arrow) => `${arrow.key.startsWith("blk-") ? "blk" : "atk"}:${arrow.fromId}`));
    const persisted = combatStateArrows.filter((arrow) => (
      !declaredFrom.has(`${arrow.key.startsWith("blk-") ? "blk" : "atk"}:${arrow.fromId}`)
    ));
    return [...combatArrows, ...persisted, ...stackArrows];
  }, [combatArrows, combatStateArrows, stackArrows]);

  return (
    <CombatArrowContext.Provider value={{
      arrows, updateArrows, clearArrows,
      updateCombatStateArrows, clearCombatStateArrows,
      updateStackArrows, clearStackArrows,
      dragArrow, dragArrowRef, startDragArrow, updateDragArrow, endDragArrow,
      combatMode, combatModeRef, setCombatMode,
    }}>
      {children}
    </CombatArrowContext.Provider>
  );
}
