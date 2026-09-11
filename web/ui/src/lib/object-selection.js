export const SELECT_OBJECT_CHOICE_EVENT = "ironsmith:select-object-choice";

function sameObjectId(left, right) {
  return String(left) === String(right);
}

export function isObjectChosen(selectedIds, objectId) {
  if (objectId == null) return false;
  for (const id of selectedIds || []) {
    if (sameObjectId(id, objectId)) return true;
  }
  return false;
}

/**
 * Card surfaces only ever add a choice: a search that takes several clicks must
 * not lose one to a stray click on a card already chosen. Removing is reserved
 * for the check badge ("remove"); the decision list keeps toggling.
 */
export function selectionAfterChoice(
  selectedIds,
  { objectId, mode = "toggle", max = Infinity } = {},
) {
  const current = Array.isArray(selectedIds) ? selectedIds : Array.from(selectedIds || []);
  if (objectId == null) return current;
  const index = current.findIndex((id) => sameObjectId(id, objectId));
  if (index >= 0) {
    if (mode === "add") return current;
    return current.filter((_, position) => position !== index);
  }
  if (mode === "remove") return current;
  const limit = Number(max);
  if (Number.isFinite(limit) && current.length >= Math.max(0, limit)) return current;
  return [...current, objectId];
}

/** Board, zone and card surfaces all reach the live decision through this event. */
export function requestObjectSelection(objectId, mode = "toggle") {
  if (objectId == null || typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent(SELECT_OBJECT_CHOICE_EVENT, {
    detail: { objectId, mode },
  }));
}
