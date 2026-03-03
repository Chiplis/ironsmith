import { createContext, useContext, useState, useCallback } from "react";

const HoverContext = createContext(null);

export function HoverProvider({ children }) {
  const [hoveredObjectId, setHoveredObjectId] = useState(null);

  const hoverCard = useCallback((objectId) => {
    setHoveredObjectId(objectId != null ? String(objectId) : null);
  }, []);

  const clearHover = useCallback(() => {
    setHoveredObjectId(null);
  }, []);

  return (
    <HoverContext.Provider value={{ hoveredObjectId, hoverCard, clearHover }}>
      {children}
    </HoverContext.Provider>
  );
}

export function useHover() {
  const ctx = useContext(HoverContext);
  if (!ctx) throw new Error("useHover must be inside HoverProvider");
  return ctx;
}
