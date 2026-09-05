/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext, useState, useCallback, useMemo } from "react";

const HoverStateContext = createContext(undefined);
const HoverLinkedObjectsContext = createContext(undefined);
const AnchoredCardPreviewContext = createContext(undefined);
const HoverActionsContext = createContext(undefined);

function normalizeAnchorRect(anchor) {
  const rect = typeof anchor?.getBoundingClientRect === "function"
    ? anchor.getBoundingClientRect()
    : anchor;
  if (!rect) return null;
  const left = Number(rect.left);
  const top = Number(rect.top);
  const right = Number(rect.right);
  const bottom = Number(rect.bottom);
  if (![left, top, right, bottom].every(Number.isFinite)) return null;
  return {
    left,
    top,
    right,
    bottom,
    width: Number.isFinite(Number(rect.width)) ? Number(rect.width) : Math.max(0, right - left),
    height: Number.isFinite(Number(rect.height)) ? Number(rect.height) : Math.max(0, bottom - top),
  };
}

export function HoverProvider({ children }) {
  const [hoveredObjectId, setHoveredObjectId] = useState(null);
  const [hoveredLinkedObjectIds, setHoveredLinkedObjectIds] = useState(() => new Set());
  const [previewLinkedObjectIds, setPreviewLinkedObjectIds] = useState(() => new Set());
  const [anchoredCardPreview, setAnchoredCardPreview] = useState(null);

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

  const showAnchoredCardPreview = useCallback((objectId, anchor) => {
    const anchorRect = normalizeAnchorRect(anchor);
    if (objectId == null || !anchorRect) return;
    setAnchoredCardPreview({
      objectId: String(objectId),
      anchorRect,
    });
  }, []);

  const clearAnchoredCardPreview = useCallback(() => {
    setAnchoredCardPreview(null);
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
      showAnchoredCardPreview,
      clearAnchoredCardPreview,
    }),
    [
      hoverCard,
      clearHover,
      setHoverLinkedObjects,
      clearHoverLinkedObjects,
      setPreviewLinkedObjects,
      clearPreviewLinkedObjects,
      showAnchoredCardPreview,
      clearAnchoredCardPreview,
    ]
  );

  const linkedObjectIds = useMemo(
    () => new Set([...hoveredLinkedObjectIds, ...previewLinkedObjectIds]),
    [hoveredLinkedObjectIds, previewLinkedObjectIds]
  );

  return (
    <HoverStateContext.Provider value={hoveredObjectId}>
      <HoverLinkedObjectsContext.Provider value={linkedObjectIds}>
        <AnchoredCardPreviewContext.Provider value={anchoredCardPreview}>
          <HoverActionsContext.Provider value={actions}>
            {children}
          </HoverActionsContext.Provider>
        </AnchoredCardPreviewContext.Provider>
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

export function useAnchoredCardPreview() {
  const preview = useContext(AnchoredCardPreviewContext);
  if (preview === undefined) {
    throw new Error("useAnchoredCardPreview must be inside HoverProvider");
  }
  return preview;
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
    showAnchoredCardPreview,
    clearAnchoredCardPreview,
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
    showAnchoredCardPreview,
    clearAnchoredCardPreview,
  };
}
