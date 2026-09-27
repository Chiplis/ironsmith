import { normalizeSelectObjectHiddenRef } from "./sync-commands.js";

export function actionRefObjectId(actionRef) {
  if (!actionRef || typeof actionRef !== "object") return null;
  switch (String(actionRef.kind || "")) {
    case "play_land":
      return actionRef.land_id;
    case "cast_spell":
      return actionRef.spell_id;
    case "use_pregame_action":
      return actionRef.card_id;
    case "activate_ability":
    case "activate_mana_ability":
      return actionRef.source;
    case "turn_face_up":
      return actionRef.creature_id;
    case "special_action": {
      const action = actionRef.action || {};
      return action.card_id ?? action.permanent_id ?? action.room_id;
    }
    default:
      return null;
  }
}

export function actionRefWithObjectId(actionRef, objectId) {
  if (!actionRef || typeof actionRef !== "object") return actionRef;
  const next = JSON.parse(JSON.stringify(actionRef));
  switch (String(next.kind || "")) {
    case "play_land":
      next.land_id = Number(objectId);
      break;
    case "cast_spell":
      next.spell_id = Number(objectId);
      break;
    case "use_pregame_action":
      next.card_id = Number(objectId);
      break;
    case "activate_ability":
    case "activate_mana_ability":
      next.source = Number(objectId);
      break;
    case "turn_face_up":
      next.creature_id = Number(objectId);
      break;
    case "special_action":
      if (next.action?.card_id != null) {
        next.action.card_id = Number(objectId);
      } else if (next.action?.permanent_id != null) {
        next.action.permanent_id = Number(objectId);
      } else if (next.action?.room_id != null) {
        next.action.room_id = Number(objectId);
      }
      break;
  }
  return next;
}

export function hiddenObjectIdForHiddenRefFromCheckpoint(checkpoint, hiddenRef) {
  const ref = normalizeSelectObjectHiddenRef(hiddenRef);
  if (!ref) return null;
  const matches = [];
  for (const object of checkpoint?.objects || []) {
    const hidden = object?.hiddenCard || object?.hidden_card || null;
    const owner = hidden?.owner ?? object?.owner;
    if (ref.owner != null && Number(owner) !== Number(ref.owner)) continue;
    if (ref.zone && String(object?.zone || "") !== String(ref.zone)) continue;
    const hiddenSlot = hidden?.slot == null ? null : Number(hidden.slot);
    const hiddenCommitment = String(hidden?.commitment || "");
    const publicSlot = hidden?.publicSlot ?? hidden?.public_slot ?? null;
    const publicCommitment = String(hidden?.publicCommitment || hidden?.public_commitment || "");
    if (ref.slot != null && hiddenSlot !== Number(ref.slot)) continue;
    if (ref.public_slot != null && Number(publicSlot) !== Number(ref.public_slot)) continue;
    if (
      ref.commitment
      && hiddenCommitment !== String(ref.commitment)
      && publicCommitment !== String(ref.commitment)
    ) {
      continue;
    }
    if (
      ref.public_commitment
      && hiddenCommitment !== String(ref.public_commitment)
      && publicCommitment !== String(ref.public_commitment)
    ) {
      continue;
    }
    const objectId = Number(object?.id);
    if (Number.isSafeInteger(objectId) && objectId > 0) matches.push(objectId);
  }
  return matches.length === 1 ? matches[0] : null;
}
