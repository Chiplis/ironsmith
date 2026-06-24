export function resolveInspectorObjectId({
  selectedObjectId = null,
  pinnedObjectId = null,
  hoveredObjectId = null,
} = {}) {
  if (selectedObjectId != null) return String(selectedObjectId);
  if (pinnedObjectId != null) return String(pinnedObjectId);
  if (hoveredObjectId != null) return String(hoveredObjectId);
  return null;
}
