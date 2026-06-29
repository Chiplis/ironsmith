function normalizeZoneLabel(zone) {
  if (!zone) return "Unknown";
  switch (String(zone).toLowerCase()) {
    case "library": return "Library";
    case "hand": return "Hand";
    case "battlefield": return "Battlefield";
    case "graveyard": return "GY";
    case "exile": return "Exile";
    case "stack": return "Stack";
    case "command": return "CZ";
    default:
      return String(zone)
        .split(/[_\s]+/)
        .filter(Boolean)
        .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
        .join(" ");
  }
}

export function zoneLabelFromAction(zone) {
  return normalizeZoneLabel(zone);
}

export function formatPriorityActionLabel(action) {
  const label = action?.label || "";
  if (action?.kind === "play_land") {
    return `Play ${zoneLabelFromAction(action.from_zone)}`;
  }
  if (action?.kind === "cast_spell") {
    return `From ${zoneLabelFromAction(action.from_zone)}`;
  }
  if (action?.kind === "untap_land") {
    return "Untap";
  }
  if (action?.kind === "activate_ability" || action?.kind === "activate_mana_ability") {
    const match = label.match(/^Activate\s+.+?:\s*(.+)$/i);
    if (match) return match[1];
  }
  return label;
}

export function formatPriorityInlineActionLabel(action) {
  const label = String(action?.label || "").trim();
  if (!label) return "Action";

  if (action?.kind === "cast_spell" && action?.object_id != null) {
    return formatCastActionGroupLabel(label);
  }

  if (action?.kind === "activate_ability" || action?.kind === "activate_mana_ability") {
    const activateMatch = label.match(/^Activate\s+.+?:\s*(.+)$/i);
    if (activateMatch) return activateMatch[1];
    const tapMatch = label.match(/^Tap\s+.+?:\s*(.+)$/i);
    if (tapMatch) return tapMatch[1];
  }

  return label;
}

export function formatCastActionGroupLabel(label) {
  const raw = String(label || "").trim();
  const match = raw.match(/^Cast\s+(.+)$/i);
  if (!match) return raw || "Cast";

  const castName = match[1].replace(/\s+\([^()]*\)\s*$/u, "").trim();
  return castName ? `Cast ${castName}` : raw;
}

export function buildBattlefieldFamilies(players) {
  const familyIdByObjectId = new Map();
  const familyMembersByFamilyId = new Map();

  for (const player of players || []) {
    for (const card of player?.battlefield || []) {
      const rootId = card?.id != null ? String(card.id) : null;
      if (!rootId) continue;

      const memberIds = Array.isArray(card?.member_ids)
        ? card.member_ids.map((memberId) => String(memberId))
        : [];
      const familyMembers = Array.from(new Set([rootId, ...memberIds]));

      for (const id of familyMembers) {
        familyIdByObjectId.set(id, rootId);
      }
      familyMembersByFamilyId.set(rootId, familyMembers);
    }
  }

  return { familyIdByObjectId, familyMembersByFamilyId };
}

function castingMethodPreference(action) {
  if (action?.kind !== "cast_spell") return 0;

  const method = action?.action_ref?.casting_method;
  const kind = String(method?.kind || "");
  if (kind === "normal") return 0;
  if (kind === "play_from" && method?.use_alternative == null) return 0;
  if (kind === "split_other_half") return 1;
  if (kind === "fuse") return 2;
  return 10;
}

function priorityActionGroupKey(action, familyId, label) {
  if (action?.kind === "cast_spell" && action?.object_id != null) {
    return `${action.kind || ""}|${action.from_zone || ""}|${familyId}|cast`;
  }
  return `${action?.kind || ""}|${action?.from_zone || ""}|${familyId}|${label}`;
}

export function buildPriorityActionGroups(actions, families = buildBattlefieldFamilies([])) {
  const { familyIdByObjectId = new Map(), familyMembersByFamilyId = new Map() } = families || {};
  const groups = [];
  const byKey = new Map();

  for (const action of actions || []) {
    const label = formatPriorityInlineActionLabel(action);
    const objectId = action?.object_id != null ? String(action.object_id) : null;
    const familyId = objectId != null ? (familyIdByObjectId.get(objectId) || objectId) : "";
    const key = priorityActionGroupKey(action, familyId, label);

    let group = byKey.get(key);
    if (!group) {
      group = {
        key,
        label,
        count: 0,
        firstAction: action,
        actions: [],
        actionIndices: new Set(),
        hoverObjectId: objectId != null ? (familyIdByObjectId.get(objectId) || objectId) : null,
        linkedObjectIds: new Set(),
      };
      byKey.set(key, group);
      groups.push(group);
    }

    group.actions.push(action);
    if (castingMethodPreference(action) < castingMethodPreference(group.firstAction)) {
      group.firstAction = action;
      group.label = formatPriorityInlineActionLabel(action);
    }

    group.count = action?.kind === "cast_spell" && objectId != null
      ? 1
      : group.count + 1;
    group.actionIndices.add(action.index);

    if (objectId != null) {
      const actionFamilyId = familyIdByObjectId.get(objectId);
      if (actionFamilyId && familyMembersByFamilyId.has(actionFamilyId)) {
        for (const id of familyMembersByFamilyId.get(actionFamilyId)) {
          group.linkedObjectIds.add(id);
        }
      } else {
        group.linkedObjectIds.add(objectId);
      }
    }
  }

  return groups;
}
