import { getVisibleStackObjects } from "./stack-targets.js";

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

export function objectExistsInState(state, objectId) {
  if (!state || objectId == null) return false;
  const needle = String(objectId);
  const players = state?.players || [];

  for (const player of players) {
    const zones = [
      player?.battlefield || [],
      player?.hand_cards || [],
      player?.graveyard_cards || [],
      player?.exile_cards || [],
      player?.command_cards || [],
      player?.ante_cards || [],
    ];
    for (const cards of zones) {
      for (const card of cards) {
        if (String(card?.id) === needle) return true;
        if (Array.isArray(card?.member_ids) && card.member_ids.some((id) => String(id) === needle)) {
          return true;
        }
      }
    }
  }

  for (const entry of getVisibleStackObjects(state)) {
    if (String(entry?.id) === needle) return true;
    if (String(entry?.inspect_object_id) === needle) return true;
  }

  for (const card of state?.planechase?.face_up || []) {
    if (String(card?.id) === needle) return true;
  }

  if ((state?.viewed_cards?.card_ids || []).some((id) => String(id) === needle)) {
    return true;
  }

  return false;
}

export function canHoverInspectorObject(state, objectId) {
  if (!objectExistsInState(state, objectId)) return false;
  return !getVisibleStackObjects(state).some((entry) =>
    [entry.id, entry.inspect_object_id].some((id) => id != null && String(id) === String(objectId))
  );
}
