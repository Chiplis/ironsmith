// The objects an option stands for: the one it names, plus any it drags along.
export function optionObjectIds(opt) {
  const ids = [];
  if (opt?.object_id != null) ids.push(String(opt.object_id));
  if (Array.isArray(opt?.related_object_ids)) {
    for (const relatedId of opt.related_object_ids) {
      if (relatedId != null) ids.push(String(relatedId));
    }
  }
  return Array.from(new Set(ids));
}

export function optionReferencesObject(opt, objectId) {
  if (objectId == null) return false;
  const normalizedId = String(objectId);
  return optionObjectIds(opt).some((id) => id === normalizedId);
}

// A click can stand for every member of a merged permanent. Submit only when
// all matching options describe one choice; sharing a preview object does not
// make two modes equivalent. Explicitly grouped equivalent actions may still
// choose the matching member, as their option-row shortcut does.
export function optionForClickedObjects(decision, objectIds) {
  if (!decision || decision.kind !== "select_options") return null;
  const clicked = new Set((objectIds || []).filter(id => id != null).map(String));
  const matches = new Map();
  for (const opt of decision.options || []) {
    const grouped = Array.isArray(opt?.grouped_options) ? opt.grouped_options : [];
    const candidates = [opt, ...grouped];
    const actionSignature = (option) => JSON.stringify([
      String(option?.description || "").trim().toLowerCase(),
      Boolean(option?.repeatable),
      option?.max_count ?? (option?.repeatable ? null : 1),
      option?.point_cost ?? 1,
    ]);
    const equivalentGroup = grouped.length > 0
      && String(opt?.description || "").trim().length > 0
      && candidates.every(candidate => actionSignature(candidate) === actionSignature(opt));
    for (const candidate of candidates) {
      if (candidate?.legal === false || candidate?.index == null) continue;
      if (!optionObjectIds(candidate).some(id => clicked.has(id))) continue;
      const key = equivalentGroup ? `group:${opt.index}` : `option:${candidate.index}`;
      if (!matches.has(key)) matches.set(key, candidate);
    }
  }
  return matches.size === 1 ? matches.values().next().value : null;
}

export function optionForClickedObject(decision, objectId) {
  return optionForClickedObjects(decision, [objectId]);
}

import { getPlayerAccent } from "./player-colors.js";
import { getVisibleStackObjects } from "./stack-targets.js";

// An ability is targeted as "<source> ability", the way the engine names it.
function stackTargetName(stackObject) {
  const name = String(stackObject?.name || "").trim();
  if (!name || !stackObject?.ability_kind) return name;
  return `${name} ability`;
}

function registerName(map, id, name) {
  if (id == null) return;
  const key = String(id);
  if (!key) return;
  const text = String(name || "").trim();
  if (!text) return;
  const existing = map.get(key);
  if (existing && isHiddenCardName(text) && !isHiddenCardName(existing)) return;
  map.set(key, text);
}

function isHiddenCardName(name) {
  return String(name || "").trim().toLowerCase() === "hidden card";
}

function decisionViewedCards(state) {
  const viewedCards = state?.viewed_cards || null;
  if (viewedCards?.inspector_only || viewedCards?.inspectorOnly) return null;
  return viewedCards;
}

function registerController(map, id, controller) {
  if (id == null || controller == null) return;
  const key = String(id);
  if (!key) return;
  map.set(key, Number(controller));
}

export function buildObjectNameById(state) {
  const map = new Map();

  for (const player of state?.players || []) {
    for (const card of player?.hand_cards || []) {
      registerName(map, card?.id, card?.name);
    }
    for (const card of player?.graveyard_cards || []) {
      registerName(map, card?.id, card?.name);
    }
    for (const card of player?.exile_cards || []) {
      registerName(map, card?.id, card?.name);
    }
    for (const card of player?.command_cards || []) {
      registerName(map, card?.id, card?.name);
    }
    for (const card of player?.ante_cards || []) {
      registerName(map, card?.id, card?.name);
    }
    for (const card of player?.sideboard_cards || []) {
      registerName(map, card?.id, card?.name);
    }
    for (const card of player?.battlefield || []) {
      registerName(map, card?.id, card?.name);
      for (const memberId of card?.member_ids || []) {
        registerName(map, memberId, card?.name);
      }
    }
  }

  for (const stackObject of getVisibleStackObjects(state)) {
    registerName(map, stackObject?.id, stackObject?.name);
    registerName(map, stackObject?.inspect_object_id, stackObject?.name);
    // The id a decision names the entry by (an ability's own stack id).
    registerName(map, stackObject?.target_object_id, stackTargetName(stackObject));
  }

  const viewedCards = decisionViewedCards(state);
  for (const card of viewedCards?.cards || []) {
    registerName(map, card?.id, card?.name);
  }

  return map;
}

export function buildObjectControllerById(state) {
  const map = new Map();

  for (const player of state?.players || []) {
    for (const zone of [
      player?.hand_cards || [],
      player?.graveyard_cards || [],
      player?.exile_cards || [],
      player?.command_cards || [],
      player?.ante_cards || [],
      player?.sideboard_cards || [],
    ]) {
      for (const card of zone) {
        registerController(map, card?.id, player?.id);
      }
    }

    for (const card of player?.battlefield || []) {
      registerController(map, card?.id, player?.id);
      for (const memberId of card?.member_ids || []) {
        registerController(map, memberId, player?.id);
      }
    }
  }

  for (const stackObject of getVisibleStackObjects(state)) {
    registerController(map, stackObject?.id, stackObject?.controller);
    registerController(map, stackObject?.inspect_object_id, stackObject?.controller);
    registerController(map, stackObject?.target_object_id, stackObject?.controller);
  }

  const viewedCards = decisionViewedCards(state);
  const viewedSubject = viewedCards?.subject;
  for (const card of viewedCards?.cards || []) {
    registerController(map, card?.id, viewedSubject);
  }
  for (const cardId of viewedCards?.card_ids || []) {
    registerController(map, cardId, viewedSubject);
  }

  return map;
}

export function buildInspectableObjectIdSet(state) {
  const ids = new Set();

  for (const player of state?.players || []) {
    for (const zone of [
      player?.battlefield || [],
      player?.hand_cards || [],
      player?.graveyard_cards || [],
      player?.exile_cards || [],
      player?.command_cards || [],
      player?.ante_cards || [],
      player?.sideboard_cards || [],
    ]) {
      for (const card of zone) {
        if (card?.id != null) {
          ids.add(String(card.id));
        }
        for (const memberId of card?.member_ids || []) {
          if (memberId != null) {
            ids.add(String(memberId));
          }
        }
      }
    }
  }

  for (const stackObject of getVisibleStackObjects(state)) {
    if (stackObject?.id != null) {
      ids.add(String(stackObject.id));
    }
    if (stackObject?.inspect_object_id != null) {
      ids.add(String(stackObject.inspect_object_id));
    }
  }

  const viewedCards = decisionViewedCards(state);
  for (const card of viewedCards?.cards || []) {
    if (card?.id != null) {
      ids.add(String(card.id));
    }
  }
  for (const cardId of viewedCards?.card_ids || []) {
    if (cardId != null) {
      ids.add(String(cardId));
    }
  }

  return ids;
}

export function getObjectAccent(state, objectId, explicitControllerId = null) {
  if (objectId == null) return null;
  const controllerById = buildObjectControllerById(state);
  const controllerId = explicitControllerId != null
    ? Number(explicitControllerId)
    : controllerById.get(String(objectId));
  if (controllerId == null || Number(controllerId) === Number(state?.perspective)) {
    return null;
  }
  return getPlayerAccent(state?.players || [], controllerId, state?.perspective);
}

const OBJECT_ZONES = new Set(['hand', 'library', 'graveyard', 'exile', 'command', 'battlefield', 'stack', 'ante', 'sideboard']);
export function normalizeObjectZone(zone) {
  const value = String(zone || '').toLowerCase().replace(/[_ -]/g, '').replace(/zone$/, '');
  return OBJECT_ZONES.has(value) ? value : null;
}

// Read only snapshot-visible metadata; never inspect or fetch a hidden card.
export function buildObjectLocationById(state) {
  const locations = new Map();
  const add = (card, zone, owner, controller) => {
    if (card?.id == null) return;
    const value = {
      zone: normalizeObjectZone(card.zone) || normalizeObjectZone(zone),
      owner: card.owner ?? owner ?? null,
      controller: card.controller ?? controller ?? null,
    };
    for (const id of [card.id, ...(card.member_ids || [])]) locations.set(String(id), value);
  };
  for (const player of state?.players || []) {
    for (const zone of ['hand', 'library', 'graveyard', 'exile', 'command', 'ante', 'sideboard']) {
      for (const card of player[`${zone}_cards`] || []) add(card, zone, player.id, player.id);
    }
    for (const card of player.battlefield || []) add(card, 'battlefield', null, player.id);
    for (const card of player.persistent_look_cards || []) add(card, card.zone, player.id, player.id);
  }
  for (const card of getVisibleStackObjects(state)) {
    // Display ids are synthetic and can collide with real card ids. An
    // ability's inspect id names its source, which stays in its own zone.
    const ids = [card.target_object_id ?? card.id];
    if (!card.ability_kind) ids.push(card.inspect_object_id);
    for (const id of ids) add({ ...card, id }, 'stack', null, card.controller);
  }
  const viewed = decisionViewedCards(state);
  for (const card of viewed?.cards || []) {
    // A viewed card can have moved since the look started. Prefer its current
    // visible location over the original view's subject/zone.
    if (!locations.has(String(card.id))) add(card, viewed.zone, viewed.subject, viewed.subject);
  }
  return locations;
}

export function decisionObjectLocation(locations, objectId, object = {}) {
  if (objectId == null) return null;
  const known = locations.get(String(objectId));
  const zone = normalizeObjectZone(object.zone ?? object.hidden_ref?.zone) || known?.zone;
  if (!zone) return null;
  return {
    zone,
    owner: object.object_owner ?? object.owner ?? object.hidden_ref?.owner ?? known?.owner ?? null,
    controller: object.object_controller ?? object.controller ?? known?.controller ?? null,
  };
}
