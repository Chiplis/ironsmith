import { createContext, useCallback, useContext, useMemo, useState } from "react";

const MobileBattleContext = createContext(null);

export function MobileBattleProvider({ children, viewMode, setViewMode, phaseStops, setPhaseStops }) {
  const [handFanned, setHandFanned] = useState(false);

  const togglePhaseStop = useCallback((key) => {
    if (typeof setPhaseStops !== "function") return;
    setPhaseStops((current) => {
      const next = new Set(current || []);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }, [setPhaseStops]);

  const clearPhaseStops = useCallback(() => {
    if (typeof setPhaseStops !== "function") return;
    setPhaseStops(new Set());
  }, [setPhaseStops]);

  const value = useMemo(() => ({
    viewMode: viewMode || "battlefield",
    setViewMode,
    phaseStops: phaseStops || new Set(),
    setPhaseStops,
    togglePhaseStop,
    clearPhaseStops,
    handFanned,
    setHandFanned,
  }), [viewMode, setViewMode, phaseStops, setPhaseStops, togglePhaseStop, clearPhaseStops, handFanned]);

  return (
    <MobileBattleContext.Provider value={value}>
      {children}
    </MobileBattleContext.Provider>
  );
}

export function useMobileBattle() {
  const ctx = useContext(MobileBattleContext);
  if (!ctx) {
    return {
      viewMode: "battlefield",
      setViewMode: () => {},
      phaseStops: new Set(),
      setPhaseStops: () => {},
      togglePhaseStop: () => {},
      clearPhaseStops: () => {},
      handFanned: false,
      setHandFanned: () => {},
    };
  }
  return ctx;
}

export default MobileBattleContext;
