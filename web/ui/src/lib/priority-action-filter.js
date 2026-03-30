export function collectSelectedPriorityActionIndices(actions = [], selectedObjectFamilyIds = new Set()) {
  const ids = new Set();
  const familyIds = selectedObjectFamilyIds instanceof Set ? selectedObjectFamilyIds : new Set();
  if (familyIds.size === 0) return ids;

  for (const action of actions) {
    const actionObjectId = action?.object_id != null ? String(action.object_id) : null;
    if (actionObjectId != null && familyIds.has(actionObjectId)) {
      ids.add(action.index);
    }
  }

  return ids;
}

export function filterPriorityActionGroups(
  actionGroups = [],
  selectedObjectFamilyIds = new Set(),
  selectedActionIndices = new Set(),
) {
  const familyIds = selectedObjectFamilyIds instanceof Set ? selectedObjectFamilyIds : new Set();
  const actionIds = selectedActionIndices instanceof Set ? selectedActionIndices : new Set();
  if (familyIds.size === 0 && actionIds.size === 0) return actionGroups;

  return actionGroups.filter((group) => {
    for (const linkedObjectId of group?.linkedObjectIds || []) {
      if (familyIds.has(linkedObjectId)) return true;
    }
    for (const actionIndex of group?.actionIndices || []) {
      if (actionIds.has(actionIndex)) return true;
    }
    return false;
  });
}
