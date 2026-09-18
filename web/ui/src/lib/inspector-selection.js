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

// The card view for one object id, or null. Callers that only need existence
// should use objectExistsInState, which also counts merged members and cards
// that are visible without being in a zone list.
export function findObjectCardInState(state, objectId) {
  if (!state || objectId == null) return null;
  const needle = String(objectId);

  for (const player of state?.players || []) {
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
        if (String(card?.id) === needle) return card;
      }
    }
  }

  for (const entry of getVisibleStackObjects(state)) {
    if (String(entry?.id) === needle || String(entry?.inspect_object_id) === needle) return entry;
  }

  for (const card of state?.planechase?.face_up || []) {
    if (String(card?.id) === needle) return card;
  }

  return null;
}

export function objectExistsInState(state, objectId) {
  if (!state || objectId == null) return false;
  if (findObjectCardInState(state, objectId)) return true;
  const needle = String(objectId);

  for (const player of state?.players || []) {
    for (const card of player?.battlefield || []) {
      if (Array.isArray(card?.member_ids) && card.member_ids.some((id) => String(id) === needle)) {
        return true;
      }
    }
  }

  if (
    (state?.viewed_cards?.card_ids || []).some((id) => String(id) === needle)
    || (state?.viewed_cards?.cards || []).some((card) => String(card?.id) === needle)
    || (state?.players || []).some((player) => (
      (player?.persistent_look_cards || []).some((card) => String(card?.id) === needle)
    ))
  ) {
    return true;
  }

  return false;
}

// An object id is only valid for the zone it was in. A stack entry outlives
// that: a dies trigger's source is already in the graveyard under a *new*
// object id by the time the trigger is on the stack, so inspect_object_id
// misses. The stable id survives the move, which is what finds it again.
export function findObjectIdByStableId(state, stableId) {
  if (!state || stableId == null) return null;
  const needle = String(stableId);

  for (const player of state?.players || []) {
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
        if (card?.stable_id != null && String(card.stable_id) === needle) return String(card.id);
        // A merged permanent carries its members' stable ids, and the member
        // is the card a player means when they point at it.
        if ((card?.member_stable_ids || []).some((id) => String(id) === needle)) {
          return String(card.id);
        }
      }
    }
  }

  for (const card of state?.planechase?.face_up || []) {
    if (card?.stable_id != null && String(card.stable_id) === needle) return String(card.id);
  }

  return null;
}

// The object id to inspect for a stack entry, preferring one that still exists.
//
// The stable id is tried first on purpose. objectExistsInState would answer
// "yes" for a stale inspect_object_id purely because this very stack entry
// still names it, so asking it first would never reach the fallback. If the
// stable id finds a card, that card *is* the source in whatever zone it now
// occupies; only when it finds nothing is the entry's own id the best answer.
export function resolveStackInspectObjectId(state, entry) {
  const viaStableId = findObjectIdByStableId(state, entry?.source_stable_id ?? entry?.stable_id);
  if (viaStableId != null) return viaStableId;
  const direct = entry?.inspect_object_id ?? entry?.id ?? null;
  return direct == null ? null : String(direct);
}

export function canHoverInspectorObject(state, objectId) {
  if (!objectExistsInState(state, objectId)) return false;
  return !getVisibleStackObjects(state).some((entry) =>
    [entry.id, entry.inspect_object_id].some((id) => id != null && String(id) === String(objectId))
  );
}
