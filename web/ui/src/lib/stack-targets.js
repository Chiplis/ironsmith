import { getPlayerAccent } from "@/lib/player-colors";

export const STACK_TARGET_ZONE_ORDER = [
  "battlefield",
  "hand",
  "graveyard",
  "library",
  "exile",
  "command",
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
  if (!resolvingStackObject) return realStackObjects;

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
  }

  for (const stackEntry of getVisibleStackObjects(state)) {
    const stackObjectId = Number(stackEntry?.id);
    if (!Number.isFinite(stackObjectId)) continue;
    indexObject(index, stackObjectId, stackObjectId, "stack", null);
  }

  return index;
}

function resolveActiveStackObject(stackObjects = [], selectedObjectId = null) {
  const selectedKey = selectedObjectId == null ? null : String(selectedObjectId);
  if (selectedKey != null) {
    const selectedEntry = stackObjects.find((entry) => (
      String(entry?.inspect_object_id ?? entry?.id) === selectedKey
      || String(entry?.id) === selectedKey
    ));
    if (selectedEntry) return selectedEntry;
  }

  return stackObjects[0] || null;
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

  const accent = getPlayerAccent(state?.players || [], activeStackObject.controller);
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
