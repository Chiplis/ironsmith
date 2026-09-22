import { getPlayerAccent } from "./player-colors.js";

const PILE_ZONES = ["graveyard", "exile"];

// Ability snapshot ids can be shared by triggers from the same event or by
// copies. React keys must identify occurrences, not just those snapshot ids.
// Count from the bottom so pushing/popping the top preserves the keys below it.
export function stackEntryRenderKeys(entries) {
  const occurrences = new Map();
  const keys = new Array(entries.length);
  for (let index = entries.length - 1; index >= 0; index -= 1) {
    const id = String(entries[index].id);
    const occurrence = occurrences.get(id) || 0;
    keys[index] = `${id}:${occurrence}`;
    occurrences.set(id, occurrence + 1);
  }
  return keys;
}

export const STACK_TARGET_ZONE_ORDER = [
  "battlefield",
  "hand",
  "graveyard",
  "library",
  "exile",
  "command",
  "ante",
  "sideboard",
];

export function normalizeZoneViews(zoneViews) {
  const normalized = Array.isArray(zoneViews)
    ? zoneViews.filter((zone) => STACK_TARGET_ZONE_ORDER.includes(zone))
    : [];
  return Array.from(new Set(["battlefield", ...normalized]));
}

export function getVisibleStackObjects(state) {
  const realStackObjects = Array.isArray(state?.stack_objects) ? state.stack_objects : [];
  const resolvingStackObject = state?.resolving_stack_object || null;
  // This presentation-only entry keeps a popped spell visible while its
  // resolution asks for a choice. Priority means that resolution has ended;
  // a leftover resolving snapshot must not resurrect it in the stack UI.
  if (!resolvingStackObject || state?.decision?.kind === "priority") return realStackObjects;

  const resolvingId = Number(resolvingStackObject?.id);
  if (
    Number.isFinite(resolvingId)
    && realStackObjects.some((entry) => Number(entry?.id) === resolvingId)
  ) {
    return realStackObjects;
  }

  return [resolvingStackObject, ...realStackObjects];
}

export function getVisibleTopStackObject(state) {
  return getVisibleStackObjects(state)[0] || null;
}

export function stackInspectObjectId(entry) {
  return entry?.inspect_object_id ?? entry?.id ?? null;
}

/**
 * The engine ids a targeting decision can name for a stack entry.
 *
 * A stack tile is drawn under its own presentation id, which is not the id a
 * spell like Counterspell targets: a spell on the stack is targeted by the
 * object it inspects to, and an ability on the stack is not a targetable
 * object at all (its inspect id is the permanent that produced it, and a
 * Bolt aimed at that permanent must not light the ability up).
 */
export function stackEntryTargetObjectIds(entry) {
  if (!entry || entry.ability_kind) return [];
  const objectId = Number(entry.inspect_object_id);
  return Number.isFinite(objectId) ? [objectId] : [];
}

/**
 * The objects a stack entry is aimed at: what lights up (and what pile opens)
 * while the entry is hovered. Players are not objects and are left out.
 */
export function stackEntryAimedObjectIds(entry) {
  return (entry?.targets || [])
    .filter((target) => target?.kind === "object" && target.object != null)
    .map((target) => Number(target.object))
    .filter((objectId) => Number.isFinite(objectId));
}

export function stackEntryIsLegalTarget(decision, entry) {
  if (decision?.kind !== "targets") return false;
  const ids = stackEntryTargetObjectIds(entry);
  if (ids.length === 0) return false;
  return (decision.requirements || []).some((requirement) =>
    (requirement?.legal_targets || []).some((target) =>
      target?.kind === "object" && ids.includes(Number(target.object))
    )
  );
}

export function stackSelectionKeys(entry) {
  const keys = [entry?.id, entry?.inspect_object_id]
    .filter((value) => value != null)
    .map((value) => String(value));
  return Array.from(new Set(keys));
}

function indexObject(map, objectId, renderedId, zone, playerId) {
  const numericObjectId = Number(objectId);
  if (!Number.isFinite(numericObjectId)) return;
  map.set(String(numericObjectId), {
    renderedId: Number(renderedId),
    zone,
    playerId: playerId == null ? null : Number(playerId),
  });
}

function indexPlayerZone(map, cards, zone, playerId) {
  for (const card of cards || []) {
    const renderedId = Number(card?.id);
    if (!Number.isFinite(renderedId)) continue;
    indexObject(map, renderedId, renderedId, zone, playerId);

    if (Array.isArray(card?.member_ids)) {
      for (const memberId of card.member_ids) {
        indexObject(map, memberId, renderedId, zone, playerId);
      }
    }
  }
}

export function buildRenderableObjectIndex(state) {
  const index = new Map();

  for (const player of state?.players || []) {
    const playerId = Number(player?.id);
    indexPlayerZone(index, player?.battlefield || [], "battlefield", playerId);
    indexPlayerZone(index, player?.hand_cards || [], "hand", playerId);
    indexPlayerZone(index, player?.graveyard_cards || [], "graveyard", playerId);
    indexPlayerZone(index, player?.exile_cards || [], "exile", playerId);
    indexPlayerZone(index, player?.command_cards || [], "command", playerId);
    indexPlayerZone(index, player?.ante_cards || [], "ante", playerId);
    indexPlayerZone(index, player?.sideboard_cards || [], "sideboard", playerId);
  }

  for (const stackEntry of getVisibleStackObjects(state)) {
    const stackObjectId = Number(stackEntry?.id);
    if (!Number.isFinite(stackObjectId)) continue;
    indexObject(index, stackObjectId, stackObjectId, "stack", null);
    // A spell that targets another spell names the engine object, not the
    // tile it is drawn as; the arrow still has to land on that tile.
    for (const targetObjectId of stackEntryTargetObjectIds(stackEntry)) {
      if (!index.has(String(targetObjectId))) {
        indexObject(index, targetObjectId, stackObjectId, "stack", null);
      }
    }
  }

  return index;
}

function resolveActiveStackObject(stackObjects = [], selectedObjectId = null) {
  const selectedKey = selectedObjectId == null ? null : String(selectedObjectId);
  if (selectedKey != null) {
    const selectedEntry = stackObjects.find((entry) => (
      String(stackInspectObjectId(entry)) === selectedKey
      || String(entry?.id) === selectedKey
    ));
    if (selectedEntry) return selectedEntry;
  }

  return stackObjects[0] || null;
}

// A decision option linked to an object needs that object on screen. A
// battlefield permanent already is; one sitting in a pile has no element to
// highlight and nothing for a card frame to sit beside until the pile opens.
// Returns the zones to open on top of the ones the player opened themselves.
export function hoveredObjectZoneViews(state, hoveredObjectId, zoneViews = []) {
  if (hoveredObjectId == null) return [];
  const resolved = buildRenderableObjectIndex(state).get(String(hoveredObjectId));
  // The library is never browsable and the stack has its own presentation.
  if (!resolved || resolved.zone === "stack" || resolved.zone === "library") return [];
  // Graveyard and exile are piles, not inline zone bodies (shouldShowZoneBody
  // refuses them), so a zone view cannot reveal them -- and changing the view
  // remounts the pile, throwing away the open state it is opening itself with.
  // ZonePile watches the hovered object directly instead.
  if (PILE_ZONES.includes(resolved.zone)) return [];
  const activeZones = new Set(normalizeZoneViews(zoneViews));
  return activeZones.has(resolved.zone) ? [] : [resolved.zone];
}

export function buildStackTargetPresentation(state, zoneViews = [], selectedObjectId = null) {
  const stackObjects = getVisibleStackObjects(state);
  const activeStackObject = resolveActiveStackObject(stackObjects, selectedObjectId);
  if (!activeStackObject) {
    return {
      activeStackObject: null,
      arrows: [],
      temporaryZoneViews: [],
    };
  }

  const accent = getPlayerAccent(state?.players || [], activeStackObject.controller, state?.perspective);
  const arrowColor = accent?.hex || "#ff3b30";
  const activeZones = new Set(normalizeZoneViews(zoneViews));
  const renderableObjectIndex = buildRenderableObjectIndex(state);
  const temporaryZones = new Set();
  const arrows = [];

  for (const [targetIndex, target] of (activeStackObject.targets || []).entries()) {
    if (target?.kind === "player" && target.player != null) {
      arrows.push({
        fromId: activeStackObject.id,
        toPlayerId: Number(target.player),
        color: arrowColor,
        key: `stack-target-${activeStackObject.id}-player-${target.player}-${targetIndex}`,
      });
      continue;
    }

    if (target?.kind !== "object" || target.object == null) continue;
    const resolvedTarget = renderableObjectIndex.get(String(target.object));
    if (!resolvedTarget || !Number.isFinite(resolvedTarget.renderedId)) continue;

    if (
      resolvedTarget.zone !== "stack"
      && resolvedTarget.zone !== "library"
      && !activeZones.has(resolvedTarget.zone)
    ) {
      temporaryZones.add(resolvedTarget.zone);
    }

    arrows.push({
      fromId: activeStackObject.id,
      toId: resolvedTarget.renderedId,
      color: arrowColor,
      key: `stack-target-${activeStackObject.id}-object-${target.object}-${targetIndex}`,
    });
  }

  return {
    activeStackObject,
    arrows,
    temporaryZoneViews: STACK_TARGET_ZONE_ORDER.filter((zone) => temporaryZones.has(zone)),
  };
}
