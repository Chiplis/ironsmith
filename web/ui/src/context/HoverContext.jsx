/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext, useState, useCallback, useMemo } from "react";

const HoverStateContext = createContext(undefined);
const HoverLinkedObjectsContext = createContext(undefined);
const HoverActionsContext = createContext(undefined);

export function HoverProvider({ children }) {
  const [hoveredObjectId, setHoveredObjectId] = useState(null);
  const [hoveredLinkedObjectIds, setHoveredLinkedObjectIds] = useState(() => new Set());
  const [previewLinkedObjectIds, setPreviewLinkedObjectIds] = useState(() => new Set());

  const hoverCard = useCallback((objectId) => {
    setHoveredObjectId(objectId != null ? String(objectId) : null);
  }, []);

  const setHoverLinkedObjects = useCallback((objectIds) => {
    if (!objectIds) {
      setHoveredLinkedObjectIds(new Set());
      return;
    }
    const ids = Array.isArray(objectIds) ? objectIds : Array.from(objectIds);
    const normalized = new Set(
      ids
        .filter((id) => id != null)
        .map((id) => String(id))
    );
    setHoveredLinkedObjectIds(normalized);
  }, []);

  const clearHoverLinkedObjects = useCallback(() => {
    setHoveredLinkedObjectIds(new Set());
  }, []);

  const setPreviewLinkedObjects = useCallback((objectIds) => {
    const ids = Array.isArray(objectIds) ? objectIds : Array.from(objectIds || []);
    setPreviewLinkedObjectIds(new Set(ids.filter((id) => id != null).map(String)));
  }, []);

  const clearPreviewLinkedObjects = useCallback(() => {
    setPreviewLinkedObjectIds(new Set());
  }, []);

  const clearHover = useCallback(() => {
    setHoveredObjectId(null);
    setHoveredLinkedObjectIds(new Set());
  }, []);

  const actions = useMemo(
    () => ({
      hoverCard,
      clearHover,
      setHoverLinkedObjects,
      clearHoverLinkedObjects,
      setPreviewLinkedObjects,
      clearPreviewLinkedObjects,
    }),
    [
      hoverCard,
      clearHover,
      setHoverLinkedObjects,
      clearHoverLinkedObjects,
      setPreviewLinkedObjects,
      clearPreviewLinkedObjects,
    ]
  );

  const linkedObjectIds = useMemo(
    () => new Set([...hoveredLinkedObjectIds, ...previewLinkedObjectIds]),
    [hoveredLinkedObjectIds, previewLinkedObjectIds]
  );

  return (
    <HoverStateContext.Provider value={hoveredObjectId}>
      <HoverLinkedObjectsContext.Provider value={linkedObjectIds}>
        <HoverActionsContext.Provider value={actions}>
          {children}
        </HoverActionsContext.Provider>
      </HoverLinkedObjectsContext.Provider>
    </HoverStateContext.Provider>
  );
}

export function useHoveredObjectId() {
  const hoveredObjectId = useContext(HoverStateContext);
  if (hoveredObjectId === undefined) {
    throw new Error("useHoveredObjectId must be inside HoverProvider");
  }
  return hoveredObjectId;
}

export function useHoverActions() {
  const ctx = useContext(HoverActionsContext);
  if (!ctx) throw new Error("useHoverActions must be inside HoverProvider");
  return ctx;
}

export function useHover() {
  const hoveredObjectId = useHoveredObjectId();
  const hoveredLinkedObjectIds = useContext(HoverLinkedObjectsContext);
  if (hoveredLinkedObjectIds === undefined) {
    throw new Error("useHover must be inside HoverProvider");
  }
  const {
    hoverCard,
    clearHover,
    setHoverLinkedObjects,
    clearHoverLinkedObjects,
    setPreviewLinkedObjects,
    clearPreviewLinkedObjects,
  } = useHoverActions();
  return {
    hoveredObjectId,
    hoveredLinkedObjectIds,
    hoverCard,
    clearHover,
    setHoverLinkedObjects,
    clearHoverLinkedObjects,
    setPreviewLinkedObjects,
    clearPreviewLinkedObjects,
  };
}
