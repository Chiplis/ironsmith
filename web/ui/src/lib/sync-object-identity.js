import { normalizeSelectObjectHiddenRef, findPriorityActionForCommand } from "./sync-commands.js";

export function actionRefObjectId(actionRef) {
  if (!actionRef || typeof actionRef !== "object") return null;
  switch (String(actionRef.kind || "")) {
    case "play_land":
      return actionRef.land_id;
    case "cast_spell":
      return actionRef.spell_id;
    case "open_exiled_card_for_play":
    case "cast_exiled_card_face_down":
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
    case "open_exiled_card_for_play":
    case "cast_exiled_card_face_down":
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


export function isOpaqueExilePlayCommand(command) {
  return command?.type === "priority_action"
    && ["open_exiled_card_for_play", "cast_exiled_card_face_down"].includes(command.action_ref?.kind);
}

// Opaque actions use the shared ciphertext identity when one exists. Private
// hydration may replace the local manifest slot without changing that public
// origin. Reading metadata here never requires an exported face/name.
export function opaqueExileOriginReference(metadata) {
  if (!metadata || metadata.zone !== "exile") return null;
  const publicSlot = metadata.publicSlot ?? metadata.public_slot;
  const publicCommitment = metadata.publicCommitment ?? metadata.public_commitment;
  const slot = publicCommitment && publicSlot != null ? publicSlot : metadata.slot;
  const commitment = publicCommitment && publicSlot != null ? publicCommitment : metadata.commitment;
  if (!Number.isSafeInteger(metadata.owner) || metadata.owner < 0
    || !Number.isSafeInteger(slot) || slot < 0 || typeof commitment !== "string" || !commitment) return null;
  return { owner: metadata.owner, zone: "exile", slot, commitment };
}

function opaqueExileObjectForReference(checkpoint, reference) {
  // Do not use the general selection normalizer: its legacy partial-reference
  // completion may pair a private slot with a public commitment.
  const owner = reference?.owner;
  const pairs = [[reference?.slot, reference?.commitment],
    [reference?.public_slot ?? reference?.publicSlot,
      reference?.public_commitment ?? reference?.publicCommitment]]
    .filter(([slot, commitment]) => slot != null || commitment != null);
  if (!Number.isSafeInteger(owner) || owner < 0 || reference?.zone !== "exile"
    || pairs.length === 0 || pairs.some(([slot, commitment]) =>
      !Number.isSafeInteger(slot) || slot < 0 || typeof commitment !== "string" || !commitment)) {
    throw new Error("Opaque exile remapping requires its complete public hidden identity");
  }
  const matches = (checkpoint?.objects || []).filter(object => {
    const hidden = object?.hiddenCard ?? object?.hidden_card;
    if (object?.zone !== "exile" || hidden?.owner !== owner) return false;
    return pairs.every(([slot, commitment]) =>
      (hidden.slot === slot && hidden.commitment === commitment)
      || ((hidden.publicSlot ?? hidden.public_slot) === slot
        && (hidden.publicCommitment ?? hidden.public_commitment) === commitment));
  });
  return matches.length === 1 ? matches[0] : null;
}

// A hidden commitment identifies a physical card, not its current CR 400.7
// object. The offered action freezes its public zone-change generation. Only
// that pair permits a legitimate peer-local ObjectId translation.
export function resolveOpaqueExilePlayCommand(command, checkpoint) {
  if (!isOpaqueExilePlayCommand(command)) return command;
  const originalId = Number(actionRefObjectId(command.action_ref));
  if (!Number.isSafeInteger(originalId) || originalId <= 0) {
    throw new Error("Opaque exile action has an invalid original object");
  }
  const hiddenRef = command.object_hidden_ref ?? command.objectHiddenRef;
  const matches = (checkpoint?.objects || []).filter(object => Number(object?.id) === originalId);
  const current = hiddenRef ? opaqueExileObjectForReference(checkpoint, hiddenRef)
    : matches.length === 1 ? matches[0] : null;
  const currentId = current == null ? null : Number(current.id);
  if (!current || !Number.isSafeInteger(currentId) || currentId <= 0 || current.zone !== "exile") {
    throw new Error("Opaque exile action has no exact current exile origin");
  }
  const hidden = current.hiddenCard ?? current.hidden_card;
  const frozen = command.action_ref.incarnation;
  if (hidden) {
    if (!hiddenRef) {
      throw new Error("Tracked opaque exile action requires its complete public hidden identity");
    }
    if (!Number.isSafeInteger(frozen) || frozen < 0
      || !Number.isSafeInteger(hidden.incarnation) || hidden.incarnation < 0
      || hidden.incarnation !== frozen) {
      throw new Error("Opaque exile action refers to an obsolete or unknown hidden incarnation");
    }
  } else if (frozen != null || hiddenRef || currentId !== originalId) {
    // Untracked local cards have no cryptographic incarnation witness. Their
    // native ObjectId remains exact; they cannot use an identity fallback.
    throw new Error("Untracked exile action requires its exact original object");
  }
  if (currentId === originalId) return command;
  return { ...command, object_id: currentId,
    action_ref: actionRefWithObjectId(command.action_ref, currentId) };
}


export async function localOpaqueExilePlayCommand(game, command) {
  if (command?.type === "priority_action" && !command.action_ref && command.action_index != null) {
    if (typeof game?.uiState !== "function") {
      throw new Error("Index-only priority command requires a current public decision or an explicit action reference");
    }
    const decision = (await game.uiState())?.decision;
    const action = findPriorityActionForCommand(decision, command);
    if (!action || isOpaqueExilePlayCommand({ type: "priority_action", action_ref: action.action_ref })) {
      throw new Error("Opaque or unavailable index-only priority command requires its explicit frozen action reference");
    }
  }
  if (!isOpaqueExilePlayCommand(command)) return command;
  if (typeof game?.getHiddenCardState !== "function") {
    throw new Error("Engine cannot validate the opaque exile incarnation");
  }
  return resolveOpaqueExilePlayCommand(command, await game.getHiddenCardState());
}
